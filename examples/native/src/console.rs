use std::io::{Read, Write};
use zenoh_examples_common::cli::{self, Cli, ConsoleRead, ConsoleWrite, Line};
use zenoh_examples_native::{ENDPOINT, ExampleConfig};

struct Stdin(std::sync::mpsc::Receiver<u8>);
impl ConsoleRead for Stdin {
    async fn read(&mut self, bytes: &mut [u8]) -> Result<usize, ()> {
        loop {
            match self.0.try_recv() {
                Ok(byte) => {
                    bytes[0] = byte;
                    let mut n = 1;
                    while n < bytes.len() {
                        match self.0.try_recv() {
                            Ok(byte) => {
                                bytes[n] = byte;
                                n += 1;
                            }
                            Err(_) => break,
                        }
                    }
                    return Ok(n);
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => return Ok(0),
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    embassy_time::Timer::after_millis(5).await
                }
            }
        }
    }
}
struct Stdout;
impl ConsoleWrite for Stdout {
    async fn write(&mut self, bytes: &[u8]) -> Result<(), ()> {
        let mut out = std::io::stdout().lock();
        out.write_all(bytes)
            .and_then(|_| out.flush())
            .map_err(|_| ())
    }
}

// Interactive Ctrl-C is input for the command runner, rather than process exit.
// Restore terminal settings even if the command engine returns an error.
#[cfg(unix)]
struct Terminal(Option<libc::termios>);
#[cfg(unix)]
impl Terminal {
    fn raw() -> Self {
        unsafe {
            if libc::isatty(libc::STDIN_FILENO) == 0 {
                return Self(None);
            }
            let mut saved = core::mem::MaybeUninit::<libc::termios>::uninit();
            if libc::tcgetattr(libc::STDIN_FILENO, saved.as_mut_ptr()) != 0 {
                return Self(None);
            }
            let saved = saved.assume_init();
            let mut raw = saved;
            libc::cfmakeraw(&mut raw);
            if libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &raw) != 0 {
                return Self(None);
            }
            Self(Some(saved))
        }
    }
}
#[cfg(unix)]
impl Drop for Terminal {
    fn drop(&mut self) {
        if let Some(saved) = self.0 {
            unsafe {
                libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &saved);
            }
        }
    }
}

fn argument_line(args: impl Iterator<Item = String>) -> Result<Line, &'static str> {
    let mut line = Line::new();
    for arg in args {
        if !line.is_empty() {
            line.push(' ').map_err(|_| "command exceeds 512 bytes")?;
        }
        line.push('"').map_err(|_| "command exceeds 512 bytes")?;
        for ch in arg.chars() {
            if matches!(ch, '\\' | '"') {
                line.push('\\').map_err(|_| "command exceeds 512 bytes")?;
            }
            line.push(ch).map_err(|_| "command exceeds 512 bytes")?;
        }
        line.push('"').map_err(|_| "command exceeds 512 bytes")?;
    }
    Ok(line)
}

#[embassy_executor::task]
async fn application() {
    let code = run().await;
    std::process::exit(code);
}

fn main() {
    let executor = Box::leak(Box::new(embassy_executor::Executor::new()));
    executor.run(|spawner| spawner.must_spawn(application()));
}

async fn run() -> i32 {
    let _ = env_logger::try_init();
    let cli = Box::leak(Box::new(Cli::new()));
    let config = Box::leak(Box::new(ExampleConfig::new(zenoh_std::StdLinkManager)));
    let endpoint = std::env::var("ENDPOINT").unwrap_or_else(|_| ENDPOINT.into());
    let line = match argument_line(std::env::args().skip(1)) {
        Ok(line) => line,
        Err(error) => {
            eprintln!("{error}");
            return 2;
        }
    };
    if !line.is_empty() {
        cli.set_one_shot(line).unwrap();
        cli::run(
            cli,
            cli::PendingRead,
            Stdout,
            async { &*config },
            &endpoint,
            true,
        )
        .await
    } else {
        #[cfg(unix)]
        let _terminal = Terminal::raw();
        let (tx, rx) = std::sync::mpsc::sync_channel(1024);
        std::thread::spawn(move || {
            let mut input = std::io::stdin().lock();
            let mut buffer = [0u8; 64];
            while let Ok(n) = input.read(&mut buffer) {
                if n == 0 {
                    break;
                }
                for &byte in &buffer[..n] {
                    if tx.send(byte).is_err() {
                        return;
                    }
                }
            }
        });
        cli::run(cli, Stdin(rx), Stdout, async { &*config }, &endpoint, true).await
    }
}
