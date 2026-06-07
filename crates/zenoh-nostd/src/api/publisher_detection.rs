use zenoh_proto::keyexpr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublisherDetectionError {
    Full,
}

pub const DEFAULT_MAX_KEYEXPR: usize = 255;

#[derive(Debug, Clone)]
pub struct PublisherInfo<const MAX_KEYEXPR: usize = DEFAULT_MAX_KEYEXPR> {
    pub zid: [u8; 16],
    pub keyexpr: heapless::String<MAX_KEYEXPR>,
    pub last_seen: Option<u64>,
}

impl<const MAX_KEYEXPR: usize> PublisherInfo<MAX_KEYEXPR> {
    pub fn new(zid: [u8; 16], keyexpr_str: &str) -> Option<Self> {
        let mut ke = heapless::String::new();
        ke.push_str(keyexpr_str).ok()?;
        Some(Self {
            zid,
            keyexpr: ke,
            last_seen: None,
        })
    }

    pub fn from_keyexpr(zid: [u8; 16], ke: &keyexpr) -> Option<Self> {
        Self::new(zid, ke.as_str())
    }
}

pub struct PublisherDetector<
    const MAX_PUBLISHERS: usize = 16,
    const MAX_KEYEXPR: usize = DEFAULT_MAX_KEYEXPR,
> {
    active_publishers: heapless::Vec<PublisherInfo<MAX_KEYEXPR>, MAX_PUBLISHERS>,
    keyexpr: heapless::String<MAX_KEYEXPR>,
}

