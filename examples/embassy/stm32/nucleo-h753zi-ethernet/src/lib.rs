#![no_std]

mod network;
mod runtime;

pub use network::{init_board, init_network, init_session_example};
pub use zenoh_examples_common::{LISTEN, PAYLOAD};
pub type LinkManager = zenoh_embassy::EmbassyLinkManager<'static, 512, 3>;
pub type ExampleConfig = zenoh_examples_common::ExampleConfig<LinkManager, 512>;
pub const ENDPOINT: &str = zenoh_examples_common::endpoint("");

#[cfg(feature = "alloc")]
pub use init_session_example as init_broker_example;
