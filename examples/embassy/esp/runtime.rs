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
