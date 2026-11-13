pub use zenoh_examples_common::{LISTEN, PAYLOAD};
pub type ExampleConfig = zenoh_examples_common::ExampleConfig<zenoh_std::StdLinkManager, 65535>;
pub const ENDPOINT: &str = zenoh_examples_common::endpoint("tcp/127.0.0.1:7447");

pub async fn init_session_example(_: &embassy_executor::Spawner) -> ExampleConfig {
    let _ = env_logger::try_init();
    ExampleConfig::new(zenoh_std::StdLinkManager)
}

#[cfg(feature = "alloc")]
pub use init_session_example as init_broker_example;
