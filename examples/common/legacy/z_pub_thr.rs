use zenoh_examples::*;
use zenoh_nostd::session::*;

async fn entry(spawner: embassy_executor::Spawner) -> zenoh::ZResult<()> {
    #[cfg(feature = "log")]
    env_logger::init();

    zenoh::info!("zenoh-nostd z_pub_thr example");

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
    let builder = builder.ignore_invalid_sn();
    let session = if LISTEN {
        builder.listen(endpoint).await?
    } else {
        builder.connect(endpoint).await?
    };

    let payload: [u8; PAYLOAD] = core::array::from_fn(|i| (i % 10) as u8);
    let publisher = session
        .declare_publisher(zenoh::keyexpr::new("test/thr")?)
        .finish()
        .await?;

    let mut count: usize = 0;
    let mut start = embassy_time::Instant::now();
    embassy_futures::select::select(session.run(), async {
        loop {
            if let Err(e) = publisher.put(&payload).finish().await {
                zenoh::error!("Error publishing message: {}", e);
                break;
            }

            if count < 100_000 {
                count += 1;
            } else {
                let thpt = count as f64 / (start.elapsed().as_micros() as f64 / 1_000_000.0);
                zenoh::info!("{} msgs/s", thpt);
                count = 0;
                start = embassy_time::Instant::now();
            }
        }

        Ok::<(), Error>(())
    })
    .await;

    Ok(())
}

#[cfg_attr(feature = "std", embassy_executor::main)]
#[cfg_attr(feature = "esp", esp_rtos::main)]
#[cfg_attr(feature = "stm32", embassy_executor::main)]
#[cfg_attr(feature = "wasm", embassy_executor::main)]
async fn main(spawner: embassy_executor::Spawner) {
    if let Err(e) = entry(spawner).await {
        zenoh::error!("Error in main: {}", e);
    }

    zenoh::info!("Exiting main");
}
