use zenoh_proto::SessionError;

use crate::{api::session::Session, config::ZSessionConfig};

pub struct AutoReconnectConfig {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub jitter: bool,
}

impl Default for AutoReconnectConfig {
    fn default() -> Self {
        Self {
            max_retries: 10,
            base_delay_ms: 100,
            max_delay_ms: 10_000,
            jitter: true,
        }
    }
}

pub fn is_retryable(err: &SessionError) -> bool {
    use SessionError::*;
    matches!(
        err,
        TransportClosed
            | OpenTimeout
            | RxLeaseExpired
            | TxLeaseExpired
            | CouldNotConnect
            | LinkTxFailed
            | LinkRxFailed
            | CouldNotListen
            | CouldNotGetAddrInfo
    )
}

fn jitter_ms(attempt: u32, range: u64) -> u64 {
    let x = attempt as u64;
    let val = (1103515245u64.wrapping_mul(x).wrapping_add(12345)) % 1_000_000;
    range * val / 1_000_000
}

/// Busy-wait delay using a simple loop.
/// On embedded systems this would typically yield to a scheduler.
fn delay_ms(ms: u64) {
    // Approximate busy-wait: each iteration is ~1 cycle
    // Actual timing depends on CPU speed; on real HW use a timer peripheral.
    let iterations = ms * 10_000;
    for _ in 0..iterations {
        core::hint::spin_loop();
    }
}

/// Run with automatic reconnect using a session factory.
///
/// The `factory` closure is called each time a new session is needed (initial
/// creation and after every retryable transport failure). Inside the factory,
/// the caller creates a new transport, builds a `Session`, and re-declares
/// all active subscriptions, queryables, etc.
///
/// On retryable errors (transport closed, lease expired, link failure, …),
/// the factory is called again with exponential backoff. Permanent errors
/// (protocol violations, invalid state, …) are propagated immediately.
pub fn run_with_retry<'res, Config, F>(
    mut factory: F,
    reconnect: &AutoReconnectConfig,
) -> Result<(), SessionError>
where
    Config: ZSessionConfig + 'res,
    F: FnMut() -> Result<Session<'res, Config>, SessionError>,
{
    let mut attempt = 0u32;

    loop {
        let session = factory()?;

        match session.run() {
            Ok(()) => return Ok(()),
            Err(e) if is_retryable(&e) && attempt < reconnect.max_retries => {
                attempt += 1;

                let exponent = (attempt - 1).min(16);
                let delay_ms_base = reconnect.base_delay_ms;
                let backoff_ms = delay_ms_base.saturating_mul(1u64 << exponent);
                let mut delay_ms_val = backoff_ms.min(reconnect.max_delay_ms);

                if reconnect.jitter && delay_ms_val > 0 {
                    let half = delay_ms_val / 4;
                    let offset = jitter_ms(attempt, half * 2 + 1);
                    delay_ms_val = delay_ms_val + offset - half;
                }

                delay_ms(delay_ms_val);
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}
