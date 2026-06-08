use core::fmt::Display;
use core::time::Duration;

use establishment::Description;
use zenoh_proto::{fields::*, msgs::*};

pub(crate) mod establishment;

#[cfg(kani)]
mod kani_proofs;

#[cfg(feature = "creusot")]
mod creusot_contracts;

mod handshake;
mod rx;
mod traits;
mod tx;

pub use handshake::*;
pub use rx::*;
pub use traits::*;
pub use tx::*;

use crate::transport::establishment::State;

pub struct TransportBuilder<Buff> {
    zid: ZenohIdProto,
    whatami: WhatAmI,
    batch_size: u16,
    lease: Duration,
    resolution: Resolution,
    max_fragments: usize,

    buff: Buff,

    #[cfg(feature = "auth")]
    auth_payload: Option<heapless::Vec<u8, 256>>,
}

impl<Buff> TransportBuilder<Buff> {
    fn new(buff: Buff) -> Self
    where
        Buff: AsRef<[u8]>,
    {
        TransportBuilder {
            zid: ZenohIdProto::default(),
            whatami: WhatAmI::default(),
            batch_size: buff.as_ref().len() as u16,
            lease: Duration::from_secs(10),
            resolution: Resolution::default(),
            max_fragments: 1,
            buff,

            #[cfg(feature = "auth")]
            auth_payload: None,
        }
    }
    pub fn with_zid(mut self, zid: ZenohIdProto) -> Self {
        self.zid = zid;
        self
    }

    pub fn with_whatami(mut self, whatami: WhatAmI) -> Self {
        self.whatami = whatami;
        self
    }

    pub fn with_batch_size(mut self, batch_size: u16) -> Self {
        self.batch_size = batch_size;
        self
    }

    pub fn with_lease(mut self, lease: Duration) -> Self {
        self.lease = lease;
        self
    }

    pub fn with_resolution(mut self, resolution: Resolution) -> Self {
        self.resolution = resolution;
        self
    }

