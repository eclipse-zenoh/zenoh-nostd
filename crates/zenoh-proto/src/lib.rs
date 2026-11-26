//! Borrowed Zenoh wire types, key expressions, endpoints and bounded byte codecs.
//!
//! Application integrations normally use `zenoh-nostd`; use this crate when handling
//! wire messages or implementing a transport. Start with [`keyexpr`] and [`Endpoint`]
//! for validated application inputs, [`msgs`] for message layouts, and [`ZEncode`],
//! [`ZDecode`] and [`ZLen`] for codecs. Decoded payloads borrow the receive buffer.
//!
//! The default is `no_std` without allocation. `std` enables standard-library
//! dependencies; `embedded` selects the custom `getrandom` backend the application
//! must register. Choose at most one logging backend: `log`, `defmt`, `web_console`.
//! Representing a protocol extension here does not implement its session behavior.
//!
//! # Codec round trip
//!
//! ```
//! use zenoh_proto::{ZDecode, ZEncode, ZLen, msgs::KeepAlive};
//! let message = KeepAlive;
//! let mut bytes = [0; 8];
//! let written = message.z_len();
//! message.z_encode(&mut &mut bytes[..])?;
//! assert_eq!(KeepAlive::z_decode(&mut &bytes[..written])?, message);
//! # Ok::<(), zenoh_proto::CodecError>(())
//! ```

#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

pub(crate) use zenoh_derive::*;

mod bytes;
mod codec;
mod endpoint;
mod ke;
mod zerror;

/// Feature-selected logging macros; enable at most one of `log`, `defmt` or `web_console`.
pub mod logging;
/// Borrowed Zenoh wire messages. Codec representation does not imply session-level support for every message or extension.
pub mod msgs;

pub use bytes::*;
pub use codec::*;
pub use endpoint::*;
pub use ke::*;
pub use msgs::{exts, fields};
pub use zerror::*;

#[cfg(test)]
mod tests;

/// Current version of the Zenoh protocol implementation.
pub const VERSION: u8 = 9;
