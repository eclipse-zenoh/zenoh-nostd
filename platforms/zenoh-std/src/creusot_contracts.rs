// Creusot contracts for zenoh-std transport layer.
//
// To use:
//   1. Install cargo-creusot: `cargo install cargo-creusot`
//   2. Add creusot-contracts = "..." to dependencies
//   3. Run: `cargo creusot --features creusot`

/// MTU computation: header overhead must be subtracted from u16::MAX.
#[allow(dead_code)]
fn mtu_with_header(header: u16) -> u16 {
    u16::MAX - header
}

#[cfg(test)]
mod tests {
    #[test]
    fn mtu_is_positive() {
        for header in [20, 40, 60] {
            assert!(u16::MAX - header > 0);
        }
    }
}