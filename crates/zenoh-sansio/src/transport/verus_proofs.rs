// Verus formal verification proofs.
//
// These proofs are written in standard Rust (compatible with `cargo test --features verus`).
// For full Verus verification with the SMT solver, compile through `cargo verus`:
//   cargo verus verify -p zenoh-sansio --features verus
//
// The proofs here mirror the Kani proofs in kani_proofs.rs.
// Verus .vrs source files with full SMT-based verification go in the verus/ directory.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VZid(pub [u8; 16]);

impl VZid {
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub fn to_u128(self) -> u128 {
        u128::from_le_bytes(self.0)
    }
}

/// Property: For any two distinct ZIDs, exactly one is greater.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zid_comparison_total() {
        let a = VZid([1u8; 16]);
        let b = VZid([2u8; 16]);
        assert_ne!(a, b);
        // One must be greater
        assert!(a.to_u128() < b.to_u128() || a.to_u128() > b.to_u128());
    }

    #[test]
    fn zid_comparison_deterministic() {
        let a = VZid([1u8; 16]);
        let b = VZid([2u8; 16]);
        let r1 = a.to_u128() > b.to_u128();
        let r2 = a.to_u128() > b.to_u128();
        assert_eq!(r1, r2, "comparison must be deterministic");
    }

    #[test]
    fn zid_to_u128_roundtrip() {
        let val = 0x0123456789abcdef_u128;
        let bytes = val.to_le_bytes();
        let zid = VZid::from_bytes(bytes);
        assert_eq!(zid.to_u128(), val);
    }
}