    pub fn with_max_fragments(mut self, max: usize) -> Self {
        self.max_fragments = max;
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

    #[cfg(feature = "auth")]
    pub fn with_auth_payload(mut self, payload: heapless::Vec<u8, 256>) -> Self {
        self.auth_payload = Some(payload);
        self
    }

    pub fn with_buff<NewBuff>(self, buff: NewBuff) -> TransportBuilder<NewBuff> {
        TransportBuilder {
            zid: self.zid,
            whatami: self.whatami,
            batch_size: self.batch_size,
            lease: self.lease,
            resolution: self.resolution,
            max_fragments: self.max_fragments,
            buff,

            #[cfg(feature = "auth")]
            auth_payload: self.auth_payload,
        }
    }

    pub fn codec(self) -> Transport<Buff>
    where
        Buff: Clone,
    {
        Transport {
            tx: TransportTx::new(
                self.buff.clone(),
                self.batch_size as usize,
                0,
                self.resolution,
                self.lease,
            )
            .with_max_fragments(self.max_fragments),
            rx: TransportRx::new(
                self.buff,
                self.batch_size as usize,
                0,
                self.resolution,
                self.lease,
            )
            .with_max_fragments(self.max_fragments),
            mine_zid: self.zid,
            other_zid: self.zid,
        }
    }

    pub fn listen<T, E, Read, Write>(
        self,
        handle: T,
        read: Read,
        write: Write,
    ) -> Handshake<Buff, T, Read, Write>
    where
        E: Display,
        Buff: Clone + AsMut<[u8]> + AsRef<[u8]>,
        Read: FnMut(&mut T, &mut [u8]) -> core::result::Result<usize, E>,
        Write: FnMut(&mut T, &[u8]) -> core::result::Result<(), E>,
    {
        #[cfg(feature = "auth")]
        let auth_payload = self.auth_payload;
        let state = State::WaitingInitSyn {
            mine_zid: self.zid,
            mine_whatami: self.whatami,
            mine_batch_size: self.batch_size,
            mine_resolution: self.resolution,
            mine_lease: self.lease,
            #[cfg(feature = "auth")]
            mine_auth: auth_payload,
        };

        let tx = TransportTx::new(
            self.buff.clone(),
            self.batch_size as usize,
            0,
            self.resolution,
            self.lease,
        )
        .with_max_fragments(self.max_fragments);

        let rx = TransportRx::new(
            self.buff,
            self.batch_size as usize,
            0,
            self.resolution,
            self.lease,
        )
        .with_max_fragments(self.max_fragments);

        Handshake::PendingRecv {
            state,
            prefixed: false,
            tx,
            rx,
            handle,
            read,
            write,
        }
    }

    pub fn listen_async<T, E, Read, Write>(
        self,
        handle: T,
        read: Read,
        write: Write,
    ) -> Handshake<Buff, T, Read, Write>
    where
        E: Display,
        Buff: Clone + AsMut<[u8]> + AsRef<[u8]>,
        Read: AsyncFnMut(&mut T, &mut [u8]) -> core::result::Result<usize, E>,
        Write: AsyncFnMut(&mut T, &[u8]) -> core::result::Result<(), E>,
    {
        #[cfg(feature = "auth")]
        let auth_payload = self.auth_payload;
        let state = State::WaitingInitSyn {
            mine_zid: self.zid,
            mine_whatami: self.whatami,
            mine_batch_size: self.batch_size,
            mine_resolution: self.resolution,
            mine_lease: self.lease,
            #[cfg(feature = "auth")]
            mine_auth: auth_payload,
        };

        let tx = TransportTx::new(
            self.buff.clone(),
            self.batch_size as usize,
            0,
            self.resolution,
            self.lease,
        )
        .with_max_fragments(self.max_fragments);

        let rx = TransportRx::new(
            self.buff,
            self.batch_size as usize,
            0,
            self.resolution,
            self.lease,
        )
        .with_max_fragments(self.max_fragments);

        Handshake::PendingRecv {
            state,
            prefixed: false,
            tx,
            rx,
            handle,
            read,
            write,
        }
    }

    pub fn connect<T, E, Read, Write>(
        self,
        handle: T,
        read: Read,
        write: Write,
    ) -> Handshake<Buff, T, Read, Write>
    where
        E: Display,
        Buff: Clone + AsMut<[u8]> + AsRef<[u8]>,
        Read: FnMut(&mut T, &mut [u8]) -> core::result::Result<usize, E>,
        Write: FnMut(&mut T, &[u8]) -> core::result::Result<(), E>,
    {
        #[cfg(feature = "auth")]
        let auth_payload = self.auth_payload;
        let state = State::WaitingInitAck {
            mine_zid: self.zid,
            mine_whatami: self.whatami,
            mine_batch_size: self.batch_size,
            mine_resolution: self.resolution,
            mine_lease: self.lease,
            #[cfg(feature = "auth")]
            mine_auth: auth_payload.clone(),
        };

        let tx = TransportTx::new(
            self.buff.clone(),
            self.batch_size as usize,
            0,
            self.resolution,
            self.lease,
        )
        .with_max_fragments(self.max_fragments);

        let rx = TransportRx::new(
            self.buff,
            self.batch_size as usize,
            0,
            self.resolution,
            self.lease,
        )
        .with_max_fragments(self.max_fragments);

        #[cfg(feature = "auth")]
        let auth_bytes = auth_payload;
        

        #[cfg_attr(not(feature = "auth"), allow(unused_mut))]
        let mut init = InitSyn {
            identifier: InitIdentifier {
                zid: self.zid,
                whatami: self.whatami,
            },
            resolution: InitResolution {
                resolution: self.resolution,
                batch_size: BatchSize(self.batch_size),
            },
            ..Default::default()
        };

        // SAFETY: auth_bytes and init live in the same PendingInit variant.
        // The reference from auth_bytes.as_slice() is valid for 'static because
        // both fields are in the same enum variant and are dropped together.
        #[cfg(feature = "auth")]
        if let Some(ref bytes) = auth_bytes {
            init.auth = Some(zenoh_proto::exts::Auth {
                payload: unsafe {
                    core::mem::transmute::<&[u8], &'static [u8]>(bytes.as_slice())
                },
            });
        }

        Handshake::PendingInit {
            state,
            init,
            #[cfg(feature = "auth")]
            auth_bytes,
            prefixed: false,
            tx,
            rx,
            handle,
            read,
            write,
        }
    }

    pub fn peer_connect<T, E, Read, Write>(
        self,
        handle: T,
        read: Read,
        write: Write,
    ) -> Handshake<Buff, T, Read, Write>
    where
        E: Display,
        Buff: Clone + AsMut<[u8]> + AsRef<[u8]>,
        Read: FnMut(&mut T, &mut [u8]) -> core::result::Result<usize, E>,
        Write: FnMut(&mut T, &[u8]) -> core::result::Result<(), E>,
    {
        self.with_whatami(WhatAmI::Peer)
            .connect(handle, read, write)
    }

    pub fn peer_listen<T, E, Read, Write>(
        self,
        handle: T,
        read: Read,
        write: Write,
    ) -> Handshake<Buff, T, Read, Write>
    where
        E: Display,
        Buff: Clone + AsMut<[u8]> + AsRef<[u8]>,
        Read: FnMut(&mut T, &mut [u8]) -> core::result::Result<usize, E>,
        Write: FnMut(&mut T, &[u8]) -> core::result::Result<(), E>,
    {
        self.with_whatami(WhatAmI::Peer)
            .listen(handle, read, write)
    }

    pub fn connect_async<T, E, Read, Write>(
        self,
        handle: T,
        read: Read,
        write: Write,
    ) -> Handshake<Buff, T, Read, Write>
    where
        E: Display,
        Buff: Clone + AsMut<[u8]> + AsRef<[u8]>,
        Read: AsyncFnMut(&mut T, &mut [u8]) -> core::result::Result<usize, E>,
        Write: AsyncFnMut(&mut T, &[u8]) -> core::result::Result<(), E>,
    {
        #[cfg(feature = "auth")]
        let auth_payload = self.auth_payload;
        let state = State::WaitingInitAck {
            mine_zid: self.zid,
            mine_whatami: self.whatami,
            mine_batch_size: self.batch_size,
            mine_resolution: self.resolution,
            mine_lease: self.lease,
            #[cfg(feature = "auth")]
            mine_auth: auth_payload.clone(),
        };

        let tx = TransportTx::new(
            self.buff.clone(),
            self.batch_size as usize,
            0,
            self.resolution,
            self.lease,
        )
        .with_max_fragments(self.max_fragments);

        let rx = TransportRx::new(
            self.buff,
            self.batch_size as usize,
            0,
            self.resolution,
            self.lease,
        )
        .with_max_fragments(self.max_fragments);

        #[cfg(feature = "auth")]
        let auth_bytes = auth_payload;
        

        #[cfg_attr(not(feature = "auth"), allow(unused_mut))]
        let mut init = InitSyn {
            identifier: InitIdentifier {
                zid: self.zid,
                whatami: self.whatami,
            },
            resolution: InitResolution {
                resolution: self.resolution,
                batch_size: BatchSize(self.batch_size),
            },
            ..Default::default()
        };

        #[cfg(feature = "auth")]
        if let Some(ref bytes) = auth_bytes {
            init.auth = Some(zenoh_proto::exts::Auth {
                payload: unsafe {
                    core::mem::transmute::<&[u8], &'static [u8]>(bytes.as_slice())
                },
            });
        }

        Handshake::PendingInit {
            state,
            init,
            #[cfg(feature = "auth")]
            auth_bytes,
            prefixed: false,
            tx,
            rx,
            handle,
            read,
            write,
        }
    }
}

pub struct Transport<Buff> {
    pub tx: TransportTx<Buff>,
    pub rx: TransportRx<Buff>,

