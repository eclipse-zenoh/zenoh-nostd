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

impl ZLinkManager for WasmLinkManager {
    type Link<'a>
        = WasmLink
    where
        Self: 'a;

    fn connect(
        &self,
        endpoint: Endpoint<'_>,
    ) -> core::result::Result<Self::Link<'_>, LinkError> {
        let protocol = endpoint.protocol();
        let address = endpoint.address();

        match protocol.as_str() {
            "ws" => {
                let url = format!("ws://{}", address);
                let (socket, _) =
                    tungstenite::connect(url).map_err(|_| LinkError::CouldNotConnect)?;
                Ok(Self::Link::Ws(ws::WasmWsLink::new(socket)))
            }
            _ => zenoh::zbail!(LinkError::CouldNotParseProtocol),
        }
    }

    fn listen(&self, _: Endpoint<'_>) -> core::result::Result<Self::Link<'_>, LinkError> {
        zenoh::zbail!(LinkError::CouldNotListen)
    }
}