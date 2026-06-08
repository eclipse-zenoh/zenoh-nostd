#![cfg_attr(feature = "esp32s3", no_std)]
#![cfg_attr(feature = "esp32s3", no_main)]
#![cfg_attr(feature = "wasm", no_main)]

use zenoh_examples::*;
use zenoh_nostd::session::*;

fn entry() -> zenoh::ZResult<()> {
    #[cfg(feature = "log")]
    env_logger::init();

    zenoh::info!("zenoh-nostd z_mesh example");

    let ke = zenoh::keyexpr::new("demo/mesh")?;

    let cfg_a = init_session_example().with_transports(
        TransportLinkManager::from(LinkManager).with_whatami(zenoh_nostd::session::WhatAmI::Peer),
    );
    let session_a = zenoh::listen!(
        ExampleConfig: cfg_a,
        Endpoint::try_from("tcp/127.0.0.1:7444")?
    );

    let _sub_a = session_a
        .declare_subscriber(ke)
        .callback_sync(|sample| {
            zenoh::info!(
                "[Peer A] Received: {:?}",
                core::str::from_utf8(sample.payload()).unwrap_or("")
            );
        })
        .finish()?;

    let cfg_b = init_session_example().with_transports(
        TransportLinkManager::from(LinkManager).with_whatami(zenoh_nostd::session::WhatAmI::Peer),
    );
    let session_b = zenoh::connect!(
        ExampleConfig: cfg_b,
        Endpoint::try_from("tcp/127.0.0.1:7444")?
    );

    let publisher = session_b.declare_publisher(ke).finish()?;

    for i in 0..5 {
        let payload = b"Hello from peer B";
        publisher.put(payload).finish()?;
        zenoh::info!("[Peer B] Sent #{}", i);
    }

    zenoh::info!("Mesh example done");
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
