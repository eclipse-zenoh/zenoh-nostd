use zenoh_examples::*;
use zenoh_nostd::session::*;

async fn entry(spawner: embassy_executor::Spawner) -> zenoh::ZResult<()> {
    #[cfg(feature = "log")]
    env_logger::init();

    zenoh::info!("zenoh-nostd z_sub example");

    // All channels that will be used must outlive `Resources`.
    // **Note**: as a direct implication, you may need to make static channels
    // if you want a `'static` session.
    use static_cell::StaticCell;
    static CHANNEL: StaticCell<
        embassy_sync::channel::Channel<
            embassy_sync::blocking_mutex::raw::NoopRawMutex,
            FixedCapacitySample<128, 128>,
            8,
        >,
    > = StaticCell::new();
    let channel: &'static _ = CHANNEL.init(embassy_sync::channel::Channel::new());

    let endpoint = Endpoint::try_from(ENDPOINT)?;
    let config = init_session_example(&spawner).await;
    let builder = {
        #[cfg(feature = "embedded")]
        {
            zenoh::session!(static ExampleConfig: config)
        }
        #[cfg(not(feature = "embedded"))]
        {
            zenoh::session!(ExampleConfig: config)
        }
    };
    let session = if LISTEN {
        builder.listen(endpoint).await?
    } else {
        builder.connect(endpoint).await?
    };

    let subscriber = session
        .declare_subscriber(zenoh::keyexpr::new("demo/example/**")?)
        .channel(channel.dyn_sender(), channel.dyn_receiver())
        .finish()
        .await?;

    embassy_futures::select::select(session.run(), async {
        while let Some(sample) = subscriber.recv().await {
            zenoh::info!(
                "[Subscriber] Received sample ('{}': '{}')",
                sample.keyexpr().as_str(),
                core::str::from_utf8(sample.payload()).unwrap()
            );
        }

        Ok::<(), Error>(())
    })
    .await;

    Ok(())
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
