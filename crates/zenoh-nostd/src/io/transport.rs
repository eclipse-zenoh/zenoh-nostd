use core::time::Duration;

use zenoh_proto::{
    Endpoint, TransportLinkError,
    fields::{Resolution, WhatAmI, ZenohIdProto},
};
use zenoh_sansio::{Transport, ZTransportRx, ZTransportTx};

use crate::io::link::EmbeddedIOLink;

use super::link::{ZLink, ZLinkInfo, ZLinkManager, ZLinkRx, ZLinkTx};

mod rx;
mod traits;
mod tx;

pub use rx::*;
pub use traits::*;
pub use tx::*;

pub struct TransportLink<Link, Buff> {
    link: Link,
    transport: Transport<Buff>,
}

impl<Link, Buff> TransportLink<Link, Buff> {
    pub fn new(link: Link, transport: Transport<Buff>) -> Self {
        Self { link, transport }
    }

    pub fn split(
        &mut self,
    ) -> (
        TransportLinkTx<'_, Link::Tx<'_>, Buff>,
        TransportLinkRx<'_, Link::Rx<'_>, Buff>,
    )
    where
        Link: ZLink,
    {
        let (link_tx, link_rx) = self.link.split();
        let (transport_tx, transport_rx) = self.transport.split();

        (
            TransportLinkTx::new(link_tx, transport_tx),
            TransportLinkRx::new(link_rx, transport_rx),
        )
    }

    pub fn transport(&self) -> &Transport<Buff> {
        &self.transport
    }

    pub fn transport_mut(&mut self) -> &mut Transport<Buff> {
        &mut self.transport
    }
}

impl<Link, Buff> ZTransportLinkTx for TransportLink<Link, Buff>
where
    Link: ZLinkTx,
    Buff: AsMut<[u8]> + AsRef<[u8]> + Clone,
{
    fn tx(&mut self) -> (&mut impl ZLinkTx, &mut impl ZTransportTx) {
        (&mut self.link, &mut self.transport.tx)
    }
}

impl<Link, Buff> ZTransportLinkRx for TransportLink<Link, Buff>
where
    Link: ZLinkRx,
    Buff: AsMut<[u8]> + AsRef<[u8]> + Clone,
{
    fn rx(&mut self) -> (&mut impl ZLinkRx, &mut impl ZTransportRx) {
        (&mut self.link, &mut self.transport.rx)
    }
}

pub struct TransportLinkManager<LinkManager> {
    link_manager: LinkManager,

    #[allow(dead_code)]
    open_timeout: Duration,
    zid: ZenohIdProto,
    whatami: WhatAmI,
    lease: Duration,
    resolution: Resolution,

    #[cfg(feature = "auth")]
    auth_payload: Option<heapless::Vec<u8, 256>>,
}

impl<LinkManager> From<LinkManager> for TransportLinkManager<LinkManager> {
    fn from(value: LinkManager) -> Self {
        Self::new(
            value,
            Duration::from_secs(10),
            ZenohIdProto::default(),
            WhatAmI::default(),
            Duration::from_secs(10),
            Resolution::default(),
        )
    }
}

impl<LinkManager> TransportLinkManager<LinkManager> {
    pub(crate) fn new(
        link_manager: LinkManager,
        open_timeout: Duration,
        zid: ZenohIdProto,
        whatami: WhatAmI,
        lease: Duration,
        resolution: Resolution,
    ) -> Self {
        Self {
            link_manager,
            open_timeout,
            zid,
            whatami,
            lease,
            resolution,

            #[cfg(feature = "auth")]
            auth_payload: None,
        }
    }

    pub fn with_whatami(mut self, whatami: WhatAmI) -> Self {
        self.whatami = whatami;
        self
    }

    #[cfg(feature = "auth")]
    pub fn with_auth(mut self, username: &str, password: &str) -> Self {
        let mut payload = heapless::Vec::new();
        let _ = payload.push(0x01);
        for b in username.bytes() {
            let _ = payload.push(b);
        }
        let _ = payload.push(0x00);
        for b in password.bytes() {
            let _ = payload.push(b);
        }
        self.auth_payload = Some(payload);
        self
    }

