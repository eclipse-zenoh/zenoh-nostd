use embedded_io::{Read, Write};
use zenoh_proto::{Endpoint, LinkError};

pub trait ZLinkInfo {
    fn mtu(&self) -> u16;
    fn is_streamed(&self) -> bool;
}

pub trait ZLinkTx: ZLinkInfo {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), zenoh_proto::LinkError>;
}

pub trait ZLinkRx: ZLinkInfo {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, zenoh_proto::LinkError>;

    fn read_exact(&mut self, buffer: &mut [u8])
    -> core::result::Result<(), zenoh_proto::LinkError>;
}

pub trait ZLink: ZLinkInfo + ZLinkTx + ZLinkRx {
    type Tx<'link>: ZLinkTx + ZLinkInfo
    where
        Self: 'link;

    type Rx<'link>: ZLinkRx + ZLinkInfo
    where
        Self: 'link;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>);
}

pub trait ZLinkManager {
    type Link<'a>: ZLink
    where
        Self: 'a;

    fn connect(&self, endpoint: Endpoint<'_>) -> core::result::Result<Self::Link<'_>, LinkError>;

    fn listen(&self, endpoint: Endpoint<'_>) -> core::result::Result<Self::Link<'_>, LinkError>;
}

pub struct EmbeddedIOLink<Tx: Write, Rx: Read> {
    tx: Tx,
    rx: Rx,

    mtu: u16,
    is_streamed: bool,
}

impl<Tx: Write, Rx: Read> EmbeddedIOLink<Tx, Rx> {
    pub fn new(tx: Tx, rx: Rx, mtu: u16, is_streamed: bool) -> Self {
        Self {
            tx,
            rx,
            mtu,
            is_streamed,
        }
    }
}

pub struct EmbeddedIOLinkTx<'a, Tx: Write> {
    tx: &'a mut Tx,

    mtu: u16,
    is_streamed: bool,
}

pub struct EmbeddedIOLinkRx<'a, Rx: Read> {
    rx: &'a mut Rx,

    mtu: u16,
    is_streamed: bool,
}

impl<Tx: Write, Rx: Read> ZLinkInfo for EmbeddedIOLink<Tx, Rx> {
    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn is_streamed(&self) -> bool {
        self.is_streamed
    }
}

impl<Tx: Write> ZLinkInfo for EmbeddedIOLinkTx<'_, Tx> {
    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn is_streamed(&self) -> bool {
        self.is_streamed
    }
}

impl<Rx: Read> ZLinkInfo for EmbeddedIOLinkRx<'_, Rx> {
    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn is_streamed(&self) -> bool {
        self.is_streamed
    }
}

impl<Tx: Write, Rx: Read> ZLinkTx for EmbeddedIOLink<Tx, Rx> {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), zenoh_proto::LinkError> {
        self.tx
            .write_all(buffer)
            .map_err(|_| LinkError::LinkTxFailed)
    }
}

impl<Tx: Write> ZLinkTx for EmbeddedIOLinkTx<'_, Tx> {
    fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), zenoh_proto::LinkError> {
        self.tx
            .write_all(buffer)
            .map_err(|_| LinkError::LinkTxFailed)
    }
}

impl<Tx: Write, Rx: Read> ZLinkRx for EmbeddedIOLink<Tx, Rx> {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, zenoh_proto::LinkError> {
        self.rx.read(buffer).map_err(|_| LinkError::LinkRxFailed)
    }

    fn read_exact(
        &mut self,
        buffer: &mut [u8],
    ) -> core::result::Result<(), zenoh_proto::LinkError> {
        self.rx
            .read_exact(buffer)
            .map_err(|_| LinkError::LinkRxFailed)
    }
}

impl<Rx: Read> ZLinkRx for EmbeddedIOLinkRx<'_, Rx> {
    fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, zenoh_proto::LinkError> {
        self.rx.read(buffer).map_err(|_| LinkError::LinkRxFailed)
    }

    fn read_exact(
        &mut self,
        buffer: &mut [u8],
    ) -> core::result::Result<(), zenoh_proto::LinkError> {
        self.rx
            .read_exact(buffer)
            .map_err(|_| LinkError::LinkRxFailed)
    }
}

impl<Tx: Write, Rx: Read> ZLink for EmbeddedIOLink<Tx, Rx> {
    type Tx<'link>
        = EmbeddedIOLinkTx<'link, Tx>
    where
        Self: 'link;

    type Rx<'link>
        = EmbeddedIOLinkRx<'link, Rx>
    where
        Self: 'link;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        let Self {
            tx,
            rx,
            mtu,
            is_streamed,
        } = self;

        (
            EmbeddedIOLinkTx {
                tx,
                mtu: *mtu,
                is_streamed: *is_streamed,
            },
            EmbeddedIOLinkRx {
                rx,
                mtu: *mtu,
                is_streamed: *is_streamed,
            },
        )
    }
}
