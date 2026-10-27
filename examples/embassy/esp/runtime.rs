use esp_hal::rng::Rng;
use zenoh_nostd::session::zenoh;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    zenoh::error!("Panic: {}", info);
    loop {}
}

esp_bootloader_esp_idf::esp_app_desc!();
getrandom::register_custom_getrandom!(getrandom_custom);
fn getrandom_custom(bytes: &mut [u8]) -> Result<(), getrandom::Error> {
    Rng::new().read(bytes);
    Ok(())
}

// Keep USB text mode free of binary defmt traffic. This logger has no shared
// state and deliberately discards records; command results use the CLI writer.
#[cfg(feature = "console-usb")]
#[defmt::global_logger]
struct SilentLogger;
#[cfg(feature = "console-usb")]
unsafe impl defmt::Logger for SilentLogger {
    fn acquire() {}
    unsafe fn release() {}
    unsafe fn write(_: &[u8]) {}
    unsafe fn flush() {}
}
