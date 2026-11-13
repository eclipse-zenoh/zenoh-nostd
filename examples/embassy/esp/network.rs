use crate::{ExampleConfig, LinkManager};
use zenoh_nostd::session::zenoh;

use embassy_net::{DhcpConfig, Runner, StackResources};
use embassy_time::{Duration, Timer};
use esp_hal::{clock::CpuClock, rng::Rng, timer::timg::TimerGroup};
use esp_println as _;
use esp_radio::{
    Controller,
    wifi::{ClientConfig, ModeConfig, WifiController, WifiDevice, WifiEvent, WifiStaState},
};

const SSID: Option<&str> = option_env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASSWORD");

pub fn init_board() -> esp_hal::peripherals::Peripherals {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    crate::init_wifi_heap();
    peripherals
}

pub fn start_runtime(
    timer: esp_hal::peripherals::TIMG0<'static>,
    #[cfg(target_arch = "riscv32")] software: esp_hal::peripherals::SW_INTERRUPT<'static>,
) {
    let timg0 = TimerGroup::new(timer);
    #[cfg(target_arch = "xtensa")]
    esp_rtos::start(timg0.timer0);
    #[cfg(target_arch = "riscv32")]
    {
        let software = esp_hal::interrupt::software::SoftwareInterruptControl::new(software);
        esp_rtos::start(timg0.timer0, software.software_interrupt0);
    }
}

pub async fn init_session_example(spawner: &embassy_executor::Spawner) -> ExampleConfig {
    let peripherals = init_board();
    #[cfg(target_arch = "xtensa")]
    start_runtime(peripherals.TIMG0);
    #[cfg(target_arch = "riscv32")]
    start_runtime(peripherals.TIMG0, peripherals.SW_INTERRUPT);
    init_network(spawner, peripherals.WIFI, |_| {}).await
}

pub async fn init_network(
    spawner: &embassy_executor::Spawner,
    wifi: esp_hal::peripherals::WIFI<'static>,
    on_address: impl FnOnce(embassy_net::Ipv4Cidr),
) -> ExampleConfig {
    use static_cell::StaticCell;
    zenoh::info!("Embassy initialized!");

    let rng = Rng::new();

    static RADIO_CTRL: StaticCell<Controller<'static>> = StaticCell::new();
    let radio_ctrl = esp_radio::init().expect("Failed to init radio");

    zenoh::info!("Radio initialized");

    let (wifi_controller, interfaces) =
        esp_radio::wifi::new(RADIO_CTRL.init(radio_ctrl), wifi, Default::default())
            .expect("Failed to initialize WIFI controller");

    zenoh::info!("Wifi controller created");

    let wifi_interface = interfaces.sta;
    let net_seed = rng.random() as u64 | ((rng.random() as u64) << 32);
    let dhcp_config = DhcpConfig::default();
    let config = embassy_net::Config::dhcpv4(dhcp_config);

    static RESOURCES: StaticCell<StackResources<4>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        wifi_interface,
        config,
        RESOURCES.init_with(StackResources::new),
        net_seed,
    );

    zenoh::info!("Network stack created");

    spawner.must_spawn(connection(wifi_controller));
    spawner.must_spawn(net_task(runner));

    zenoh::info!("Waiting for link to be up");
    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    zenoh::info!("Waiting to get IP address...");
    stack.wait_config_up().await;
    let ip = stack
        .config_v4()
        .expect("DHCP configuration disappeared")
        .address;
    zenoh::info!("Network initialized with IP: {}", ip);
    on_address(ip);

    ExampleConfig::new(LinkManager::new(stack))
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    zenoh::info!("start connection task");
    zenoh::info!("Device capabilities: {:?}", controller.capabilities());
    loop {
        if matches!(esp_radio::wifi::sta_state(), WifiStaState::Connected) {
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
            Timer::after(Duration::from_millis(5000)).await;
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = ModeConfig::Client(
                ClientConfig::default()
                    .with_ssid(SSID.unwrap_or("Zenoh").into())
                    .with_password(PASSWORD.into()),
            );
            controller.set_config(&client_config).unwrap();
            zenoh::info!("Starting wifi");
            controller.start_async().await.unwrap();
            zenoh::info!("Wifi started!");
        }
        zenoh::info!("About to connect...");

        match controller.connect_async().await {
            Ok(_) => zenoh::info!("Wifi connected!"),
            Err(e) => {
                zenoh::info!("Failed to connect to wifi: {:?}", e);
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}
