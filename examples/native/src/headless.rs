use zenoh_examples_common::scenarios::{self, Context, Options, ReportFn};
use zenoh_examples_native::{ENDPOINT, ExampleConfig};

#[embassy_executor::task]
async fn application() {
    std::process::exit(run().await);
}
fn main() {
    let executor = Box::leak(Box::new(embassy_executor::Executor::new()));
    executor.run(|spawner| spawner.must_spawn(application()));
}
async fn run() -> i32 {
    let _ = env_logger::try_init();
    if std::env::args_os().len() > 1 {
        eprintln!(
            "Headless builds use scenario settings; enable the cli feature to accept commands."
        );
        return 2;
    }
    let values = scenarios::SETTING_NAMES.map(|name| (name, std::env::var(name).ok()));
    let options = match Options::from_lookup(|name| {
        values
            .iter()
            .find(|(key, _)| *key == name)
            .and_then(|(_, value)| value.as_deref())
            .or_else(|| scenarios::compiled_setting(name))
    }) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("configuration error: {error}");
            return 2;
        }
    };
    let config = Box::leak(Box::new(ExampleConfig::new(zenoh_std::StdLinkManager)));
    let context = Box::leak(Box::new(Context::new()));
    static REPORT: ReportFn = ReportFn(|text| print!("{text}"));
    let endpoint = std::env::var("ENDPOINT").unwrap_or_else(|_| ENDPOINT.into());
    scenarios::run(config, context, &options, &endpoint, true, &REPORT).await
}
