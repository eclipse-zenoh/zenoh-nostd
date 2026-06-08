use alloc::collections::BTreeMap;
use alloc::{boxed::Box, sync::Arc};
use core::cell::RefCell;

use zenoh_proto::BrokerError;
use zenoh_proto::msgs::NetworkMessage;
use zenoh_proto::{Endpoint, fields::ZenohIdProto};

use crate::api::connectivity::ConnectivityState;
use crate::io::transport::{TransportLink, ZTransportLinkTx};
use crate::{config::ZBrokerConfig, io::driver::Driver, platform::ZLinkManager};

type Link<Config> = <<Config as ZBrokerConfig>::LinkManager as ZLinkManager>::Link<'static>;

struct StaticDriver<Config>
where
    Config: ZBrokerConfig + 'static,
{
    driver: RefCell<Driver<'static, Link<Config>, Config::Buff>>,
}

pub struct BrokerState<Config>
where
    Config: ZBrokerConfig + 'static,
{
    north: Option<(ZenohIdProto, Arc<StaticDriver<Config>>)>,
    south: BTreeMap<ZenohIdProto, Arc<StaticDriver<Config>>>,
}

pub struct Broker<Config>
where
    Config: ZBrokerConfig + 'static,
{
    config: &'static Config,
    state: BrokerState<Config>,
}

impl<Config> Broker<Config>
where
    Config: ZBrokerConfig,
{
    pub fn new(config: &'static Config) -> Self {
        Self {
            config,
            state: BrokerState {
                north: None,
                south: BTreeMap::new(),
            },
        }
    }

    fn state(&mut self) -> &mut BrokerState<Config> {
        &mut self.state
    }

    fn update(
        north: bool,
        id: ZenohIdProto,
        state: &mut BrokerState<Config>,
        msg: NetworkMessage<'_>,
        bytes: &[u8],
    ) -> core::result::Result<(), BrokerError> {
        if north {
            for south in state.south.values() {
                south
                    .driver
                    .borrow_mut()
                    .tx_mut()
                    .send_optimized_ref(core::iter::once((msg.as_ref(), bytes)))?;
            }
        } else {
            if let Some((_, north)) = &state.north {
                north
                    .driver
                    .borrow_mut()
                    .tx_mut()
                    .send_optimized_ref(core::iter::once((msg.as_ref(), bytes)))?;
            }

            for (sid, south) in state.south.iter().filter(|(sid, _)| **sid != id) {
                south
                    .driver
                    .borrow_mut()
                    .tx_mut()
                    .send_optimized_ref(core::iter::once((msg.as_ref(), bytes)))?;
            }
        }

        Ok(())
    }

    fn update_north(
        id: ZenohIdProto,
        state: &mut BrokerState<Config>,
        msg: NetworkMessage<'_>,
        bytes: &[u8],
    ) -> core::result::Result<(), BrokerError> {
        Self::update(true, id, state, msg, bytes)
    }

    fn update_south(
        id: ZenohIdProto,
        state: &mut BrokerState<Config>,
        msg: NetworkMessage<'_>,
        bytes: &[u8],
    ) -> core::result::Result<(), BrokerError> {
        Self::update(false, id, state, msg, bytes)
    }

    pub fn open(&mut self, endpoint: Endpoint<'_>) -> core::result::Result<(), BrokerError> {
        loop {
            let transport = Box::leak(Box::new(
                self.config
                    .transports()
                    .connect(endpoint.clone(), self.config.buff())?,
            ));

            let driver = Arc::new(StaticDriver {
                driver: RefCell::new(Driver::new(transport)),
            });

            self.state().north = Some((driver.driver.borrow().zid(), driver.clone()));

            let connectivity = ConnectivityState::new();
            loop {
                let result = driver
                    .driver
                    .borrow_mut()
                    .poll(&mut self.state, &connectivity, Self::update_north)
                    .map_err(|e| e.flatten_map::<BrokerError>());
                match result {
                    Err(e) => {
                        zenoh_proto::error!("Error on north: {}", e);
                        break;
                    }
                    Ok(false) => break,
                    Ok(true) => {}
                }
            }

            self.state().north.take();
        }
    }

    pub fn accept(&mut self, endpoint: Endpoint<'_>) -> core::result::Result<(), BrokerError> {
        loop {
            let transport = Box::leak(Box::new(
                self.config
                    .transports()
                    .listen(endpoint.clone(), self.config.buff())?,
            ));

            let driver = Arc::new(StaticDriver {
                driver: RefCell::new(Driver::new(transport)),
            });

            self.state()
                .south
                .insert(driver.driver.borrow().zid(), driver.clone());

            let connectivity = ConnectivityState::new();
            loop {
                let result = driver
                    .driver
                    .borrow_mut()
                    .poll(&mut self.state, &connectivity, Self::update_south)
                    .map_err(|e| e.flatten_map::<BrokerError>());
                match result {
                    Err(e) => {
                        zenoh_proto::error!("Error on south: {}", e);
                        break;
                    }
                    Ok(false) => break,
                    Ok(true) => {}
                }
            }

            self.state().south.remove(&driver.driver.borrow().zid());
        }
    }
}

#[macro_export]
macro_rules! __broker {
    ($CONFIG:ty: $config:expr) => {{
        static CONFIG: static_cell::StaticCell<$CONFIG> = static_cell::StaticCell::new();
        let config = CONFIG.init($config);

        static BROKER: static_cell::StaticCell<$crate::broker::Broker<$CONFIG>> =
            static_cell::StaticCell::new();

        BROKER.init($crate::broker::Broker::new(config)) as &mut $crate::broker::Broker<$CONFIG>
    }};
}