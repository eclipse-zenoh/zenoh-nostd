use static_cell::StaticCell;
#[cfg(not(feature = "console-usb"))]
use zenoh_examples_common::cli::IoRead;
#[cfg(not(feature = "console-usb"))]
use zenoh_examples_common::cli::IoWrite;
use zenoh_examples_common::cli::{self, Cli};
use zenoh_examples_esp32s3_wifi::*;

#[esp_rtos::main]
async fn main(spawner: embassy_executor::Spawner) {
    let p = init_board();
    start_runtime(p.TIMG0);
    // UART1 keeps the text console separate from ESP defmt output.
    #[cfg(not(feature = "console-usb"))]
    let uart = esp_hal::uart::Uart::new(
        p.UART1,
        esp_hal::uart::Config::default().with_baudrate(115200),
    )
    .unwrap()
    .with_tx(p.GPIO17)
    .with_rx(p.GPIO18)
    .into_async();
    #[cfg(not(feature = "console-usb"))]
    let (rx, tx) = {
        let (rx, tx) = uart.split();
        (IoRead(rx), IoWrite(tx))
    };
    #[cfg(feature = "console-usb")]
    let (mut device, rx, tx) = {
        use esp_hal::otg_fs::{
            Usb,
            asynch::{Config, Driver},
        };
        static OUT: StaticCell<[u8; 256]> = StaticCell::new();
        static BUFFERS: StaticCell<zenoh_examples_embassy::usb_console::Buffers> =
            StaticCell::new();
        let usb = Usb::new(p.USB0, p.GPIO20, p.GPIO19);
        let driver = Driver::new(usb, OUT.init([0; 256]), Config::default());
        let mut config = embassy_usb::Config::new(0xc0de, 0xcafe);
        config.manufacturer = Some("zenoh-nostd examples");
        config.product = Some("Zenoh ESP32-S3 console");
        zenoh_examples_embassy::usb_console::new(
            driver,
            config,
            BUFFERS.init_with(Default::default),
        )
    };
    static CLI: StaticCell<Cli> = StaticCell::new();
    static CONFIG: StaticCell<ExampleConfig> = StaticCell::new();
    let console: &'static Cli = CLI.init(Cli::new());
    let init = async {
        &*CONFIG.init(init_network(&spawner, p.WIFI, |ip| console.set_network_address(ip)).await)
    };
    let application = cli::run(
        console,
        rx,
        tx,
        init,
        option_env!("ENDPOINT").unwrap_or(""),
        true,
    );
    #[cfg(feature = "console-usb")]
    let _ = embassy_futures::select::select(device.run(), application).await;
    #[cfg(not(feature = "console-usb"))]
    let _ = application.await;
}
