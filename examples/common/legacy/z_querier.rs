use core::time::Duration;

use zenoh_examples::*;
use zenoh_nostd::session::*;

fn response_callback(resp: &GetResponse<'_>) {
    match resp {
        GetResponse::Ok(reply) => {
            zenoh::info!(
                "[Querier] Received OK Reply ('{}': '{:?}')",
                reply.keyexpr().as_str(),
                core::str::from_utf8(reply.payload()).unwrap()
            );
        }
        GetResponse::Err(reply) => {
            zenoh::error!(
                "[Querier] Received ERR Reply ('{}': '{:?}')",
                reply.keyexpr().as_str(),
                core::str::from_utf8(reply.payload()).unwrap()
            );
        }
    }
}

#[embassy_executor::task]
async fn session_task(session: &'static Session<'static, ExampleConfig>) {
    if let Err(e) = session.run().await {
        zenoh::error!("Error in session task: {}", e);
    }
}

async fn entry(spawner: embassy_executor::Spawner) -> zenoh::ZResult<()> {
    #[cfg(feature = "log")]
    env_logger::init();

    zenoh::info!("zenoh-nostd z_querier example");

    let endpoint = Endpoint::try_from(ENDPOINT)?;
    let config = init_session_example(&spawner).await;
    let builder = zenoh::session!(static ExampleConfig: config);
    let session = if LISTEN {
        builder.listen(endpoint).await?
    } else {
        builder.connect(endpoint).await?
    };

    spawner.spawn(session_task(session)).unwrap();

    let querier = session
        .declare_querier(zenoh::keyexpr::new("demo/example/**")?)
        .timeout(Duration::from_secs(1))
        .finish()
        .await?;

    loop {
        querier
            .get()
            .callback_sync(response_callback)
            .finish()
            .await?;

        embassy_time::Timer::after(embassy_time::Duration::from_secs(1)).await;
    }
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
