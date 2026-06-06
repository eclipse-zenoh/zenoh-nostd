use heapless::Vec;
use zenoh_proto::{SessionError, keyexpr};

use crate::{
    api::sample::{FixedCapacitySample, Sample, SampleKind},
    config::ZSessionConfig,
};

pub struct AdvancedSubscriberOptions {
    pub max_samples: usize,
    pub max_age_ms: u64,
    pub detect_late_publishers: bool,
}

impl Default for AdvancedSubscriberOptions {
    fn default() -> Self {
        Self {
            max_samples: 0,
            max_age_ms: 0,
            detect_late_publishers: false,
        }
    }
}

pub struct AdvancedSubscriber<
    'a,
    'res,
    Config: ZSessionConfig,
    const HISTORY_SIZE: usize,
    const MAX_KEYEXPR: usize,
    const MAX_PAYLOAD: usize,
    const MAX_ATTACHMENT: usize,
> {
    id: u32,
    ke: &'static keyexpr,
    #[allow(dead_code)]
    session: &'a crate::session::Session<'res, Config>,
    history: Vec<
        FixedCapacitySample<MAX_KEYEXPR, MAX_PAYLOAD, MAX_ATTACHMENT>,
        HISTORY_SIZE,
    >,
}

impl<
        'a,
        'res,
        Config: ZSessionConfig,
        const HISTORY_SIZE: usize,
        const MAX_KEYEXPR: usize,
        const MAX_PAYLOAD: usize,
        const MAX_ATTACHMENT: usize,
    >
    AdvancedSubscriber<'a, 'res, Config, HISTORY_SIZE, MAX_KEYEXPR, MAX_PAYLOAD, MAX_ATTACHMENT>
{
    pub fn record(&mut self, sample: &Sample<'_>) -> Result<(), SessionError> {
        if sample.kind() != SampleKind::Put {
            return Ok(());
        }
        let owned = FixedCapacitySample::<MAX_KEYEXPR, MAX_PAYLOAD, MAX_ATTACHMENT>::try_from(
            sample,
        )
        .map_err(|_| SessionError::RequestTimedout)?;
        if self.history.is_full() {
            self.history.remove(0);
        }
        self.history
            .push(owned)
            .map_err(|_| SessionError::RequestTimedout)?;
        Ok(())
    }

    pub fn history(
        &self,
    ) -> &[FixedCapacitySample<MAX_KEYEXPR, MAX_PAYLOAD, MAX_ATTACHMENT>] {
        self.history.as_slice()
    }

    pub fn keyexpr(&self) -> &keyexpr {
        self.ke
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}
