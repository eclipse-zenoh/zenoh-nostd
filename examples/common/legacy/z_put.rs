use zenoh_examples::*;
use zenoh_nostd::session::*;

async fn entry(spawner: embassy_executor::Spawner) -> zenoh::ZResult<()> {
    #[cfg(feature = "log")]
    env_logger::init();

    zenoh::info!("zenoh-nostd z_put example");

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

    // In this example we don't care about maintaining the session alive but we do it anyway for demonstration purpose. Know
    // that it's not mandatory to do a `session.run()` if you just need to `put` a value on the network.
    // We then have two choices:
    //  1) Spawn a new task to run the `session.run()` in background, but it requires the `session` to be `'static`.
    //  2) Use `select` or `join` to run both the session and the subscriber in the same task.
    // Here we use the second approach. For a demonstration of the first approach, see the `z_open` example.

    let ke = zenoh::keyexpr::new("demo/example")?;
    let payload = b"Hello, from no-std!";

    embassy_futures::select::select(session.run(), async {
        match session.put(ke, payload).finish().await {
            Ok(_) => {
                zenoh::info!(
                    "[Put] Sent PUT ('{}': '{}')",
                    ke.as_str(),
                    core::str::from_utf8(payload).unwrap()
                );
            }
            Err(e) => {
                zenoh::error!("{}", e)
            }
        };
    })
    .await;

    embassy_time::Timer::after(embassy_time::Duration::from_millis(100)).await;

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
