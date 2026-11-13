use super::*;
use crate::scenarios::operation;
use embassy_futures::select::{Either, Either3, select, select3};
use embassy_time::{Duration, with_timeout};
use zenoh_nostd::session::ZSessionConfig;
use zenoh_nostd::session::{Endpoint, Session};

enum CommandError {
    Operation(crate::scenarios::Error),
    Cleanup,
}

async fn next(cli: &Cli) -> Result<Command, i32> {
    if cli.shutdown.get() {
        return Err(0);
    }
    let initial = cli.initial.borrow_mut().take();
    if let Some(line) = initial {
        match frontend::parse_line(&line) {
            Ok(submission) => {
                cli.bytes(&submission.output).await;
                return submission
                    .command
                    .ok_or(if submission.invalid { 2 } else { 0 });
            }
            Err(error) => {
                cli.report(format_args!("error: {error}\r\n")).await;
                return Err(2);
            }
        }
    }
    Ok(cli.commands.receive().await)
}

pub async fn application<C: AppConfig + 'static>(
    config: &'static C,
    cli: &'static Cli,
    default_endpoint: &str,
    can_listen: bool,
) -> i32 {
    loop {
        let cmd = match next(cli).await {
            Ok(cmd) => cmd,
            Err(code) => return code,
        };
        match cmd.kind {
            Kind::Status => {
                cli.status();
                if cli.one_shot.get() {
                    return 0;
                }
                cli.prompt().await;
                continue;
            }
            Kind::Quit => return 0,
            Kind::Close => {
                cli.text("session already closed\r\n").await;
                if cli.one_shot.get() {
                    return 0;
                }
                cli.prompt().await;
                continue;
            }
            _ => {}
        }
        if cmd.listen && !can_listen {
            cli.text("error: listening is unsupported on this platform\r\n")
                .await;
            if cli.one_shot.get() {
                return 2;
            }
            cli.prompt().await;
            continue;
        }
        let address = if cmd.endpoint.is_empty() {
            default_endpoint
        } else {
            cmd.endpoint.as_str()
        };
        if address.is_empty() {
            cli.text("error: set ENDPOINT or use z_open <endpoint>\r\n")
                .await;
            if cli.one_shot.get() {
                return 2;
            }
            cli.prompt().await;
            continue;
        }
        let endpoint = match Endpoint::try_from(address) {
            Ok(endpoint) => endpoint,
            Err(error) => {
                cli.report(format_args!("error: {error}\r\n")).await;
                if cli.one_shot.get() {
                    return 2;
                }
                cli.prompt().await;
                continue;
            }
        };
        cli.reset_command();
        if cmd.kind == Kind::Broker {
            #[cfg(feature = "alloc")]
            {
                if can_listen {
                    let options = cmd.scenario().expect("broker scenario");
                    let run = crate::scenarios::broker(
                        config,
                        endpoint,
                        options.listen1,
                        options.listen2,
                        cli,
                    );
                    let code = match select(cli.cancel.wait(), run).await {
                        Either::First(_) => {
                            cli.text("broker stopped\r\n").await;
                            0
                        }
                        Either::Second(Ok(())) => 0,
                        Either::Second(Err(error)) => {
                            cli.report(format_args!("broker error: {error}\r\n")).await;
                            1
                        }
                    };
                    if cli.one_shot.get() {
                        return code;
                    }
                } else {
                    cli.text("error: broker listening is unsupported on this platform\r\n")
                        .await;
                    if cli.one_shot.get() {
                        return 2;
                    }
                }
            }
            #[cfg(not(feature = "alloc"))]
            {
                cli.text("error: z_broker requires --features alloc\r\n")
                    .await;
                if cli.one_shot.get() {
                    return 2;
                }
            }
            cli.prompt().await;
            continue;
        }
        cli.report(format_args!("opening {address}\r\n")).await;
        // Scope the transport and session to this connection. No reusable static
        // cell or lifetime extension is needed, including after cancellation.
        let open = async {
            if cmd.listen {
                ZSessionConfig::transports(config)
                    .listen(endpoint, ZSessionConfig::buff(config))
                    .await
            } else {
                ZSessionConfig::transports(config)
                    .connect(endpoint, ZSessionConfig::buff(config))
                    .await
            }
        };
        let exit = {
            let mut transport = match select(
                cli.cancel.wait(),
                with_timeout(Duration::from_secs(30), open),
            )
            .await
            {
                Either::First(_) => {
                    cli.text("connection cancelled\r\n").await;
                    if cli.one_shot.get() {
                        return 130;
                    }
                    cli.prompt().await;
                    continue;
                }
                Either::Second(Err(_)) => {
                    cli.text("error: connection timed out\r\n").await;
                    if cli.one_shot.get() {
                        return 1;
                    }
                    cli.prompt().await;
                    continue;
                }
                Either::Second(Ok(Err(error))) => {
                    cli.report(format_args!("connection error: {error}\r\n"))
                        .await;
                    if cli.one_shot.get() {
                        return 1;
                    }
                    cli.prompt().await;
                    continue;
                }
                Either::Second(Ok(Ok(transport))) => transport,
            };
            let session = Session::<C>::new(&mut transport);
            let session = &session;
            cli.connected.set(true);
            let first = if cmd.kind == Kind::Open {
                None
            } else {
                Some(&cmd)
            };
            let commands = async {
                cli.text("session open\r\n").await;
                connected(session, cli, first).await
            };
            // Cancellation owns this whole connection, including the driver. No
            // cancellation/error reporting may await until this scope is dropped.
            select3(cli.cancel.wait(), session.run(), commands).await
        };
        cli.connected.set(false);
        cli.scenario.reset();
        let exit = match exit {
            Either3::First(_) => {
                // Cancellation can win before connected() takes a queued
                // command. Do not start that stale work on a fresh connection.
                cli.commands.clear();
                cli.text("cancelled\r\n").await;
                Some(if cli.shutdown.get() { 0 } else { 130 })
            }
            Either3::Second(Err(error)) => {
                cli.report(format_args!("session error: {error}\r\n")).await;
                Some(1)
            }
            Either3::Second(Ok(())) => Some(0),
            Either3::Third(Err(CommandError::Operation(error))) => {
                cli.report(format_args!("command error: {error}\r\n")).await;
                Some(1)
            }
            Either3::Third(Err(CommandError::Cleanup)) => {
                cli.text("cleanup failed; session closed\r\n").await;
                Some(1)
            }
            Either3::Third(Ok(exit)) => exit,
        };
        if let Some(code) = exit
            && (cli.one_shot.get() || cli.shutdown.get() || code == 0)
        {
            return code;
        }
        cli.text("session closed\r\n").await;
        cli.prompt().await;
    }
}

