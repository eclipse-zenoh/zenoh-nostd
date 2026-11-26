//! Typed example operations shared by CLI and unattended applications.
mod config;
mod context;
mod runner;
pub use config::{
    KEY_CAPACITY, Options, PAYLOAD_CAPACITY, SETTING_NAMES, Scenario, compiled_setting, configured,
};
use context::Event;
pub use context::{Context, Ready, ReportFn, Reporter};
use embassy_futures::yield_now;
use embassy_time::{Duration, Instant, Timer, with_timeout};
use heapless::{String, Vec};
#[cfg(all(feature = "alloc", feature = "cli"))]
pub(crate) use runner::broker;
pub use runner::run;
use zenoh_nostd::session::ZSessionConfig;
use zenoh_nostd::session::{GetResponse, Session, zenoh};

#[derive(Debug)]
pub enum Error {
    InvalidOptions(&'static str),
    FlushTimedOut,
    Zenoh(zenoh_nostd::session::Error),
}
impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidOptions(message) => f.write_str(message),
            Self::FlushTimedOut => f.write_str("final transmit flush timed out"),
            Self::Zenoh(error) => core::fmt::Display::fmt(error, f),
        }
    }
}
/// Run an operation on a session whose driver is polled by the application.
/// Cancelling this future or encountering a transport write error requires
/// dropping the session and transport before further network I/O. `run`
/// manages that connection scope for you.
pub async fn operation<C: ZSessionConfig, R: Reporter>(
    session: &Session<'_, '_, C>,
    context: &'static Context,
    options: &Options<'_>,
    reporter: &R,
) -> Result<(), Error> {
    options.validate().map_err(Error::InvalidOptions)?;
    if matches!(options.scenario, Scenario::Open | Scenario::Broker) {
        return Err(Error::InvalidOptions(
            "use scenarios::run for open or broker",
        ));
    }
    operation_inner(session, context, options, reporter)
        .await
        .map_err(Error::Zenoh)
}

