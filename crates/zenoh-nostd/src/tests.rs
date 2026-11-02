//! In-crate session tests.
//!
//! A `Session` is connected over an in-memory link to a scripted fake router
//! (see [`harness`]). The router performs the real transport handshake, records
//! every network message the session sends, and lets a test inject the messages
//! a Zenoh router would send. No network and no `zenohd` are needed.

mod harness;
mod session;
