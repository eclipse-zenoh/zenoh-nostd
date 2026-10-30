#![no_std]

#[cfg(feature = "alloc")]
pub trait AppConfig: ZSessionConfig + ZBrokerConfig {}
#[cfg(feature = "alloc")]
impl<T: ZSessionConfig + ZBrokerConfig> AppConfig for T {}
#[cfg(not(feature = "alloc"))]
pub trait AppConfig: ZSessionConfig {}
#[cfg(not(feature = "alloc"))]
impl<T: ZSessionConfig> AppConfig for T {}

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "alloc")]
use zenoh_nostd::broker::ZBrokerConfig;
use zenoh_nostd::{platform::ZLinkManager, session::*};

/// Resolve the application endpoint without selecting a transport or a board.
pub const fn endpoint(default: &'static str) -> &'static str {
    match option_env!("ENDPOINT") {
        Some(value) => value,
        None => default,
    }
}

/// Addresses on local interfaces; the gateway is the application's ENDPOINT.
pub const BROKER_LISTEN_1: &str = match option_env!("BROKER_LISTEN_1") {
    Some(value) => value,
    None => "tcp/0.0.0.0:7444",
};
pub const BROKER_LISTEN_2: &str = match option_env!("BROKER_LISTEN_2") {
    Some(value) => value,
    None => "tcp/0.0.0.0:7445",
};

pub const PAYLOAD: usize = match usize::from_str_radix(
    match option_env!("PAYLOAD") {
        Some(v) => v,
        None => "8",
    },
    10,
) {
    Ok(v) => v,
    Err(_) => 8,
};

pub const LISTEN: bool = matches!(
    usize::from_str_radix(
        match option_env!("LISTEN") {
            Some(v) => v,
            None => "0",
        },
        10,
    ),
    Ok(1)
);

pub struct ExampleConfig<M: ZLinkManager, const BUFF_SIZE: usize> {
    transports: TransportLinkManager<M>,
}

#[cfg(feature = "alloc")]
impl<M: ZLinkManager, const BUFF_SIZE: usize> ZBrokerConfig for ExampleConfig<M, BUFF_SIZE> {
    type LinkManager = M;
    type Buff = alloc::vec::Vec<u8>;

    fn buff(&self) -> Self::Buff {
        alloc::vec![0; BUFF_SIZE]
    }

    fn transports(&self) -> &TransportLinkManager<Self::LinkManager> {
        &self.transports
    }
}

impl<M: ZLinkManager, const BUFF_SIZE: usize> ZSessionConfig for ExampleConfig<M, BUFF_SIZE> {
    type LinkManager = M;

    #[cfg(not(feature = "alloc"))]
    type Buff = [u8; BUFF_SIZE];

    #[cfg(feature = "alloc")]
    type Buff = alloc::vec::Vec<u8>;

    #[cfg(not(feature = "alloc"))]
    type SubCallbacks<'res> = FixedCapacitySubCallbacks<
        'res,
        8,
        zenoh::storage::RawOrBox<56>,
        zenoh::storage::RawOrBox<600>,
    >;

    #[cfg(feature = "alloc")]
    type SubCallbacks<'res> = AllocSubCallbacks<'res, zenoh::storage::Box, zenoh::storage::Box>;

    #[cfg(not(feature = "alloc"))]
    type GetCallbacks<'res> = FixedCapacityGetCallbacks<
        'res,
        8,
        zenoh::storage::RawOrBox<16>,
        zenoh::storage::RawOrBox<32>,
    >;

    #[cfg(feature = "alloc")]
    type GetCallbacks<'res> = AllocGetCallbacks<'res, zenoh::storage::Box, zenoh::storage::Box>;

    #[cfg(not(feature = "alloc"))]
    type QueryableCallbacks<'res>
        = FixedCapacityQueryableCallbacks<
        'res,
        Self,
        8,
        zenoh::storage::RawOrBox<32>,
        zenoh::storage::RawOrBox<952>,
    >
    where
        Self: 'res;

    #[cfg(feature = "alloc")]
    type QueryableCallbacks<'res>
        = AllocQueryableCallbacks<'res, Self, zenoh::storage::Box, zenoh::storage::Box>
    where
        Self: 'res;

    fn buff(&self) -> Self::Buff {
        #[cfg(not(feature = "alloc"))]
        {
            [0u8; BUFF_SIZE]
        }
        #[cfg(feature = "alloc")]
        {
            alloc::vec![0; BUFF_SIZE]
        }
    }

    fn transports(&self) -> &TransportLinkManager<Self::LinkManager> {
        &self.transports
    }
}

impl<M: ZLinkManager, const BUFF_SIZE: usize> ExampleConfig<M, BUFF_SIZE> {
    pub fn new(manager: M) -> Self {
        Self {
            transports: TransportLinkManager::from(manager),
        }
    }
}
