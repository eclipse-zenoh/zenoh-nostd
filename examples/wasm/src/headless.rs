use static_cell::StaticCell;
use wasm_bindgen::prelude::*;
use zenoh_examples_common::scenarios::{self, Context, ReportFn, Reporter};
use zenoh_examples_wasm::{ENDPOINT, ExampleConfig};
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace=window, js_name=zenohOutput)]
    fn output(bytes: &[u8]);
}
#[embassy_executor::main]
async fn main(_spawner: embassy_executor::Spawner) {
    static CONTEXT: StaticCell<Context> = StaticCell::new();
    static CONFIG: StaticCell<ExampleConfig> = StaticCell::new();
    static REPORT: ReportFn = ReportFn(|text| output(text.as_bytes()));
    let config = CONFIG.init(ExampleConfig::new(zenoh_wasm::WasmLinkManager));
    match scenarios::configured() {
        Ok(options) => {
            let _ = scenarios::run(
                config,
                CONTEXT.init(Context::new()),
                &options,
                ENDPOINT,
                false,
                &REPORT,
            )
            .await;
        }
        Err(error) => REPORT.emit(format_args!("configuration error: {error}\r\n")),
    }
}