    pub fn bridge_connect<Tx: embedded_io::Write, Rx: embedded_io::Read, Buff>(
        &self,
        mut link: EmbeddedIOLink<Tx, Rx>,
        buff: Buff,
    ) -> core::result::Result<TransportLink<EmbeddedIOLink<Tx, Rx>, Buff>, TransportLinkError>
    where
        Buff: AsMut<[u8]> + AsRef<[u8]> + Clone,
    {
        let streamed = link.is_streamed();
        let transport = {
            #[cfg_attr(not(feature = "auth"), allow(unused_mut))]
            let mut builder = Transport::builder(buff)
                .with_zid(self.zid)
                .with_whatami(self.whatami)
                .with_lease(self.lease)
                .with_resolution(self.resolution);
            #[cfg(feature = "auth")]
            if let Some(ref p) = self.auth_payload {
                builder = builder.with_auth_payload(p.clone());
            }
            builder
                .connect(
                &mut link,
                |link, bytes| {
                    if link.is_streamed() {
                        link.read_exact(bytes).map(|_| bytes.len())
                    } else {
                        link.read(bytes)
                    }
                },
                |link, bytes| link.write_all(bytes),
            )
            .with_prefixed(streamed)
            .finish()
            .map_err(|e| e.flatten_map::<TransportLinkError>())?
        };

        Ok(TransportLink::new(link, transport))
    }

    pub fn bridge_listen<Tx: embedded_io::Write, Rx: embedded_io::Read, Buff>(
        &self,
        mut link: EmbeddedIOLink<Tx, Rx>,
        buff: Buff,
    ) -> core::result::Result<TransportLink<EmbeddedIOLink<Tx, Rx>, Buff>, TransportLinkError>
    where
        Buff: AsMut<[u8]> + AsRef<[u8]> + Clone,
    {
        let streamed = link.is_streamed();
        let transport = {
            #[cfg_attr(not(feature = "auth"), allow(unused_mut))]
            let mut builder = Transport::builder(buff)
                .with_zid(self.zid)
                .with_whatami(self.whatami)
                .with_lease(self.lease)
                .with_resolution(self.resolution);
            #[cfg(feature = "auth")]
            if let Some(ref p) = self.auth_payload {
                builder = builder.with_auth_payload(p.clone());
            }
            builder
                .listen(
                &mut link,
                |link, bytes| {
                    if link.is_streamed() {
                        link.read_exact(bytes).map(|_| bytes.len())
                    } else {
                        link.read(bytes)
                    }
                },
                |link, bytes| link.write_all(bytes),
            )
            .with_prefixed(streamed)
            .finish()
            .map_err(|e| e.flatten_map::<TransportLinkError>())?
        };

        Ok(TransportLink::new(link, transport))
    }

    pub fn peer_connect<Buff>(
        &self,
        endpoint: Endpoint<'_>,
        buff: Buff,
    ) -> core::result::Result<TransportLink<LinkManager::Link<'_>, Buff>, TransportLinkError>
    where
        LinkManager: ZLinkManager,
        Buff: AsMut<[u8]> + AsRef<[u8]> + Clone,
    {
        let mut link = self.link_manager.connect(endpoint)?;
        let streamed = link.is_streamed();
        let transport = {
            #[cfg_attr(not(feature = "auth"), allow(unused_mut))]
            let mut builder = Transport::builder(buff)
                .with_zid(self.zid)
                .with_whatami(WhatAmI::Peer)
                .with_lease(self.lease)
                .with_resolution(self.resolution);
            #[cfg(feature = "auth")]
            if let Some(ref p) = self.auth_payload {
                builder = builder.with_auth_payload(p.clone());
            }
            builder
                .peer_connect(
                &mut link,
                |link, bytes| {
                    if link.is_streamed() {
                        link.read_exact(bytes).map(|_| bytes.len())
                    } else {
                        link.read(bytes)
                    }
                },
                |link, bytes| link.write_all(bytes),
            )
            .with_prefixed(streamed)
            .finish()
            .map_err(|e| e.flatten_map::<TransportLinkError>())?
        };

        Ok(TransportLink::new(link, transport))
    }

