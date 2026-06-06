#![cfg_attr(feature = "esp32s3", no_std)]
#![cfg_attr(feature = "esp32s3", no_main)]
#![cfg_attr(feature = "wasm", no_main)]

use zenoh_examples::*;
use zenoh_nostd::session::*;

async fn entry(spawner: embassy_executor::Spawner) -> zenoh::ZResult<()> {
    #[cfg(feature = "log")]
    env_logger::init();

    zenoh::info!("zenoh-nostd z_liveliness example");

    let config = init_session_example(&spawner).await;
    let mut resources = Resources::default();
    let session = if LISTEN {
        zenoh::listen(&mut resources, &config, Endpoint::try_from(ENDPOINT)?).await?
    } else {
        zenoh::connect(&mut resources, &config, Endpoint::try_from(ENDPOINT)?).await?
    };

    let _token = session
        .declare_liveliness_token(zenoh::keyexpr::new("demo/example")?)
        .await?;

    let _subscriber = session
        .declare_liveliness_subscriber(zenoh::keyexpr::new("demo/**")?)
        .callback_sync(|sample| {
            zenoh::info!(
                "[Liveliness] Received liveliness sample ('{}': '{:?}')",
                sample.keyexpr().as_str(),
                sample.payload()
            );
        })
        .finish()
        .await?;

    session.run().await?;

    Ok(())
}

#[cfg_attr(feature = "std", embassy_executor::main)]
#[cfg_attr(feature = "wasm", embassy_executor::main)]
#[cfg_attr(feature = "esp32s3", esp_rtos::main)]
async fn main(spawner: embassy_executor::Spawner) {
    if let Err(e) = entry(spawner).await {
        zenoh::error!("Error in main: {}", e);
    }

    zenoh::info!("Exiting main");
}

#[cfg(feature = "esp32s3")]
mod esp32s3_app {
    use esp_hal::rng::Rng;
    pub use esp_println as _;
    use getrandom::{Error, register_custom_getrandom};

    #[panic_handler]
    fn panic(info: &core::panic::PanicInfo) -> ! {
        zenoh_nostd::session::zenoh::error!("Panic: {}", info);

        loop {}
    }

    extern crate alloc;

    esp_bootloader_esp_idf::esp_app_desc!();

    register_custom_getrandom!(getrandom_custom);
    pub fn getrandom_custom(bytes: &mut [u8]) -> Result<(), Error> {
        Rng::new().read(bytes);
        Ok(())
    }
}
