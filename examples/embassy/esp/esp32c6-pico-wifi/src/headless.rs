use static_cell::StaticCell;
use zenoh_examples_common::scenarios::ReportFn;
use zenoh_examples_esp32c6_pico_wifi::*;

#[esp_rtos::main]
async fn main(spawner: embassy_executor::Spawner) {
    static CONFIG: StaticCell<ExampleConfig> = StaticCell::new();
    static REPORT: ReportFn =
        ReportFn(|text| zenoh_nostd::session::zenoh::info!("{}", text.trim_end()));
    let config = CONFIG.init(init_session_example(&spawner).await);
    let _ = zenoh_examples_embassy::headless::run(config, ENDPOINT, &REPORT).await;
}