impl<const MAX_PUBLISHERS: usize, const MAX_KEYEXPR: usize>
    PublisherDetector<MAX_PUBLISHERS, MAX_KEYEXPR>
{
    pub fn new(ke: &keyexpr) -> Option<Self> {
        let mut s = heapless::String::new();
        s.push_str(ke.as_str()).ok()?;
        Some(Self {
            active_publishers: heapless::Vec::new(),
            keyexpr: s,
        })
    }

    /// Create a `PublisherDetector` from a key expression string slice.
    /// Returns `None` if the string exceeds `MAX_KEYEXPR` capacity.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(ke_str: &str) -> Option<Self> {
        let mut s = heapless::String::new();
        s.push_str(ke_str).ok()?;
        Some(Self {
            active_publishers: heapless::Vec::new(),
            keyexpr: s,
        })
    }

    pub fn register(
        &mut self,
        info: PublisherInfo<MAX_KEYEXPR>,
    ) -> Result<(), PublisherDetectionError> {
        if self.active_publishers.iter().any(|p| p.zid == info.zid) {
            return Ok(());
        }
        self.active_publishers
            .push(info)
            .map_err(|_| PublisherDetectionError::Full)
    }

    pub fn unregister(&mut self, zid: &[u8; 16]) {
        self.active_publishers.retain(|p| p.zid != *zid);
    }

    pub fn is_active(&self, zid: &[u8; 16]) -> bool {
        self.active_publishers.iter().any(|p| p.zid == *zid)
    }

    pub fn active_publishers(&self) -> &[PublisherInfo<MAX_KEYEXPR>] {
        self.active_publishers.as_slice()
    }

    pub fn publisher_count(&self) -> usize {
        self.active_publishers.len()
    }

    pub fn keyexpr(&self) -> &str {
        self.keyexpr.as_str()
    }

    pub fn clear(&mut self) {
        self.active_publishers.clear();
    }

    pub fn register_zid(&mut self, zid: [u8; 16]) -> Result<(), PublisherDetectionError> {
        let info = PublisherInfo {
            zid,
            keyexpr: self.keyexpr.clone(),
            last_seen: None,
        };
        self.register(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_keyexpr() -> &'static keyexpr {
        keyexpr::new("test/detection").unwrap()
    }

    #[test]
    fn test_new_detector() {
        let d = PublisherDetector::<4, 64>::new(test_keyexpr());
        assert!(d.is_some());
        assert_eq!(d.unwrap().publisher_count(), 0);
    }

    #[test]
    fn test_register() {
        let mut d = PublisherDetector::<4, 64>::new(test_keyexpr()).unwrap();
        let info = PublisherInfo::new([1u8; 16], "test/detection").unwrap();
        assert!(d.register(info).is_ok());
        assert_eq!(d.publisher_count(), 1);
    }

    #[test]
    fn test_register_duplicate_zid_is_noop() {
        let mut d = PublisherDetector::<4, 64>::new(test_keyexpr()).unwrap();
        let info1 = PublisherInfo::new([1u8; 16], "test/detection").unwrap();
        let info2 = PublisherInfo::new([1u8; 16], "other/expr").unwrap();
        assert!(d.register(info1).is_ok());
        assert!(d.register(info2).is_ok());
        assert_eq!(d.publisher_count(), 1);
    }

    #[test]
    fn test_register_full() {
        let mut d = PublisherDetector::<2, 64>::new(test_keyexpr()).unwrap();
        assert!(
            d.register(PublisherInfo::new([1u8; 16], "a").unwrap())
                .is_ok()
        );
        assert!(
            d.register(PublisherInfo::new([2u8; 16], "b").unwrap())
                .is_ok()
        );
        assert!(
            d.register(PublisherInfo::new([3u8; 16], "c").unwrap())
                .is_err()
        );
    }

    #[test]
    fn test_unregister() {
        let mut d = PublisherDetector::<4, 64>::new(test_keyexpr()).unwrap();
        let zid = [1u8; 16];
        d.register(PublisherInfo::new(zid, "test/detection").unwrap())
            .unwrap();
        assert_eq!(d.publisher_count(), 1);
        d.unregister(&zid);
        assert_eq!(d.publisher_count(), 0);
    }

    #[test]
    fn test_unregister_nonexistent_is_noop() {
        let mut d = PublisherDetector::<4, 64>::new(test_keyexpr()).unwrap();
        d.register(PublisherInfo::new([1u8; 16], "test/detection").unwrap())
            .unwrap();
        d.unregister(&[2u8; 16]);
        assert_eq!(d.publisher_count(), 1);
    }

    #[test]
    fn test_is_active() {
        let mut d = PublisherDetector::<4, 64>::new(test_keyexpr()).unwrap();
        let zid = [42u8; 16];
        assert!(!d.is_active(&zid));
        d.register(PublisherInfo::new(zid, "test/detection").unwrap())
            .unwrap();
        assert!(d.is_active(&zid));
    }

    #[test]
    fn test_register_zid() {
        let mut d = PublisherDetector::<4, 64>::new(test_keyexpr()).unwrap();
        d.register_zid([7u8; 16]).unwrap();
        assert_eq!(d.publisher_count(), 1);
        assert!(d.is_active(&[7u8; 16]));
        assert_eq!(d.active_publishers()[0].keyexpr.as_str(), "test/detection");
    }

    #[test]
    fn test_active_publishers_slice() {
        let mut d = PublisherDetector::<4, 64>::new(test_keyexpr()).unwrap();
        d.register(PublisherInfo::new([1u8; 16], "a").unwrap())
            .unwrap();
        d.register(PublisherInfo::new([2u8; 16], "b").unwrap())
            .unwrap();
        let slice = d.active_publishers();
        assert_eq!(slice.len(), 2);
    }

    #[test]
    fn test_from_str() {
        let d = PublisherDetector::<4, 64>::from_str("my/ke/expr").unwrap();
        assert_eq!(d.keyexpr(), "my/ke/expr");
    }

    #[test]
    fn test_clear() {
        let mut d = PublisherDetector::<4, 64>::new(test_keyexpr()).unwrap();
        d.register(PublisherInfo::new([1u8; 16], "a").unwrap())
            .unwrap();
        d.register(PublisherInfo::new([2u8; 16], "b").unwrap())
            .unwrap();
        assert_eq!(d.publisher_count(), 2);
        d.clear();
        assert_eq!(d.publisher_count(), 0);
    }

    #[test]
    fn test_info_from_keyexpr() {
        let ke = test_keyexpr();
        let info = PublisherInfo::<64>::from_keyexpr([1u8; 16], ke);
        assert!(info.is_some());
        assert_eq!(info.unwrap().keyexpr.as_str(), "test/detection");
    }
}
