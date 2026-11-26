//! Zenoh transport codecs and handshake state machines without an I/O runtime.
//!
//! The caller owns connection setup, I/O callbacks, monotonic time and scheduling.
//! Use [`Transport::builder`] and [`Handshake`] to negotiate a connection, then
//! [`ZTransportTx`] to encode batches and [`ZTransportRx`] to decode them. Keep
//! RX and TX lease bookkeeping current with [`Transport::sync`]. `no_std` is the
//! default; the `std`, `embedded` and logging features follow `zenoh-proto`.
//!
//! Prefer `zenoh-nostd` for application sessions: it drives these codecs, sends
//! keepalives and dispatches operations. Use this crate for custom drivers/codecs.
//! Its flush methods return borrowed codec data and never flush an OS socket.
//!
//! # Local codec example
//!
//! ```
//! use zenoh_sansio::{Transport, ZTransportRx, ZTransportTx};
//! // A local codec is useful for wire tests; this does not handshake with a peer.
//! let mut transport = Transport::builder([0u8; 128]).codec();
//! transport.tx.keepalive();
//! let bytes = transport.tx.flush_raw().unwrap();
//! assert!(!bytes.is_empty());
//! transport.rx.decode_raw(bytes)?;
//! // Keepalives maintain transport state, not application network messages.
//! assert_eq!(transport.rx.flush().count(), 0);
//! # Ok::<(), zenoh_proto::TransportError>(())
//! ```

#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

mod transport;

pub use transport::*;

#[cfg(test)]
mod tests;
