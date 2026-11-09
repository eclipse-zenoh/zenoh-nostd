#![no_std]

#[path = "../../network.rs"]
mod network;
#[path = "../../runtime.rs"]
mod runtime;

pub use network::{init_board, init_network, init_session_example, start_runtime};
pub use zenoh_examples_common::{LISTEN, PAYLOAD};
pub type LinkManager = zenoh_embassy::EmbassyLinkManager<'static, 512, 3>;
pub type ExampleConfig = zenoh_examples_common::ExampleConfig<LinkManager, 512>;
pub const ENDPOINT: &str = zenoh_examples_common::endpoint("");

#[cfg(feature = "alloc")]
pub use init_session_example as init_broker_example;

// The radio stack requires a heap even with Zenoh allocation disabled.
pub(crate) fn init_wifi_heap() {
    // C6 reclaimed RAM is smaller than the radio heap; use ordinary internal SRAM.
    esp_alloc::heap_allocator!(size: 73744);
}
