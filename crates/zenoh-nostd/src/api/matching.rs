#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchingStatus {
    pub matching_subscribers: bool,
    pub matching_queriers: bool,
}

impl MatchingStatus {
    pub const fn no_match() -> Self {
        Self {
            matching_subscribers: false,
            matching_queriers: false,
        }
    }

    pub fn has_any(&self) -> bool {
        self.matching_subscribers || self.matching_queriers
    }
}

impl Default for MatchingStatus {
    fn default() -> Self {
        Self::no_match()
    }
}

#[cfg(feature = "matching")]
pub trait MatchingListener {
    fn on_matching_changed(&self, status: &MatchingStatus);
}

#[cfg(feature = "matching")]
impl MatchingListener for fn(&MatchingStatus) {
    fn on_matching_changed(&self, status: &MatchingStatus) {
        (self)(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_no_match() {
        let status = MatchingStatus::default();
        assert!(!status.matching_subscribers);
        assert!(!status.matching_queriers);
        assert!(!status.has_any());
    }

    #[test]
    fn test_no_match_const() {
        let status = MatchingStatus::no_match();
        assert!(!status.matching_subscribers);
        assert!(!status.matching_queriers);
    }

    #[test]
    fn test_has_any_subscribers() {
        let status = MatchingStatus {
            matching_subscribers: true,
            matching_queriers: false,
        };
        assert!(status.has_any());
    }

    #[test]
    fn test_has_any_queriers() {
        let status = MatchingStatus {
            matching_subscribers: false,
            matching_queriers: true,
        };
        assert!(status.has_any());
    }

    #[test]
    fn test_clone_and_eq() {
        let a = MatchingStatus {
            matching_subscribers: true,
            matching_queriers: false,
        };
        let b = a;
        assert_eq!(a, b);
    }

    #[cfg(feature = "matching")]
    #[test]
    fn test_fn_pointer_listener() {
        use super::MatchingListener;
        static mut FIRED: bool = false;
        fn my_listener(status: &MatchingStatus) {
            unsafe {
                FIRED = status.matching_subscribers;
            }
        }
        let listener: fn(&MatchingStatus) = my_listener;
        let status = MatchingStatus {
            matching_subscribers: true,
            matching_queriers: false,
        };
        listener.on_matching_changed(&status);
        unsafe {
            assert!(FIRED);
        }
    }
}
