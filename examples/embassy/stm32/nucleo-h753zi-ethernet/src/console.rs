#[cfg(not(feature = "console-usb"))]
use embassy_stm32::{bind_interrupts, peripherals, usart};
use static_cell::StaticCell;
use zenoh_examples_common::cli::{self, Cli};
#[cfg(not(feature = "console-usb"))]
use zenoh_examples_common::cli::{ConsoleRead, ConsoleWrite};
use zenoh_examples_nucleo_h753zi_ethernet::*;

#[cfg(not(feature = "console-usb"))]
bind_interrupts!(struct ConsoleIrqs {
    USART3 => usart::BufferedInterruptHandler<peripherals::USART3>;
});
#[cfg(not(feature = "console-usb"))]
struct Rx(usart::BufferedUartRx<'static>);
#[cfg(not(feature = "console-usb"))]
struct Tx(usart::BufferedUartTx<'static>);
#[cfg(not(feature = "console-usb"))]
impl ConsoleRead for Rx {
    async fn read(&mut self, bytes: &mut [u8]) -> Result<usize, ()> {
        embedded_io_async_06::Read::read(&mut self.0, bytes)
            .await
            .map_err(|_| ())
    }
}
#[cfg(not(feature = "console-usb"))]
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
    #[cfg(not(feature = "console-usb"))]
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
    #[cfg(feature = "console-usb")]
    let (mut device, rx, tx) = {
        use embassy_stm32::{bind_interrupts, peripherals, usb};
        bind_interrupts!(struct UsbIrqs {
            OTG_FS => usb::InterruptHandler<peripherals::USB_OTG_FS>;
        });
        static OUT: StaticCell<[u8; 256]> = StaticCell::new();
        static BUFFERS: StaticCell<zenoh_examples_embassy::usb_console::Buffers> =
            StaticCell::new();
        let driver = usb::Driver::new_fs(
            p.USB_OTG_FS,
            UsbIrqs,
            p.PA12,
            p.PA11,
            OUT.init([0; 256]),
            usb::Config::default(),
        );
        let mut config = embassy_usb::Config::new(0xc0de, 0xcafe);
        config.manufacturer = Some("zenoh-nostd examples");
        config.product = Some("Zenoh NUCLEO-H753ZI console");
        config.self_powered = true;
        config.max_power = 0;
        zenoh_examples_embassy::usb_console::new(
            driver,
            config,
            BUFFERS.init_with(Default::default),
        )
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
    #[cfg(feature = "console-usb")]
    let _ = embassy_futures::select::select(device.run(), application).await;
    #[cfg(not(feature = "console-usb"))]
    let _ = application.await;
}
