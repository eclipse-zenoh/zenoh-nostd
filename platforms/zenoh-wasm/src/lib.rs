//! Outgoing browser WebSocket links for `zenoh-nostd`.
//!
//! Use [`WasmLinkManager`] with a router exposing a WebSocket listener. The manager
//! accepts numeric `ws/IP:port` endpoints; it does not implement DNS lookup, URL
//! paths, TLS (`wss`) or listening/brokering. Browser origin/security rules still
//! apply, and compilation does not establish that a browser permits a connection.
//!
//! The application owns its browser executor and Embassy-compatible time driver,
//! keeps session-driver polling concurrent with operations, and uses a single
//! executor/thread context. Select `alloc` on the core dependency for heap-backed
//! Zenoh storage; this adapter has no feature of that name. Dropping a browser
//! link requests WebSocket closure.
//!
//! Binary WebSocket messages are treated as datagrams. The current reader copies
//! only what fits in the caller buffer and discards an oversized message's suffix;
//! size protocol buffers for the peer's batches to avoid truncation.

#![warn(missing_docs)]

use std::net::SocketAddr;

use yawc::WebSocket;
use zenoh_nostd::platform::*;

/// Binary WebSocket link and its split halves.
pub mod ws;

/// Stateless factory for outgoing browser WebSocket links; listening returns `CouldNotListen`.
pub struct WasmLinkManager;

#[derive(ZLinkInfo, ZLinkTx, ZLinkRx, ZLink)]
#[zenoh(ZLink = (WasmLinkTx<'link>, WasmLinkRx<'link>))]
/// Connection selected by the link manager; split it through the `ZLink` trait.
pub enum WasmLink {
    /// Binary WebSocket messages.
    Ws(ws::WasmWsLink),
}

#[derive(ZLinkInfo, ZLinkTx)]
/// Transmit half produced by `ZLink::split`; retains the selected framing policy.
pub enum WasmLinkTx<'link> {
    /// Binary WebSocket messages.
    Ws(ws::WasmWsLinkTx<'link>),
}

#[derive(ZLinkInfo, ZLinkRx)]
/// Receive half produced by `ZLink::split`; retains the selected framing policy.
pub enum WasmLinkRx<'link> {
    /// Binary WebSocket messages.
    Ws(ws::WasmWsLinkRx<'link>),
}

impl ZLinkManager for WasmLinkManager {
    type Link<'a>
        = WasmLink
    where
        Self: 'a;

    async fn connect(
        &self,
        endpoint: Endpoint<'_>,
    ) -> core::result::Result<Self::Link<'_>, LinkError> {
        let protocol = endpoint.protocol();
        let address = endpoint.address();

        match protocol.as_str() {
            "ws" => {
                let dst_addr = SocketAddr::try_from(address)?;
                let url = format!("ws://{}", dst_addr);
                let socket =
                    WebSocket::connect(url.parse().map_err(|_| LinkError::CouldNotConnect)?)
                        .await
                        .map_err(|_| LinkError::CouldNotConnect)?;

                Ok(Self::Link::Ws(ws::WasmWsLink::new(socket)))
            }
            _ => zenoh::zbail!(LinkError::CouldNotParseProtocol),
        }
    }

    async fn listen(&self, _: Endpoint<'_>) -> core::result::Result<Self::Link<'_>, LinkError> {
        zenoh::zbail!(LinkError::CouldNotListen)
    }
}
