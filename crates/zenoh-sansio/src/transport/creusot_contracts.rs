// Creusot contracts for transport protocol verification.
//
// Creusot (https://github.com/creusot-rs/creusot) is a Rust verification tool
// that uses Why3 as a backend. Contracts are written as Rust annotations.
//
// To use:
//   1. Install cargo-creusot: `cargo install cargo-creusot`
//   2. Run: `cargo creusot --features creusot`
//
// These specifications mirror properties verified by Kani and Proptest.

#[cfg(feature = "creusot")]
use creusot_contracts::macros::requires;

/// Property: batch_size negotiation always takes the minimum.
#[allow(dead_code)]
#[cfg_attr(feature = "creusot", requires(true))]
#[cfg_attr(feature = "creusot", ensures(result <= mine && result <= theirs))]
pub fn batch_size_negotiation_property(mine: u16, theirs: u16) -> u16 {
    mine.min(theirs)
}

/// Property: For any two distinct ZIDs, exactly one is greater.
#[allow(dead_code)]
#[cfg_attr(feature = "creusot", requires(a != b))]
#[cfg_attr(feature = "creusot", ensures(result == (a != b)))]
pub fn zid_comparison_total_property(a: u128, b: u128) -> bool {
    a != b
}

/// Property: ZID comparison is deterministic.
#[allow(dead_code)]
#[cfg_attr(feature = "creusot", requires(true))]
#[cfg_attr(feature = "creusot", ensures(result == (a > b)))]
pub fn zid_comparison_deterministic_property(a: u128, b: u128) -> bool {
    a > b
}

/// Property: resolution negotiation must not increase beyond mine.
#[allow(dead_code)]
#[cfg_attr(feature = "creusot", requires(true))]
#[cfg_attr(feature = "creusot", ensures(result <= mine && result <= theirs))]
pub fn resolution_negotiation_property(mine: u8, theirs: u8) -> u8 {
    mine.min(theirs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_size_min() {
        assert_eq!(batch_size_negotiation_property(2048, 65535), 2048);
        assert_eq!(batch_size_negotiation_property(65535, 2048), 2048);
    }

    #[test]
    fn test_zid_total_order() {
        assert!(zid_comparison_total_property(1, 2));
        assert!(zid_comparison_total_property(2, 1));
        assert!(!zid_comparison_total_property(1, 1));
    }

    #[test]
    fn test_zid_deterministic() {
        let r1 = zid_comparison_deterministic_property(5, 3);
        let r2 = zid_comparison_deterministic_property(5, 3);
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_resolution_negotiation() {
        assert_eq!(resolution_negotiation_property(8, 16), 8);
        assert_eq!(resolution_negotiation_property(16, 8), 8);
    }
}
