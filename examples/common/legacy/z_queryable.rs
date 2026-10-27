use zenoh_examples::*;
use zenoh_nostd::session::*;

#[embassy_executor::task]
async fn session_task(session: &'static Session<'static, ExampleConfig>) {
    if let Err(e) = session.run().await {
        zenoh::error!("Error in session task: {}", e);
    }
}

async fn entry(spawner: embassy_executor::Spawner) -> zenoh::ZResult<()> {
    #[cfg(feature = "log")]
    env_logger::init();

    zenoh::info!("zenoh-nostd z_queryable example");

    // All channels that will be used must outlive `Resources`.
    // **Note**: as a direct implication, here you need to make a static channel.
    static CHANNEL: static_cell::StaticCell<
        embassy_sync::channel::Channel<
            embassy_sync::blocking_mutex::raw::NoopRawMutex,
            FixedCapacityQueryableQuery<ExampleConfig, 128, 128, 128>,
            8,
        >,
    > = static_cell::StaticCell::new();
    let channel = CHANNEL.init_with(embassy_sync::channel::Channel::new);

    let endpoint = Endpoint::try_from(ENDPOINT)?;
    let config = init_session_example(&spawner).await;
    let builder = zenoh::session!(static ExampleConfig: config);
    let session = if LISTEN {
        builder.listen(endpoint).await?
    } else {
        builder.connect(endpoint).await?
    };

    spawner.spawn(session_task(session)).unwrap();

    // Because of known limitations, the `Queryable` API (both callbacks and channels) needs the `session` to be static.
    // This may change in the future.

    let queryable = session
        .declare_queryable(zenoh::keyexpr::new("demo/example/**")?)
        .channel(channel.dyn_sender(), channel.dyn_receiver())
        .finish()
        .await?;

    while let Some(mut query) = queryable.recv().await {
        match query.payload() {
            None => {
                zenoh::info!(
                    "[Queryable] Received Query ('{}' with no payload)",
                    query.keyexpr().as_str()
                );
            }
            Some(payload) => {
                zenoh::info!(
                    "[Queryable] Received Query ('{}': '{:?}')",
                    query.keyexpr().as_str(),
                    core::str::from_utf8(payload).unwrap()
                );
            }
        }

        zenoh::info!("[Queryable] Sending OK Reply");

        let _ = query
            .reply(query.keyexpr(), b"Response from z_queryable")
            .await;

        let _ = query.finalize().await;
    }

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
