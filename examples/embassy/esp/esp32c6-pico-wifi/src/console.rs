use static_cell::StaticCell;
#[cfg(not(feature = "console-usb"))]
use zenoh_examples_common::cli::IoRead;
use zenoh_examples_common::cli::{self, Cli, IoWrite};
use zenoh_examples_esp32c6_pico_wifi::*;

#[esp_rtos::main]
async fn main(spawner: embassy_executor::Spawner) {
    let p = init_board();
    start_runtime(p.TIMG0, p.SW_INTERRUPT);
    // UART1 keeps the text console separate from ESP defmt output.
    #[cfg(not(feature = "console-usb"))]
    let uart = esp_hal::uart::Uart::new(
        p.UART1,
        esp_hal::uart::Config::default().with_baudrate(115200),
    )
    .unwrap()
    .with_tx(p.GPIO4)
    .with_rx(p.GPIO5)
    .into_async();
    #[cfg(not(feature = "console-usb"))]
    let (rx, tx) = {
        let (rx, tx) = uart.split();
        (IoRead(rx), IoWrite(tx))
    };
    #[cfg(feature = "console-usb")]
    let (rx, tx) = {
        let (rx, tx) = esp_hal::usb_serial_jtag::UsbSerialJtag::new(p.USB_DEVICE)
            .into_async()
            .split();
        (usb_serial_jtag::UsbRead(rx), IoWrite(tx))
    };
    static CLI: StaticCell<Cli> = StaticCell::new();
    static CONFIG: StaticCell<ExampleConfig> = StaticCell::new();
    let console: &'static Cli = CLI.init(Cli::new());
    let init = async {
        &*CONFIG.init(init_network(&spawner, p.WIFI, |ip| console.set_network_address(ip)).await)
    };
    let _ = cli::run(
        console,
        rx,
        tx,
        init,
        option_env!("ENDPOINT").unwrap_or(""),
        true,
    )
    .await;
}
