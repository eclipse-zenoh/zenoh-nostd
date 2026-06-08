#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(kani)]
extern crate kani;

#[cfg(feature = "auth")]
#[cfg(not(feature = "std"))]
extern crate alloc;

mod transport;

#[cfg(feature = "verus")]
mod verus_proofs;

pub use transport::*;

#[cfg(test)]
mod tests;
