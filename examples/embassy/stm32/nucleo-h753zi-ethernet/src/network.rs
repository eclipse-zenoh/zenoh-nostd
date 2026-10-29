use embassy_net::{Runner, StackResources};
use embassy_stm32::{
    bind_interrupts, eth,
    eth::{Ethernet, GenericPhy, PacketQueue},
    peripherals, rng,
};
use static_cell::StaticCell;
use zenoh_nostd::session::zenoh;

use crate::{ExampleConfig, LinkManager, runtime};

bind_interrupts!(struct Irqs {
    ETH => eth::InterruptHandler;
    HASH_RNG => rng::InterruptHandler<peripherals::RNG>;
});

type Device = Ethernet<'static, peripherals::ETH, GenericPhy>;

pub fn init_board() -> embassy_stm32::Peripherals {
    runtime::init_heap();
    let mut config = embassy_stm32::Config::default();
    config.rcc.hsi48 = Some(embassy_stm32::rcc::Hsi48Config {
        sync_from_usb: false,
    });
    embassy_stm32::init(config)
}
pub async fn init_session_example(spawner: &embassy_executor::Spawner) -> ExampleConfig {
    let p = init_board();
    init_network(
        spawner, p.ETH, p.RNG, p.PA1, p.PA2, p.PC1, p.PA7, p.PC4, p.PC5, p.PG13, p.PB13, p.PG11,
    )
    .await
}
#[allow(clippy::too_many_arguments)]
pub async fn init_network(
    spawner: &embassy_executor::Spawner,
    eth: embassy_stm32::Peri<'static, peripherals::ETH>,
    rng: embassy_stm32::Peri<'static, peripherals::RNG>,
    pa1: embassy_stm32::Peri<'static, peripherals::PA1>,
    pa2: embassy_stm32::Peri<'static, peripherals::PA2>,
    pc1: embassy_stm32::Peri<'static, peripherals::PC1>,
    pa7: embassy_stm32::Peri<'static, peripherals::PA7>,
    pc4: embassy_stm32::Peri<'static, peripherals::PC4>,
    pc5: embassy_stm32::Peri<'static, peripherals::PC5>,
    pg13: embassy_stm32::Peri<'static, peripherals::PG13>,
    pb13: embassy_stm32::Peri<'static, peripherals::PB13>,
    pg11: embassy_stm32::Peri<'static, peripherals::PG11>,
) -> ExampleConfig {
    let seed = runtime::init_rng(rng::Rng::new(rng, Irqs));

    // Locally administered unicast MAC derived from the chip's unique ID.
    let uid = embassy_stm32::uid::uid();
    let mac = [
        0x02,
        uid[0] ^ uid[5],
        uid[1] ^ uid[6],
        uid[2] ^ uid[7],
        uid[3] ^ uid[8],
        uid[4] ^ uid[9],
    ];
    static PACKETS: StaticCell<PacketQueue<4, 4>> = StaticCell::new();
    // NUCLEO-H753ZI (MB1364), onboard LAN8742A PHY, RMII pin assignments.
    let device = Ethernet::new(
        PACKETS.init_with(PacketQueue::new),
        eth,
        Irqs,
        pa1,
        pa2,
        pc1,
        pa7,
        pc4,
        pc5,
        pg13,
        pb13,
        pg11,
        GenericPhy::new(0),
        mac,
    );
    // Three Zenoh sockets (broker: two listeners + gateway) plus DHCP.
    static RESOURCES: StaticCell<StackResources<4>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        device,
        embassy_net::Config::dhcpv4(Default::default()),
        RESOURCES.init_with(StackResources::new),
        seed,
    );
    spawner.must_spawn(net_task(runner));
    zenoh::info!("Waiting for Ethernet link and DHCP");
    stack.wait_config_up().await;
    zenoh::info!("Ethernet ready: {}", stack.config_v4().unwrap().address);
    ExampleConfig::new(LinkManager::new(stack))
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, Device>) -> ! {
    runner.run().await
}
