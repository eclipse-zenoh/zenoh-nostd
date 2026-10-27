use embassy_stm32::{bind_interrupts, peripherals, usart};
use static_cell::StaticCell;
use zenoh_examples_common::cli::{self, Cli};
use zenoh_examples_common::cli::{ConsoleRead, ConsoleWrite};
use zenoh_examples_nucleo_h753zi_ethernet::*;
bind_interrupts!(struct ConsoleIrqs {
    USART3 => usart::BufferedInterruptHandler<peripherals::USART3>;
});
struct Rx(usart::BufferedUartRx<'static>);
struct Tx(usart::BufferedUartTx<'static>);
impl ConsoleRead for Rx {
    async fn read(&mut self, bytes: &mut [u8]) -> Result<usize, ()> {
        embedded_io_async_06::Read::read(&mut self.0, bytes)
            .await
            .map_err(|_| ())
    }
}
impl ConsoleWrite for Tx {
    async fn write(&mut self, bytes: &[u8]) -> Result<(), ()> {
        embedded_io_async_06::Write::write_all(&mut self.0, bytes)
            .await
            .map_err(|_| ())
    }
}

#[embassy_executor::main]
async fn main(spawner: embassy_executor::Spawner) {
    let p = init_board();
    let (rx, tx) = {
        static TX: StaticCell<[u8; 512]> = StaticCell::new();
        static RX: StaticCell<[u8; 512]> = StaticCell::new();
        let mut config = usart::Config::default();
        config.baudrate = 115200;
        let uart = usart::BufferedUart::new(
            p.USART3,
            p.PD9,
            p.PD8,
            TX.init([0; 512]),
            RX.init([0; 512]),
            ConsoleIrqs,
            config,
        )
        .unwrap();
        let (tx, rx) = uart.split();
        (Rx(rx), Tx(tx))
    };
    static CLI: StaticCell<Cli> = StaticCell::new();
    static CONFIG: StaticCell<ExampleConfig> = StaticCell::new();
    let init = async {
        &*CONFIG.init(
            init_network(
                &spawner, p.ETH, p.RNG, p.PA1, p.PA2, p.PC1, p.PA7, p.PC4, p.PC5, p.PG13, p.PB13,
                p.PG11,
            )
            .await,
        )
    };
    let application = cli::run(
        CLI.init(Cli::new()),
        rx,
        tx,
        init,
        option_env!("ENDPOINT").unwrap_or(""),
        true,
    );
    let _ = application.await;
}
