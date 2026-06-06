use embassy_time::Duration;
use zenoh_proto::SessionError;

use crate::config::ZSessionConfig;

pub struct AutoReconnectConfig {
    pub max_retries: u32,
    pub retry_delay: Duration,
}

impl Default for AutoReconnectConfig {
    fn default() -> Self {
        Self { max_retries: 10, retry_delay: Duration::from_secs(1) }
    }
}

/// Run with auto-retry: retries session.run() on transient failures.
/// For full reconnect (new transport), reconstruct the session externally.
pub async fn run_with_retry<Config: ZSessionConfig>(
    session: &crate::api::session::Session<'_, Config>,
    reconnect: &AutoReconnectConfig,
) -> core::result::Result<(), SessionError>
{
    let mut attempt = 0;
    loop {
        match session.run().await {
            Ok(()) => return Ok(()),
            Err(_) if attempt < reconnect.max_retries => {
                attempt += 1;
                embassy_time::Timer::after(reconnect.retry_delay).await;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}
