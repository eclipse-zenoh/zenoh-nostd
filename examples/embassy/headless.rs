//! MCU startup scenario policy shared by ESP and STM32 applications.
use static_cell::StaticCell;
use zenoh_examples_common::{
    AppConfig,
    scenarios::{self, Context, Reporter},
};

/// Run after the board has initialized its executor, network and static config.
/// Each application owns its configuration and diagnostic reporter. The bounded
/// callback context is initialized once for this startup run.
pub async fn run<C: AppConfig + 'static, R: Reporter + 'static>(
    config: &'static C,
    endpoint: &str,
    reporter: &'static R,
) -> i32 {
    static CONTEXT: StaticCell<Context> = StaticCell::new();
    match scenarios::configured() {
        Ok(options) => {
            scenarios::run(
                config,
                CONTEXT.init(Context::new()),
                &options,
                endpoint,
                true,
                reporter,
            )
            .await
        }
        Err(error) => {
            reporter.emit(format_args!("configuration error: {error}\r\n"));
            2
        }
    }
}
