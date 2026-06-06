#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod api;

mod config;
mod io;
mod resources;

pub mod session {
    pub use super::config::ZSessionConfig;
    pub use super::io::transport::TransportLinkManager;
    pub use super::resources::Resources;
    pub use zenoh_proto::fields::WhatAmI;
    pub use zenoh_proto::{Endpoint, Error};

    pub use super::api::{
        connectivity::{ConnectivityState, TransportEvent},
        advanced_publisher::*,
        advanced_subscriber::*,
        auto_reconnect::*,
        encoding::*,
        query::*,
        response::*,
        sample::*,
        session::Session,
        session::{
            delete::*, get::*, liveliness::*, r#pub::*, put::*, querier::*, queryable::*, sub::*,
        },
    };

    pub use zenoh_proto::msgs::exts::QoS;
    pub use zenoh_proto::msgs::exts::QueryTarget;
    pub use zenoh_proto::msgs::fields::{CongestionControl, ConsolidationMode, Locality, Priority, Reliability};
    pub use zenoh_proto::msgs::{DeclareKeyExpr, Interest, UndeclareKeyExpr};

    pub mod zenoh {
        pub use super::super::api::callbacks::storage;
        pub use super::super::api::session::{
            session_connect as connect,
            session_connect_ignore_invalid_sn as connect_ignore_invalid_sn,
            session_listen as listen, session_listen_ignore_invalid_sn as listen_ignore_invalid_sn,
        };

        pub use crate::{__session_connect as connect, __session_listen as listen};

        pub use zenoh_proto::{debug, error, info, keyexpr, trace, warn, zbail};

        pub type ZResult<T> = core::result::Result<T, super::Error>;
    }
}

#[cfg(feature = "alloc")]
pub mod broker {
    pub use super::config::ZBrokerConfig;
    pub use super::io::transport::TransportLinkManager;
    pub use zenoh_proto::{Endpoint, Error};

    pub use super::api::broker::Broker;
    pub use super::api::mesh::{MeshEntry, MeshState};

    pub mod zenoh {
        pub use zenoh_proto::{debug, error, info, keyexpr, trace, warn, zbail};
        pub type ZResult<T> = core::result::Result<T, super::Error>;

        pub use crate::__broker as broker;
    }
}

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
