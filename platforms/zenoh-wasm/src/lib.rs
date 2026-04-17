use std::net::SocketAddr;

use zenoh_nostd::platform::*;

pub mod ws;

pub struct WasmLinkManager;

#[derive(ZLinkInfo, ZLinkTx, ZLinkRx, ZLink)]
#[zenoh(ZLink = (WasmLinkTx<'link>, WasmLinkRx<'link>))]
pub enum WasmLink {
    Ws(ws::WasmWsLink),
}

#[derive(ZLinkInfo, ZLinkTx)]
pub enum WasmLinkTx<'link> {
    Ws(ws::WasmWsLinkTx<'link>),
}

#[derive(ZLinkInfo, ZLinkRx)]
pub enum WasmLinkRx<'link> {
    Ws(ws::WasmWsLinkRx<'link>),
}

impl WasmLink {
    /// Close the underlying browser WebSocket.
    ///
    /// Calling this causes the session run loop's `rx.recv()` to return an
    /// error (via the onclose channel), making `driver.run()` exit. On the
    /// broker side, the TCP/WS connection termination causes `driver.run()` to
    /// return an error too, which lets the broker's `accept()` loop call
    /// `listen()` again and accept the next incoming connection.
    pub fn close_ws(&self) {
        match self {
            WasmLink::Ws(ws) => ws.close_ws(),
        }
    }
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
                let link = ws::WasmWsLink::connect(&url).await?;
                Ok(Self::Link::Ws(link))
            }
            _ => zenoh::zbail!(LinkError::CouldNotParseProtocol),
        }
    }

    async fn listen(&self, _: Endpoint<'_>) -> core::result::Result<Self::Link<'_>, LinkError> {
        zenoh::zbail!(LinkError::CouldNotListen)
    }
}
