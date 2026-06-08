#![cfg_attr(feature = "esp32s3", no_std)]
#![cfg_attr(feature = "esp32s3", no_main)]
#![cfg_attr(feature = "wasm", no_main)]

use zenoh_examples::*;
use zenoh_nostd::session::*;

struct Stats {
    count: usize,
    round_size: usize,
    #[cfg(feature = "std")]
    start: std::time::Instant,
}

fn entry() -> zenoh::ZResult<()> {
    #[cfg(feature = "log")]
    env_logger::init();

    zenoh::info!("zenoh-nostd z_sub_thr example");

    let config = init_session_example();
    let mut resources = Resources::default();
    let session = if LISTEN {
        zenoh::listen_ignore_invalid_sn(&mut resources, &config, Endpoint::try_from(ENDPOINT)?)?
    } else {
        zenoh::connect_ignore_invalid_sn(&mut resources, &config, Endpoint::try_from(ENDPOINT)?)?
    };

    let mut stats = Stats {
        count: 0,
        round_size: 100_000,
        #[cfg(feature = "std")]
        start: std::time::Instant::now(),
    };

    let _subscriber = session
        .declare_subscriber(zenoh::keyexpr::new("test/thr")?)
        .callback_sync(move |_| {
            stats.count += 1;
            if stats.count >= stats.round_size {
                #[cfg(feature = "std")]
                {
                    let elapsed = stats.start.elapsed().as_secs_f64();
                    let throughput = stats.round_size as f64 / elapsed;
                    zenoh::info!("{} msg/s", throughput);
                }
                stats.count = 0;
                #[cfg(feature = "std")]
                {
                    stats.start = std::time::Instant::now();
                }
            }
        })
        .finish()?;

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