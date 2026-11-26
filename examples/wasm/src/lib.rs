pub use zenoh_examples_common::{LISTEN, PAYLOAD};
pub type ExampleConfig = zenoh_examples_common::ExampleConfig<zenoh_wasm::WasmLinkManager, 32767>;
pub const ENDPOINT: &str = zenoh_examples_common::endpoint("ws/127.0.0.1:7446");

pub async fn init_session_example(_: &embassy_executor::Spawner) -> ExampleConfig {
    ExampleConfig::new(zenoh_wasm::WasmLinkManager)
}

#[cfg(feature = "alloc")]
pub use init_session_example as init_broker_example;
