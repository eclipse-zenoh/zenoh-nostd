#![cfg_attr(feature = "esp32s3", no_std)]
#![cfg_attr(feature = "esp32s3", no_main)]
#![cfg_attr(feature = "wasm", no_main)]

use zenoh_examples::*;
use zenoh_nostd::session::*;

fn entry() -> zenoh::ZResult<()> {
    #[cfg(feature = "log")]
    env_logger::init();

    zenoh::info!("zenoh-nostd z_open example");

    let config = init_session_example();
    let session = if LISTEN {
        zenoh::listen!(ExampleConfig: config, Endpoint::try_from(ENDPOINT)?)
    } else {
        zenoh::connect!(ExampleConfig: config, Endpoint::try_from(ENDPOINT)?)
    };

    session.run()?;

    Ok(())
}

fn main() {
    if let Err(e) = entry() {
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