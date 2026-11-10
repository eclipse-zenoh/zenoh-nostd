//! Async Zenoh sessions for applications that supply their own platform integration.
//!
//! The default configuration is `no_std` with fixed-capacity storage. The library
//! selects no MCU, executor, network interface, console, or allocator. Start with
//! [`session::ZSessionConfig`] for memory policy and [`platform::ZLinkManager`] for
//! networking; platform adapter crates provide ready-made implementations.
//!
//! # Application requirements
//!
//! Provide an Embassy-compatible time driver and keep [`session::Session::run`]
//! polled alongside application operations. Keep a session in one executor/thread
//! context: its synchronization uses `NoopRawMutex`, not a cross-thread mutex.
//! Bare-metal applications also provide entropy through the custom `getrandom`
//! backend enabled by `embedded`, plus their panic and critical-section handlers.
//!
//! # Features
//!
//! | Feature | Effect |
//! | --- | --- |
//! | default (empty) | Fixed-capacity storage; no standard library required |
//! | `alloc` | Heap-backed storage alternatives and `broker`; requires an allocator, does not enable `std` |
//! | `std` | Standard-library support in protocol dependencies |
//! | `embedded` | Custom randomness backend integration; the application registers the backend |
//! | `log`, `defmt`, `web_console` | Logging backends; `web_console` also enables `std` |
//!
//! Select a logging backend at the application level. CLI and board-selection
//! features belong to applications, not this crate.
//!
//! # A scoped connection
//!
//! Given an initialized configuration and reachable endpoint, this function opens
//! a connection and publishes once. It polls the driver until either the operation
//! finishes or the driver fails, then drops the session before its transport.
//! Network/HAL setup and executor startup remain application responsibilities.
//!
//! ```no_run
//! use embassy_futures::select::{select, Either};
//! use zenoh_nostd::session::{Endpoint, Session, SessionError, ZSessionConfig};
//! use zenoh_nostd::session::zenoh::keyexpr;
//!
//! async fn publish_once<C: ZSessionConfig>(
//!     config: &C,
//!     endpoint: Endpoint<'_>,
//! ) -> Result<(), SessionError> {
//!     let mut transport = config.transports().connect(endpoint, config.buff()).await?;
//!     let session = Session::<C>::new(&mut transport);
//!     let operation = async {
//!         let key = keyexpr::new("demo/example")?;
//!         session.put(key, b"hello").finish().await?;
//!         session.flush().await
//!     };
//!     match select(session.run(), operation).await {
//!         Either::First(result) => result,
//!         Either::Second(result) => result,
//!     }
//! }
//! ```
//!
//! A successful flush drains adapter-owned TX buffers; it is not a remote delivery
//! acknowledgment. Cancellation or transport write errors require discarding the
//! whole connection before further network I/O. Reconnect with a new transport;
//! do not restart a cancelled driver on the existing connection.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod api;

mod config;
mod io;
mod resources;

/// Sessions, application configuration, and pub/sub/query operation builders.
pub mod session {
    pub use super::config::ZSessionConfig;
    pub use super::io::transport::TransportLinkManager;
    pub use super::resources::Resources;
    #[doc(hidden)]
    pub use static_cell::StaticCell as __StaticCell;
    pub use zenoh_proto::{Endpoint, Error, SessionError};

    pub use super::api::{
        query::*,
        response::*,
        sample::*,
        session::{Session, SessionBuilder, StaticSessionBuilder},
        session::{get::*, r#pub::*, put::*, querier::*, queryable::*, sub::*},
    };

    pub mod zenoh {
        pub use super::super::api::callbacks::storage;

        pub use crate::__session as session;

        pub use zenoh_proto::{debug, error, info, keyexpr, trace, warn, zbail};

        pub type ZResult<T> = core::result::Result<T, super::Error>;
    }
}

#[cfg(feature = "alloc")]
/// Heap-backed broker support, available with the `alloc` feature.
pub mod broker {
    pub use super::config::ZBrokerConfig;
    pub use super::io::transport::TransportLinkManager;
    pub use zenoh_proto::{Endpoint, Error};

    pub use super::api::broker::Broker;

    pub mod zenoh {
        pub use zenoh_proto::{debug, error, info, keyexpr, trace, warn, zbail};
        pub type ZResult<T> = core::result::Result<T, super::Error>;

        pub use crate::__broker as broker;
    }
}

/// Link traits and embedded-I/O adapters for implementing platform networking.
pub mod platform {
    pub use super::io::link::{
        EmbeddedIOLink, EmbeddedIOLinkRx, EmbeddedIOLinkTx, ZLink, ZLinkInfo, ZLinkManager,
        ZLinkRx, ZLinkTx,
    };
    pub use zenoh_derive::{ZLink, ZLinkInfo, ZLinkRx, ZLinkTx};
    pub use zenoh_proto::{Endpoint, LinkError};

    pub mod zenoh {
        pub use zenoh_proto::{debug, error, info, trace, warn, zbail};
    }
}