    pub fn peer_listen<Buff>(
        &self,
        endpoint: Endpoint<'_>,
        buff: Buff,
    ) -> core::result::Result<TransportLink<LinkManager::Link<'_>, Buff>, TransportLinkError>
    where
        LinkManager: ZLinkManager,
        Buff: AsMut<[u8]> + AsRef<[u8]> + Clone,
    {
        let mut link = self.link_manager.listen(endpoint)?;
        let streamed = link.is_streamed();
        let transport = {
            #[cfg_attr(not(feature = "auth"), allow(unused_mut))]
            let mut builder = Transport::builder(buff)
                .with_zid(self.zid)
                .with_whatami(WhatAmI::Peer)
                .with_lease(self.lease)
                .with_resolution(self.resolution);
            #[cfg(feature = "auth")]
            if let Some(ref p) = self.auth_payload {
                builder = builder.with_auth_payload(p.clone());
            }
            builder
                .peer_listen(
                &mut link,
                |link, bytes| {
                    if link.is_streamed() {
                        link.read_exact(bytes).map(|_| bytes.len())
                    } else {
                        link.read(bytes)
                    }
                },
                |link, bytes| link.write_all(bytes),
            )
            .with_prefixed(streamed)
            .finish()
            .map_err(|e| e.flatten_map::<TransportLinkError>())?
        };

        Ok(TransportLink::new(link, transport))
    }

    pub fn connect<Buff>(
        &self,
        endpoint: Endpoint<'_>,
        buff: Buff,
    ) -> core::result::Result<TransportLink<LinkManager::Link<'_>, Buff>, TransportLinkError>
    where
        LinkManager: ZLinkManager,
        Buff: AsMut<[u8]> + AsRef<[u8]> + Clone,
    {
        let mut link = self.link_manager.connect(endpoint)?;
        let streamed = link.is_streamed();
        let transport = {
            #[cfg_attr(not(feature = "auth"), allow(unused_mut))]
            let mut builder = Transport::builder(buff)
                .with_zid(self.zid)
                .with_whatami(self.whatami)
                .with_lease(self.lease)
                .with_resolution(self.resolution);
            #[cfg(feature = "auth")]
            if let Some(ref p) = self.auth_payload {
                builder = builder.with_auth_payload(p.clone());
            }
            builder
                .connect(
                &mut link,
                |link, bytes| {
                    if link.is_streamed() {
                        link.read_exact(bytes).map(|_| bytes.len())
                    } else {
                        link.read(bytes)
                    }
                },
                |link, bytes| link.write_all(bytes),
            )
            .with_prefixed(streamed)
            .finish()
            .map_err(|e| e.flatten_map::<TransportLinkError>())?
        };

        Ok(TransportLink::new(link, transport))
    }

    pub fn listen<Buff>(
        &self,
        endpoint: Endpoint<'_>,
        buff: Buff,
    ) -> core::result::Result<TransportLink<LinkManager::Link<'_>, Buff>, TransportLinkError>
    where
        LinkManager: ZLinkManager,
        Buff: AsMut<[u8]> + AsRef<[u8]> + Clone,
    {
        let mut link = self.link_manager.listen(endpoint)?;
        let streamed = link.is_streamed();
        let transport = {
            #[cfg_attr(not(feature = "auth"), allow(unused_mut))]
            let mut builder = Transport::builder(buff)
                .with_zid(self.zid)
                .with_whatami(self.whatami)
                .with_lease(self.lease)
                .with_resolution(self.resolution);
            #[cfg(feature = "auth")]
            if let Some(ref p) = self.auth_payload {
                builder = builder.with_auth_payload(p.clone());
            }
            builder
                .listen(
                &mut link,
                |link, bytes| {
                    if link.is_streamed() {
                        link.read_exact(bytes).map(|_| bytes.len())
                    } else {
                        link.read(bytes)
                    }
                },
                |link, bytes| link.write_all(bytes),
            )
            .with_prefixed(streamed)
            .finish()
            .map_err(|e| e.flatten_map::<TransportLinkError>())?
        };

        Ok(TransportLink::new(link, transport))
    }
}
