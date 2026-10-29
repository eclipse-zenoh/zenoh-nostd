use zenoh_examples::*;
use zenoh_nostd::broker::*;

#[embassy_executor::task(pool_size = 2)]
async fn south(broker: &'static Broker<ExampleConfig>, endpoint: Endpoint<'static>) {
    // `broker.accept` creates a listening `Endpoint`: a client can connect to the broker, one at a time
    if let Err(e) = broker.accept(endpoint.clone()).await {
        zenoh::error!("Fatal error on south {}: {}", endpoint, e);
    }
}

async fn entry(spawner: embassy_executor::Spawner) -> zenoh::ZResult<()> {
    #[cfg(feature = "log")]
    env_logger::init();

    zenoh::info!("zenoh-nostd z_broker example");

    let config = init_broker_example(&spawner).await;
    let broker = zenoh::broker!(ExampleConfig: config);

    spawner.must_spawn(south(broker, Endpoint::try_from("tcp/127.0.0.1:7444")?));
    spawner.must_spawn(south(broker, Endpoint::try_from("tcp/127.0.0.1:7445")?));

    // `broker.open` defines the gateway of this broker, it will fail if there is nobody listening at that `Endpoint`

    Ok(broker
        .open(Endpoint::try_from("tcp/127.0.0.1:7447")?)
        .await?)
}

#[cfg_attr(feature = "std", embassy_executor::main)]
#[cfg_attr(feature = "wasm", embassy_executor::main)]
#[cfg_attr(feature = "esp", esp_rtos::main)]
#[cfg_attr(feature = "stm32", embassy_executor::main)]
async fn main(spawner: embassy_executor::Spawner) {
    if let Err(e) = entry(spawner).await {
        zenoh::error!("Error in main: {}", e);
    }

    zenoh::info!("Exiting main");
}
