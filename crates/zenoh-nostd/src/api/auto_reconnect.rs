use embassy_time::{Duration, Timer};
use zenoh_proto::SessionError;

use crate::{api::session::Session, config::ZSessionConfig};

pub struct AutoReconnectConfig {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub jitter: bool,
}

impl Default for AutoReconnectConfig {
    fn default() -> Self {
        Self {
            max_retries: 10,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            jitter: true,
        }
    }
}

pub fn is_retryable(err: &SessionError) -> bool {
    use SessionError::*;
    matches!(
        err,
        TransportClosed | OpenTimeout | RxLeaseExpired | TxLeaseExpired
            | CouldNotConnect | LinkTxFailed | LinkRxFailed | CouldNotListen
            | CouldNotGetAddrInfo
    )
}

fn jitter_ms(attempt: u32, range: u64) -> u64 {
    let x = attempt as u64;
    let val = (1103515245u64.wrapping_mul(x).wrapping_add(12345)) % 1_000_000;
    range * val / 1_000_000
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
pub async fn run_with_retry<'res, Config, F, Fut>(
    mut factory: F,
    reconnect: &AutoReconnectConfig,
) -> Result<(), SessionError>
where
    Config: ZSessionConfig + 'res,
    F: FnMut() -> Fut,
    Fut: core::future::Future<Output = Result<Session<'res, Config>, SessionError>>,
{
    let mut attempt = 0u32;

    loop {
        let session = factory().await?;

        match session.run().await {
            Ok(()) => return Ok(()),
            Err(e) if is_retryable(&e) && attempt < reconnect.max_retries => {
                attempt += 1;

                let exponent = (attempt - 1).min(16);
                let delay_ms = reconnect.base_delay.as_millis();
                let backoff_ms = delay_ms.saturating_mul(1u64 << exponent);
                let max_ms = reconnect.max_delay.as_millis();
                let mut delay_ms = backoff_ms.min(max_ms);

                if reconnect.jitter && delay_ms > 0 {
                    let half = delay_ms / 4;
                    let offset = jitter_ms(attempt, half * 2 + 1);
                    delay_ms = delay_ms + offset - half;
                }

                Timer::after(Duration::from_millis(delay_ms)).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}