async fn operation_inner<C: ZSessionConfig, R: Reporter>(
    session: &Session<'_, '_, C>,
    context: &'static Context,
    cmd: &Options<'_>,
    reporter: &R,
) -> zenoh::ZResult<()> {
    let key = zenoh::keyexpr::new(cmd.key)?;
    match cmd.scenario {
        Scenario::Put => {
            session.put(key, cmd.payload).finish().await?;
        }
        Scenario::Pub | Scenario::PubThr => {
            let publisher = session.declare_publisher(key).finish().await?;
            let bytes: [u8; PAYLOAD_CAPACITY] = core::array::from_fn(|i| (i % 10) as u8);
            let payload = if cmd.scenario == Scenario::PubThr {
                &bytes[..cmd.size]
            } else {
                cmd.payload
            };
            let start = Instant::now();
            let mut report = start;
            let mut count = 0u64;
            loop {
                publisher.put(payload).finish().await?;
                count += 1;
                if cmd.scenario == Scenario::Pub {
                    reporter.emit(format_args!(
                        "put {} ({} bytes)\r\n",
                        cmd.key,
                        payload.len()
                    ));
                }
                if report.elapsed() >= Duration::from_secs(1) {
                    reporter.emit(format_args!(
                        "published={} bytes={} elapsed_us={}\r\n",
                        count,
                        count * payload.len() as u64,
                        start.elapsed().as_micros()
                    ));
                    report = Instant::now();
                }
                if cmd.count.is_some_and(|n| count >= n) {
                    break;
                }
                if cmd.scenario == Scenario::PubThr {
                    yield_now().await;
                } else {
                    Timer::after_millis(cmd.interval_ms).await;
                }
            }
            reporter.emit(format_args!(
                "published={} elapsed_us={}\r\n",
                count,
                start.elapsed().as_micros()
            ));
            publisher.undeclare().await?;
        }
        Scenario::Get | Scenario::Querier => {
            let querier = session
                .declare_querier(key)
                .parameters(cmd.parameters)
                .timeout(core::time::Duration::from_millis(cmd.timeout_ms))
                .finish()
                .await?;
            let mut count = 0u64;
            loop {
                let replies = querier
                    .get()
                    .callback_sync(move |response| match response {
                        GetResponse::Ok(sample) => context.reply(false, sample),
                        GetResponse::Err(sample) => context.reply(true, sample),
                    })
                    .finish()
                    .await?;
                while !replies.is_complete().await {
                    drain_replies(context, reporter);
                    Timer::after_millis(10).await;
                }
                drain_replies(context, reporter);
                replies.cancel().await?;
                count += 1;
                if cmd.scenario == Scenario::Get || cmd.count.is_some_and(|n| count >= n) {
                    break;
                }
                Timer::after_millis(cmd.interval_ms).await;
            }
            querier.undeclare().await?;
        }
        Scenario::Sub | Scenario::SubThr | Scenario::Ping | Scenario::Pong => {
            context.aggregate.set(cmd.scenario == Scenario::SubThr);
            let subscriber_key = if cmd.scenario == Scenario::Ping {
                zenoh::keyexpr::new(cmd.peer_key)?
            } else {
                key
            };
            let subscriber = session
                .declare_subscriber(subscriber_key)
                .callback_sync(move |sample| context.sample(sample))
                .finish()
                .await?;
            reporter.ready(Ready::Subscriber).await;
            let start = Instant::now();
            let mut report = start;
            let mut count = 0u64;
            let bytes: [u8; PAYLOAD_CAPACITY] = core::array::from_fn(|i| (i % 10) as u8);
            loop {
                if cmd.scenario == Scenario::SubThr {
                    Timer::after_millis(100).await;
                    if report.elapsed() >= Duration::from_secs(1)
                        || cmd.count.is_some_and(|n| context.received.get() >= n)
                    {
                        reporter.emit(format_args!(
                            "received={} bytes={} elapsed_us={}\r\n",
                            context.received.get(),
                            context.received_bytes.get(),
                            start.elapsed().as_micros()
                        ));
                        report = Instant::now();
                    }
                    if cmd.count.is_some_and(|n| context.received.get() >= n) {
                        break;
                    }
                    continue;
                }
                let ping_start = Instant::now();
                if cmd.scenario == Scenario::Ping {
                    session.put(key, &bytes[..cmd.size]).finish().await?;
                }
                let event = if cmd.scenario == Scenario::Ping {
                    with_timeout(
                        Duration::from_millis(cmd.timeout_ms),
                        context.events.receive(),
                    )
                    .await
                    .map_err(|_| {
                        zenoh_nostd::session::Error::from(
                            zenoh_nostd::session::SessionError::RequestTimedout,
                        )
                    })?
                } else {
                    context.events.receive().await
                };
                count += 1;
                if cmd.scenario == Scenario::Pong {
                    session
                        .put(zenoh::keyexpr::new(cmd.peer_key)?, &event.payload)
                        .finish()
                        .await?;
                } else if cmd.scenario == Scenario::Ping {
                    reporter.emit(format_args!(
                        "ping seq={} bytes={} rtt_us={}\r\n",
                        count,
                        cmd.size,
                        ping_start.elapsed().as_micros()
                    ));
                } else {
                    reporter.emit(format_args!(
                        "sample {} {:?}\r\n",
                        event.key,
                        core::str::from_utf8(&event.payload).unwrap_or("<binary>")
                    ));
                }
                if cmd.count.is_some_and(|n| count >= n) {
                    break;
                }
            }
            subscriber.undeclare().await?;
        }
        Scenario::Queryable => {
            let queryable = session
                .declare_queryable(key)
                .callback_sync(move |query| {
                    if let Ok(key) = String::try_from(query.keyexpr().as_str()) {
                        if context
                            .events
                            .try_send(Event {
                                id: query.request_id(),
                                key,
                                payload: Vec::new(),
                            })
                            .is_err()
                        {
                            context.drop_event();
                            query.reject();
                        }
                    } else {
                        context.drop_event();
                        query.reject();
                    }
                })
                .finish()
                .await?;
            reporter.ready(Ready::Queryable).await;
            let mut count = 0u64;
            loop {
                let event = context.events.receive().await;
                session
                    .reply(event.id, zenoh::keyexpr::new(&event.key)?, cmd.payload)
                    .await?;
                session.finalize(event.id).await?;
                reporter.emit(format_args!("answered {}\r\n", event.key));
                count += 1;
                if context.is_stopping() || cmd.count.is_some_and(|n| count >= n) {
                    break;
                }
            }
            queryable.undeclare().await?;
        }
        _ => {}
    }
    Ok(())
}

fn drain_replies(context: &Context, reporter: &impl Reporter) {
    while let Ok(event) = context.events.try_receive() {
        let label = if event.id == 0 {
            "reply"
        } else {
            "reply-error"
        };
        reporter.emit(format_args!(
            "{label} {} {:?}\r\n",
            event.key,
            core::str::from_utf8(&event.payload).unwrap_or("<binary>")
        ));
    }
}
