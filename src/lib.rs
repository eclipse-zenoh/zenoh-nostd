#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use zenoh_nostd::broker::ZBrokerConfig;

use zenoh_nostd::session::*;

#[cfg(feature = "std")]
pub use zenoh_std::StdLinkManager as LinkManager;

#[cfg(feature = "wasm")]
pub use zenoh_wasm::WasmLinkManager as LinkManager;

pub const ENDPOINT: &str = match option_env!("ENDPOINT") {
    Some(v) => v,
    None => {
        if cfg!(feature = "wasm") {
            "ws/127.0.0.1:7446"
        } else {
            "tcp/127.0.0.1:7447"
        }
    }
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

#[cfg(feature = "std")]
const BUFF_SIZE: u16 = u16::MAX;
#[cfg(feature = "wasm")]
const BUFF_SIZE: u16 = u16::MAX / 2;

pub struct ExampleConfig {
    transports: TransportLinkManager<LinkManager>,
}

impl ExampleConfig {
    pub fn with_transports(mut self, transports: TransportLinkManager<LinkManager>) -> Self {
        self.transports = transports;
        self
    }
}

#[cfg(feature = "alloc")]
impl ZBrokerConfig for ExampleConfig {
    type LinkManager = LinkManager;
    type Buff = alloc::vec::Vec<u8>;

    fn buff(&self) -> Self::Buff {
        alloc::vec![0; BUFF_SIZE as usize]
    }

    fn transports(&self) -> &TransportLinkManager<Self::LinkManager> {
        &self.transports
    }
}

impl ZSessionConfig for ExampleConfig {
    type LinkManager = LinkManager;

    #[cfg(not(feature = "alloc"))]
    type Buff = [u8; BUFF_SIZE as usize];

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
    type SubCallbacks<'res> = AllocSubCallbacks<'res>;

    #[cfg(not(feature = "alloc"))]
    type GetCallbacks<'res> = FixedCapacityGetCallbacks<
        'res,
        8,
        zenoh::storage::RawOrBox<1>,
        zenoh::storage::RawOrBox<32>,
    >;

    #[cfg(feature = "alloc")]
    type GetCallbacks<'res> = AllocGetCallbacks<'res>;

    #[cfg(not(feature = "alloc"))]
    type QueryableCallbacks<'res> = FixedCapacityQueryableCallbacks<
        'res,
        Self,
        8,
        zenoh::storage::RawOrBox<32>,
        zenoh::storage::RawOrBox<952>,
    >;

    #[cfg(feature = "alloc")]
    type QueryableCallbacks<'res> = AllocQueryableCallbacks<'res, Self>;

    #[cfg(not(feature = "alloc"))]
    type LivelinessCallbacks<'res> = FixedCapacityLivelinessCallbacks<
        'res,
        8,
        zenoh::storage::RawOrBox<56>,
        zenoh::storage::RawOrBox<600>,
    >;

    #[cfg(feature = "alloc")]
    type LivelinessCallbacks<'res> = AllocLivelinessCallbacks<'res>;

    fn buff(&self) -> Self::Buff {
        #[cfg(not(feature = "alloc"))]
        {
            [0u8; BUFF_SIZE as usize]
        }
        #[cfg(feature = "alloc")]
        {
            alloc::vec![0; BUFF_SIZE as usize]
        }
    }

    fn transports(&self) -> &TransportLinkManager<Self::LinkManager> {
        &self.transports
    }
}

pub fn init_broker_example() -> ExampleConfig {
    match () {
        #[cfg(feature = "std")]
        () => ExampleConfig {
            transports: TransportLinkManager::from(LinkManager),
        },
        #[cfg(feature = "wasm")]
        () => {
            unimplemented!()
        }
        #[cfg(not(any(feature = "std", feature = "wasm")))]
        () => {
            unimplemented!()
        }
    }
}

pub fn init_session_example() -> ExampleConfig {
    match () {
        #[cfg(feature = "std")]
        () => ExampleConfig {
            transports: TransportLinkManager::from(LinkManager),
        },
        #[cfg(feature = "wasm")]
        () => ExampleConfig {
            transports: TransportLinkManager::from(LinkManager),
        },
        #[cfg(not(any(feature = "std", feature = "wasm")))]
        () => {
            unimplemented!()
        }
    }
}


