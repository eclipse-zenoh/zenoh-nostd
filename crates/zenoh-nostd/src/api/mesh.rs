use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::cell::RefCell;

use zenoh_proto::{BrokerError, fields::ZenohIdProto, msgs::NetworkMessage};

use crate::{
    config::ZSessionConfig, io::driver::Driver, io::transport::ZTransportLinkTx,
    platform::ZLinkManager,
};

type Link<Config> = <<Config as ZSessionConfig>::LinkManager as ZLinkManager>::Link<'static>;

/// A peer entry holding an active driver.
pub struct MeshEntry<Config>
where
    Config: ZSessionConfig + 'static,
{
    pub zid: ZenohIdProto,
    pub driver: RefCell<Driver<'static, Link<Config>, Config::Buff>>,
}

/// Shared state for a peer mesh: maps ZID → peer entry.
pub struct MeshState<Config>
where
    Config: ZSessionConfig + 'static,
{
    pub peers: BTreeMap<ZenohIdProto, Arc<MeshEntry<Config>>>,
}

impl<Config> Default for MeshState<Config>
where
    Config: ZSessionConfig,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Config> MeshState<Config>
where
    Config: ZSessionConfig,
{
    pub fn new() -> Self {
        Self {
            peers: BTreeMap::new(),
        }
    }

    /// Insert a peer and return its Arc'd entry.
    pub fn insert(
        &mut self,
        zid: ZenohIdProto,
        driver: Driver<'static, Link<Config>, Config::Buff>,
    ) -> Arc<MeshEntry<Config>> {
        let entry = Arc::new(MeshEntry {
            zid,
            driver: RefCell::new(driver),
        });
        self.peers.insert(zid, entry.clone());
        entry
    }

    pub fn remove(&mut self, zid: &ZenohIdProto) {
        self.peers.remove(zid);
    }

    /// Forward a message from `sender_zid` to all other connected peers.
    pub fn forward(
        sender_zid: ZenohIdProto,
        state: &mut Self,
        msg: NetworkMessage<'_>,
        bytes: &[u8],
    ) -> core::result::Result<(), BrokerError> {
        for (zid, entry) in state.peers.iter() {
            if *zid != sender_zid {
                entry
                    .driver
                    .borrow_mut()
                    .tx_mut()
                    .send_optimized_ref(core::iter::once((msg.as_ref(), bytes)))?;
            }
        }
        Ok(())
    }
}
