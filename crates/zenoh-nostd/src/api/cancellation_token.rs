use core::cell::Cell;

#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: Cell<bool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Cell::new(false),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.set(true);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.get()
    }

    pub fn reset(&self) {
        self.cancelled.set(false);
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_starts_not_cancelled() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn test_cancel_sets_cancelled() {
        let token = CancellationToken::new();
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_is_cancelled_after_cancel() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
        token.cancel();
        assert!(token.is_cancelled());
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_reset_clears_cancelled() {
        let token = CancellationToken::new();
        token.cancel();
        assert!(token.is_cancelled());
        token.reset();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn test_clone() {
        let token = CancellationToken::new();
        let clone = token.clone();
        clone.cancel();
        assert!(clone.is_cancelled());
        assert!(!token.is_cancelled());
    }

    #[test]
    fn test_default_is_not_cancelled() {
        let token = CancellationToken::default();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn test_multiple_cancel_idempotent() {
        let token = CancellationToken::new();
        token.cancel();
        token.cancel();
        token.cancel();
        assert!(token.is_cancelled());
    }
}
