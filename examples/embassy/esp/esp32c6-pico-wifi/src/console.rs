use static_cell::StaticCell;
use zenoh_examples_common::cli::IoRead;
use zenoh_examples_common::cli::{self, Cli, IoWrite};
use zenoh_examples_esp32c6_pico_wifi::*;

#[esp_rtos::main]
async fn main(spawner: embassy_executor::Spawner) {
    let p = init_board();
    start_runtime(p.TIMG0, p.SW_INTERRUPT);
    // UART1 keeps the text console separate from ESP defmt output.
    let uart = esp_hal::uart::Uart::new(
        p.UART1,
        esp_hal::uart::Config::default().with_baudrate(115200),
    )
    .unwrap()
    .with_tx(p.GPIO4)
    .with_rx(p.GPIO5)
    .into_async();
    let (rx, tx) = {
        let (rx, tx) = uart.split();
        (IoRead(rx), IoWrite(tx))
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
