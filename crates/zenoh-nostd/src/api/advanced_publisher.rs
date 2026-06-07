use heapless::Vec;

use crate::{api::session::r#pub::Publisher, config::ZSessionConfig};

pub struct CachedSample {
    pub payload: heapless::Vec<u8, 256>,
    pub timestamp: Option<zenoh_proto::fields::Timestamp>,
}

impl CachedSample {
    pub fn new(payload: &[u8], timestamp: Option<zenoh_proto::fields::Timestamp>) -> Option<Self> {
        let mut buf = heapless::Vec::new();
        buf.extend_from_slice(payload).ok()?;
        Some(Self {
            payload: buf,
            timestamp,
        })
    }
}

pub enum HeartbeatMode {
    None,
    Periodic,
    Sporadic,
}

pub struct AdvancedPublisherOptions {
    pub heartbeat: HeartbeatMode,
    pub heartbeat_period_ms: u32,
    pub sample_miss_detection: bool,
}

impl Default for AdvancedPublisherOptions {
    fn default() -> Self {
        Self {
            heartbeat: HeartbeatMode::None,
            heartbeat_period_ms: 0,
            sample_miss_detection: false,
        }
    }
}

pub struct AdvancedPublisher<'a, 'res, Config: ZSessionConfig, const CACHE_SIZE: usize> {
    publisher: Publisher<'a, 'res, Config>,
    cache: Vec<CachedSample, CACHE_SIZE>,
}

impl<'a, 'res, Config: ZSessionConfig, const CACHE_SIZE: usize>
    AdvancedPublisher<'a, 'res, Config, CACHE_SIZE>
{
    pub fn new(publisher: Publisher<'a, 'res, Config>) -> Self {
        Self {
            publisher,
            cache: Vec::new(),
        }
    }

    pub async fn put(
        &mut self,
        payload: &'a [u8],
    ) -> core::result::Result<(), zenoh_proto::SessionError> {
        self.publisher.put(payload).finish().await?;

        if let Some(sample) = CachedSample::new(payload, None) {
            if self.cache.is_full() {
                self.cache.remove(0);
            }
            let _ = self.cache.push(sample);
        }

        Ok(())
    }

    pub async fn delete(
        &mut self,
        timestamp: Option<zenoh_proto::fields::Timestamp>,
        attachment: Option<&'a [u8]>,
    ) -> core::result::Result<(), zenoh_proto::SessionError> {
        self.publisher.delete(timestamp, attachment).await
    }

    pub fn cache(&self) -> &[CachedSample] {
        self.cache.as_slice()
    }

    pub fn keyexpr(&self) -> &zenoh_proto::keyexpr {
        self.publisher.keyexpr()
    }

    pub fn undeclare(self) -> Publisher<'a, 'res, Config> {
        self.publisher
    }

    #[cfg(feature = "publisher_id")]
    pub fn id(&self) -> u32 {
        self.publisher.id()
    }
}
