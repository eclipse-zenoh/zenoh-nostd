use {
    async_net::TcpStream,
    wtx::{
        collection::Vector,
        rng::Xorshift64,
        web_socket::{
            Frame, OpCode, WebSocketPayloadOrigin, WebSocketReaderOwned, WebSocketWriterOwned,
        },
    },
    zenoh_nostd::platform::*,
};

/// WebSocket link — `IS_CLIENT = true` for client connections, `false` for server.
pub struct StdWsLink<const IS_CLIENT: bool> {
    stream: WebSocketReaderOwned<(), Xorshift64, TcpStream, IS_CLIENT>,
    sink: WebSocketWriterOwned<(), Xorshift64, TcpStream, IS_CLIENT>,
    read_buffer: Vector<u8>,
    write_buffer: Vector<u8>,
    mtu: u16,
}

impl<const IS_CLIENT: bool> StdWsLink<IS_CLIENT> {
    pub fn new(
        stream: WebSocketReaderOwned<(), Xorshift64, TcpStream, IS_CLIENT>,
        sink: WebSocketWriterOwned<(), Xorshift64, TcpStream, IS_CLIENT>,
        mtu: u16,
    ) -> Self {
        Self {
            stream,
            sink,
            read_buffer: Vector::new(),
            write_buffer: Vector::new(),
            mtu,
        }
    }
}

pub struct StdWsLinkTx<'a, const IS_CLIENT: bool> {
    sink: &'a mut WebSocketWriterOwned<(), Xorshift64, TcpStream, IS_CLIENT>,
    write_buffer: &'a mut Vector<u8>,
    mtu: u16,
}

pub struct StdWsLinkRx<'a, const IS_CLIENT: bool> {
    stream: &'a mut WebSocketReaderOwned<(), Xorshift64, TcpStream, IS_CLIENT>,
    read_buffer: &'a mut Vector<u8>,
    mtu: u16,
}

impl<const IS_CLIENT: bool> ZLinkInfo for StdWsLink<IS_CLIENT> {
    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn is_streamed(&self) -> bool {
        false
    }
}

impl<const IS_CLIENT: bool> ZLinkInfo for StdWsLinkTx<'_, IS_CLIENT> {
    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn is_streamed(&self) -> bool {
        false
    }
}

impl<const IS_CLIENT: bool> ZLinkInfo for StdWsLinkRx<'_, IS_CLIENT> {
    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn is_streamed(&self) -> bool {
        false
    }
}

impl<const IS_CLIENT: bool> ZLinkTx for StdWsLink<IS_CLIENT> {
    async fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        self.write_buffer.clear();
        self.write_buffer
            .extend_from_copyable_slice(buffer)
            .map_err(|e| {
                zenoh::error!("Failed to extend write buffer: {}", e);
                LinkError::LinkTxFailed
            })?;

        let payload = self.write_buffer.as_slice_mut();

        self.sink
            .write_frame(&mut Frame::new_fin(OpCode::Binary, payload))
            .await
            .map_err(|e| {
                zenoh::error!("Could not write frame: {}", e);
                LinkError::LinkTxFailed
            })
    }
}

impl<const IS_CLIENT: bool> ZLinkTx for StdWsLinkTx<'_, IS_CLIENT> {
    async fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        self.write_buffer.clear();
        self.write_buffer
            .extend_from_copyable_slice(buffer)
            .map_err(|e| {
                zenoh::error!("Failed to extend write buffer: {}", e);
                LinkError::LinkTxFailed
            })?;

        let payload = self.write_buffer.as_slice_mut();

        self.sink
            .write_frame(&mut Frame::new_fin(OpCode::Binary, payload))
            .await
            .map_err(|e| {
                zenoh::error!("Could not write frame: {}", e);
                LinkError::LinkTxFailed
            })
    }
}

impl<const IS_CLIENT: bool> ZLinkRx for StdWsLink<IS_CLIENT> {
    async fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        self.read_buffer.clear();

        let frame = self
            .stream
            .read_frame(&mut self.read_buffer, WebSocketPayloadOrigin::Consistent)
            .await
            .map_err(|e| {
                zenoh::error!("Could not read frame: {}", e);
                LinkError::LinkRxFailed
            })?;

        match frame.op_code() {
            OpCode::Binary => {
                let len = frame.payload().len().min(buffer.len());
                buffer[..len].copy_from_slice(&frame.payload()[..len]);
                Ok(len)
            }
            code => {
                zenoh::error!(
                    "Could not read frame into buffer: unexpected OpCode {:?}",
                    code
                );
                zenoh::zbail!(LinkError::LinkRxFailed);
            }
        }
    }

    async fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        self.read(buffer).await.map(|_| ())
    }
}

impl<const IS_CLIENT: bool> ZLinkRx for StdWsLinkRx<'_, IS_CLIENT> {
    async fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        self.read_buffer.clear();

        let frame = self
            .stream
            .read_frame(self.read_buffer, WebSocketPayloadOrigin::Consistent)
            .await
            .map_err(|e| {
                zenoh::error!("Could not read frame: {}", e);
                LinkError::LinkRxFailed
            })?;

        match frame.op_code() {
            OpCode::Binary => {
                let len = frame.payload().len().min(buffer.len());
                buffer[..len].copy_from_slice(&frame.payload()[..len]);
                Ok(len)
            }
            code => {
                zenoh::error!(
                    "Could not read frame into buffer: unexpected OpCode {:?}",
                    code
                );
                zenoh::zbail!(LinkError::LinkRxFailed);
            }
        }
    }

    async fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        self.read(buffer).await.map(|_| ())
    }
}

impl<const IS_CLIENT: bool> ZLink for StdWsLink<IS_CLIENT> {
    type Tx<'link>
        = StdWsLinkTx<'link, IS_CLIENT>
    where
        Self: 'link;

    type Rx<'link>
        = StdWsLinkRx<'link, IS_CLIENT>
    where
        Self: 'link;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        let tx = StdWsLinkTx {
            sink: &mut self.sink,
            write_buffer: &mut self.write_buffer,
            mtu: self.mtu,
        };
        let rx = StdWsLinkRx {
            stream: &mut self.stream,
            read_buffer: &mut self.read_buffer,
            mtu: self.mtu,
        };
        (tx, rx)
    }
}
