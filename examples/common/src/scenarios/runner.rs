#[cfg(feature = "alloc")]
use super::Ready;
use super::{Context, Options, Reporter, Scenario, operation};
use crate::AppConfig;
use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Timer, with_timeout};
use zenoh_nostd::session::{Endpoint, Session, ZSessionConfig};

/// Run one typed startup scenario without a command parser or console.
/// Completion, errors and `Context::request_stop` drop the driver before its
/// transport. `Open` keeps the connection alive until stopped or disconnected.
/// Returns 0 on completion, 1 on operation/transport failure, 2 on invalid or
/// unsupported configuration, and 130 on an application stop request.
pub async fn run<C: AppConfig + 'static, R: Reporter + 'static>(
    config: &'static C,
    context: &'static Context,
    options: &Options<'_>,
    endpoint: &str,
    can_listen: bool,
    reporter: &'static R,
) -> i32 {
    if let Err(error) = options.validate() {
        reporter.emit(format_args!("configuration error: {error}\r\n"));
        return 2;
    }
    if (options.listen || options.scenario == Scenario::Broker) && !can_listen {
        reporter.emit(format_args!(
            "configuration error: listening unsupported on this platform\r\n"
        ));
        return 2;
    }
    #[cfg(not(feature = "alloc"))]
    if options.scenario == Scenario::Broker {
        reporter.emit(format_args!(
            "configuration error: broker requires alloc\r\n"
        ));
        return 2;
    }
    let endpoint = match Endpoint::try_from(endpoint) {
        Ok(value) => value,
        Err(error) => {
            reporter.emit(format_args!("configuration error: {error}\r\n"));
            return 2;
        }
    };
    context.reset();
    let work = async {
        if options.scenario == Scenario::Broker {
            #[cfg(feature = "alloc")]
            return match broker(config, endpoint, options.listen1, options.listen2, reporter).await
            {
                Ok(()) => 0,
                Err(error) => {
                    reporter.emit(format_args!("broker error: {error}\r\n"));
                    1
                }
            };
        }
        reporter.emit(format_args!("opening session\r\n"));
        let connect = async {
            if options.listen {
                ZSessionConfig::transports(config)
                    .listen(endpoint, ZSessionConfig::buff(config))
                    .await
            } else {
                ZSessionConfig::transports(config)
                    .connect(endpoint, ZSessionConfig::buff(config))
                    .await
            }
        };
        let mut transport = match with_timeout(Duration::from_secs(30), connect).await {
            Ok(Ok(value)) => value,
            Ok(Err(error)) => {
                reporter.emit(format_args!("connection error: {error}\r\n"));
                return 1;
            }
            Err(_) => {
                reporter.emit(format_args!("connection error: timed out\r\n"));
                return 1;
            }
        };
        let session = Session::<C>::new(&mut transport);
        let session = &session;
        reporter.emit(format_args!("session open\r\n"));
        let operation = async {
            if options.scenario == Scenario::Open {
                core::future::pending::<()>().await;
            }
            operation(session, context, options, reporter).await?;
            match with_timeout(Duration::from_secs(5), session.flush()).await {
                Ok(result) => {
                    result.map_err(|error| super::Error::Zenoh(error.into()))?;
                    // Flushing drains local buffers, but is not a peer processing
                    // acknowledgement. Allow the final best-effort burst to settle
                    // while RX/keepalives remain active before releasing the link.
                    Timer::after_millis(options.close_delay_ms).await;
                    Ok(())
                }
                Err(_) => Err(super::Error::FlushTimedOut),
            }
        };
        let code = match select(session.run(), operation).await {
            Either::First(Err(error)) => {
                reporter.emit(format_args!("session error: {error}\r\n"));
                1
            }
            Either::Second(Err(error)) => {
                reporter.emit(format_args!("scenario error: {error}\r\n"));
                1
            }
            Either::First(Ok(())) => {
                reporter.emit(format_args!("session ended before scenario completed\r\n"));
                1
            }
            Either::Second(Ok(())) => 0,
        };
        if code == 0 {
            reporter.emit(format_args!("scenario complete\r\n"));
        }
        code
    };
    let code = match select(context.stop.wait(), work).await {
        Either::First(()) => {
            reporter.emit(format_args!("scenario stopped\r\n"));
            130
        }
        Either::Second(code) => code,
    };
    context.finished();
    code
}

#[cfg(feature = "alloc")]
pub(crate) async fn broker<C: AppConfig + 'static, R: Reporter>(
    config: &'static C,
    endpoint: Endpoint<'_>,
    listen1: &str,
    listen2: &str,
    reporter: &R,
) -> Result<(), zenoh_nostd::broker::Error> {
    let broker = zenoh_nostd::broker::Broker::new(config);
    let one = Endpoint::try_from(listen1)?;
    let two = Endpoint::try_from(listen2)?;
    reporter.ready(Ready::Broker).await;
    match embassy_futures::select::select3(
        broker.open(endpoint),
        broker.accept(one),
        broker.accept(two),
    )
    .await
    {
        embassy_futures::select::Either3::First(result)
        | embassy_futures::select::Either3::Second(result)
        | embassy_futures::select::Either3::Third(result) => Ok(result?),
    }
}