async fn connected<C: AppConfig>(
    session: &Session<'_, '_, C>,
    cli: &'static Cli,
    first: Option<&Command>,
) -> Result<Option<i32>, CommandError> {
    if let Some(cmd) = first {
        execute(session, cli, cmd).await?;
        if cli.one_shot.get() {
            return Ok(Some(0));
        }
    } else if cli.one_shot.get() {
        return Ok(Some(0));
    }
    cli.prompt().await;
    loop {
        let cmd = match next(cli).await {
            Ok(cmd) => cmd,
            Err(code) => return Ok(Some(code)),
        };
        match cmd.kind {
            Kind::Close => return Ok(None),
            Kind::Quit => return Ok(Some(0)),
            Kind::Open => {
                cli.text("session already open; z_close before changing endpoint\r\n")
                    .await;
            }
            Kind::Broker => {
                cli.text("error: z_close before starting exclusive broker mode\r\n")
                    .await;
            }
            Kind::Status => cli.status(),
            _ => execute(session, cli, &cmd).await?,
        }
        if cli.shutdown.get() {
            return Ok(Some(0));
        }
        cli.prompt().await;
    }
}

async fn execute<C: AppConfig>(
    session: &Session<'_, '_, C>,
    cli: &'static Cli,
    cmd: &Command,
) -> Result<(), CommandError> {
    cli.reset_command();
    let options = cmd.scenario().expect("operation scenario");
    operation(session, &cli.scenario, &options, cli)
        .await
        .map_err(CommandError::Operation)?;
    // Cleanup is only attempted after successful completion. Cancellation drops
    // this entire future, and any operation/cleanup error closes the connection
    // before awaiting diagnostics or allowing another network operation.
    match with_timeout(Duration::from_secs(2), session.clear_declarations()).await {
        Ok(Ok(())) => {}
        _ => return Err(CommandError::Cleanup),
    }
    cli.scenario.reset();
    cli.text("done\r\n").await;
    Ok(())
}
