#![cfg_attr(feature = "esp32s3", no_std)]
#![cfg_attr(feature = "esp32s3", no_main)]
#![cfg_attr(feature = "wasm", no_main)]

use zenoh_examples::*;
use zenoh_nostd::broker::*;

/// Pool of 3: tcp/7444, tcp/7445, ws/7447
#[embassy_executor::task(pool_size = 3)]
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

    // TCP south connections (for standard Rust clients)
    spawner.must_spawn(south(broker, Endpoint::try_from("tcp/127.0.0.1:7444")?));
    spawner.must_spawn(south(broker, Endpoint::try_from("tcp/127.0.0.1:7445")?));
    // WebSocket south connection (for WASM / browser clients)
    spawner.must_spawn(south(broker, Endpoint::try_from("ws/127.0.0.1:7447")?));

    // Run forever — no upstream gateway needed for standalone use.
    core::future::pending::<()>().await;
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