    pub mine_zid: ZenohIdProto,
    pub other_zid: ZenohIdProto,
}

impl<Buff> Transport<Buff> {
    pub fn builder(buff: Buff) -> TransportBuilder<Buff>
    where
        Buff: AsRef<[u8]>,
    {
        TransportBuilder::new(buff)
    }

    pub(crate) fn new(description: Description, tx: Buff, rx: Buff) -> Self {
        Self {
            tx: TransportTx::new(
                tx,
                description.batch_size as usize,
                description.mine_sn,
                description.resolution,
                description.mine_lease,
            )
            .with_max_fragments(1),
            rx: TransportRx::new(
                rx,
                description.batch_size as usize,
                description.other_sn,
                description.resolution,
                description.other_lease,
            )
            .with_max_fragments(1),
            mine_zid: description.mine_zid,
            other_zid: description.other_zid,
        }
    }

    pub fn sync(&mut self, now: Duration) {
        let Self { tx, rx, .. } = self;
        rx.sync(Some(tx), now);
        tx.sync(Some(rx), now);
    }

    pub fn closed(&self) -> bool {
        self.tx.closed() || self.rx.closed()
    }

    pub fn split(&mut self) -> (&mut TransportTx<Buff>, &mut TransportRx<Buff>) {
        (&mut self.tx, &mut self.rx)
    }
}
