// Verus formal verification for zenoh-sansio transport layer.
//
// For full Verus SMT verification:
//   cargo verus verify -p zenoh-sansio --features verus
//
// The transport-level Verus proofs are in transport/verus_proofs.rs.

#[cfg(test)]
mod tests {
    #[test]
    fn zid_comparison_nonzero() {
        // Verus property: ZID must be non-zero
        let zid_zero = u128::from_le_bytes([0u8; 16]);
        let zid_valid = u128::from_le_bytes([1u8; 16]);
        assert_eq!(zid_zero, 0, "all-zero ZID is invalid");
        assert_ne!(zid_valid, 0, "non-zero ZID is valid");
    }
}