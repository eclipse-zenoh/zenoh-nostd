use static_cell::StaticCell;
use zenoh_examples::*;
use zenoh_nostd::session::*;

struct Stats {
    round_count: usize,
    round_size: usize,
    finished_rounds: usize,
    round_start: embassy_time::Instant,
    global_start: Option<embassy_time::Instant>,
}

impl Stats {
    fn increment(&mut self) {
        if self.round_count == 0 {
            self.round_start = embassy_time::Instant::now();
            if self.global_start.is_none() {
                self.global_start = Some(self.round_start)
            }
            self.round_count += 1;
        } else if self.round_count < self.round_size {
            self.round_count += 1;
        } else {
            self.print_round();
            self.finished_rounds += 1;
            self.round_count = 0;
        }
    }

    fn print_round(&self) {
        let elapsed = self.round_start.elapsed().as_micros() as f64 / 1_000_000.0;
        let throughput = (self.round_size as f64) / elapsed;
        zenoh::info!("{} msg/s", throughput);
    }
}

async fn entry(spawner: embassy_executor::Spawner) -> zenoh::ZResult<()> {
    #[cfg(feature = "log")]
    env_logger::init();

    zenoh::info!("zenoh-nostd z_sub_thr example");

    type CloseSignal =
        embassy_sync::signal::Signal<embassy_sync::blocking_mutex::raw::NoopRawMutex, ()>;
    static CLOSE: StaticCell<CloseSignal> = StaticCell::new();
    let close: &'static CloseSignal = CLOSE.init_with(CloseSignal::new);

    let endpoint = Endpoint::try_from(ENDPOINT)?;
    let config = init_session_example(&spawner).await;
    let builder = {
        #[cfg(feature = "embedded")]
        {
            zenoh::session!(static ExampleConfig: config)
        }
        #[cfg(not(feature = "embedded"))]
        {
            zenoh::session!(ExampleConfig: config)
        }
    };
    let builder = builder.ignore_invalid_sn();
    let session = if LISTEN {
        builder.listen(endpoint).await?
    } else {
        builder.connect(endpoint).await?
    };

    // Capture a pointer so the callback fits the fixed storage's alignment on 32-bit targets.
    static STATS: StaticCell<Stats> = StaticCell::new();
    let stats = STATS.init_with(|| Stats {
        round_count: 0,
        round_size: 100_000,
        finished_rounds: 0,
        round_start: embassy_time::Instant::now(),
        global_start: None,
    });

    let send = close;
    session
        .declare_subscriber(zenoh::keyexpr::new("test/thr")?)
        .callback_sync(move |_| {
            if stats.finished_rounds >= 10000 {
                send.signal(());
                return;
            }

            stats.increment();
        })
        .finish()
        .await?;

    embassy_futures::select::select(session.run(), close.wait()).await;

    Ok(())
}

#[cfg_attr(feature = "std", embassy_executor::main)]
#[cfg_attr(feature = "esp", esp_rtos::main)]
#[cfg_attr(feature = "stm32", embassy_executor::main)]
#[cfg_attr(feature = "wasm", embassy_executor::main)]
async fn main(spawner: embassy_executor::Spawner) {
    if let Err(e) = entry(spawner).await {
        zenoh::error!("Error in main: {}", e);
    }

    zenoh::info!("Exiting main");
}
