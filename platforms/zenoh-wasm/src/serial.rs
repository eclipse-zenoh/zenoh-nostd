//! Web Serial link speaking z-serial framing, so the page talks straight to a zenohd
//! serial listener or a zenoh-pico device.
//!
//! The browser only hands out a `SerialPort` after a user gesture, so JS picks and opens
//! the port and passes it in with [`set_serial_port`]. The endpoint address is ignored,
//! `serial/webserial` reads fine.
//!
//! The listen side sends an R as soon as it has the port. zenoh-pico sends its I only
//! once at boot and then blocks waiting for the answer, so a device that booted before
//! the page opened the port never connects. On an R it waits 250 ms and sends I again.
//! Whatever is already queued in the port is dropped before each R: stale bytes can
//! glue to the next I and get it dropped as a broken frame, and a stale I would get an
//! I|A the device doesn't expect. Without an I the R is repeated every 5 s.
//!
//! A device that was already retrying reads the R and the I|A back to back, handles the
//! R first and sends one more I, which the queued I|A already answers. So an I after
//! `listen` returns is dropped: a second I|A would land where zenoh-pico waits for the
//! InitAck and fail its open. Broken frames, like the partial one left in the port
//! before the first 0x00, are dropped too and never fail the link.
//!
//! Once open, 5 s without a frame fails the link, so a device that went away is noticed
//! well before the session lease runs out. The port stays with us and the next connect
//! or listen reuses it, until it is closed and has to be picked again.

use {
    crate::zserial::{self, ACK, INIT, MAX_FRAME, MTU, RESET},
    embassy_time::{Duration, Instant, Timer, with_deadline, with_timeout},
    js_sys::{Reflect, Uint8Array},
    std::cell::RefCell,
    wasm_bindgen::{JsCast, JsValue, prelude::wasm_bindgen},
    wasm_bindgen_futures::JsFuture,
    web_sys::{
        ReadableStream, ReadableStreamDefaultReader, WritableStream, WritableStreamDefaultWriter,
    },
    zenoh_nostd::platform::*,
};

/// How long without a frame before the link counts as lost, and between two R.
const SILENCE: Duration = Duration::from_secs(5);

thread_local! {
    static PORT: RefCell<Option<JsValue>> = const { RefCell::new(None) };
}

/// Hands an opened `SerialPort` to the next `serial/...` connect or listen.
#[wasm_bindgen]
pub fn set_serial_port(port: JsValue) {
    PORT.with(|slot| slot.replace(Some(port)));
}

async fn port() -> JsValue {
    loop {
        if let Some(port) = PORT.with(|slot| slot.borrow().clone()) {
            return port;
        }
        Timer::after(Duration::from_millis(50)).await;
    }
}

fn forget_port() {
    PORT.with(|slot| slot.take());
    zenoh::error!("Serial port closed, pick it again");
}

fn stream<T: JsCast>(port: &JsValue, name: &str) -> Result<T, LinkError> {
    Reflect::get(port, &name.into())
        .ok()
        .and_then(|value| value.dyn_into().ok())
        .ok_or(LinkError::CouldNotConnect)
}

pub struct SerialTx {
    writer: WritableStreamDefaultWriter,
    frame: Box<[u8; MAX_FRAME]>,
}

impl SerialTx {
    async fn send(&mut self, header: u8, payload: &[u8]) -> Result<(), LinkError> {
        let len =
            zserial::encode(header, payload, &mut self.frame).ok_or(LinkError::LinkTxFailed)?;
        let chunk = Uint8Array::from(&self.frame[..len]);
        JsFuture::from(self.writer.write_with_chunk(&chunk))
            .await
            .map(|_| ())
            .map_err(|_| LinkError::LinkTxFailed)
    }
}

impl Drop for SerialTx {
    fn drop(&mut self) {
        self.writer.release_lock();
    }
}

pub struct SerialRx {
    reader: ReadableStreamDefaultReader,
    pending: Vec<u8>,
    read: Option<JsFuture>,
    seen: Instant,
}

impl Drop for SerialRx {
    fn drop(&mut self) {
        self.reader.release_lock();
    }
}

impl SerialRx {
    /// Next valid frame. Broken frames are dropped like line noise.
    async fn recv(&mut self, buffer: &mut [u8]) -> Result<(u8, usize), LinkError> {
        loop {
            if let Some(end) = self.pending.iter().position(|&byte| byte == 0) {
                let frame = zserial::decode(&mut self.pending[..end]).map(|(header, payload)| {
                    let len = payload.len().min(buffer.len());
                    buffer[..len].copy_from_slice(&payload[..len]);
                    (header, len)
                });
                self.pending.drain(..=end);
                match frame {
                    Some(frame) => {
                        self.seen = Instant::now();
                        return Ok(frame);
                    }
                    None => {
                        zenoh::warn!("dropping a broken serial frame");
                        continue;
                    }
                }
            }
            if self.pending.len() > MAX_FRAME {
                self.pending.clear();
            }
            self.fill().await?;
        }
    }

    /// Appends the next chunk from the port to `pending`.
    async fn fill(&mut self) -> Result<(), LinkError> {
        let reader = &self.reader;
        let read = self
            .read
            .get_or_insert_with(|| JsFuture::from(reader.read()));
        let chunk = read.await;
        self.read = None;
        let chunk = chunk.map_err(|_| LinkError::LinkRxFailed)?;
        let value = Reflect::get(&chunk, &"value".into()).map_err(|_| LinkError::LinkRxFailed)?;
        let Some(bytes) = value.dyn_ref::<Uint8Array>() else {
            // `done`, the port was closed.
            forget_port();
            return Err(LinkError::LinkRxFailed);
        };
        self.pending.extend(bytes.to_vec());
        Ok(())
    }

    /// Drops what is already queued: reads until the port stays quiet for 100 ms.
    async fn drain(&mut self) -> Result<(), LinkError> {
        while let Ok(chunk) = with_timeout(Duration::from_millis(100), self.fill()).await {
            chunk?;
        }
        self.pending.clear();
        Ok(())
    }

    async fn read_data(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        loop {
            let Ok(frame) = with_deadline(self.seen + SILENCE, self.recv(buffer)).await else {
                zenoh::warn!("Connection lost (no data for 5 s), reconnecting...");
                return Err(LinkError::LinkRxFailed);
            };
            let (_, len) = frame?;
            if len > 0 {
                return Ok(len);
            }
        }
    }
}

pub struct WasmSerialLink {
    tx: SerialTx,
    rx: SerialRx,
}

impl WasmSerialLink {
    async fn open() -> Result<Self, LinkError> {
        loop {
            let port = port().await;
            let (Ok(readable), Ok(writable)) = (
                stream::<ReadableStream>(&port, "readable"),
                stream::<WritableStream>(&port, "writable"),
            ) else {
                forget_port();
                continue;
            };
            return Ok(Self {
                tx: SerialTx {
                    writer: writable
                        .get_writer()
                        .map_err(|_| LinkError::CouldNotConnect)?,
                    frame: Box::new([0; MAX_FRAME]),
                },
                rx: SerialRx {
                    reader: readable.get_reader().unchecked_into(),
                    pending: Vec::new(),
                    read: None,
                    seen: Instant::now(),
                },
            });
        }
    }

    /// Same handshake as z-serial `connect`: send I, wait for I|A, retry on R.
    pub async fn connect() -> Result<Self, LinkError> {
        let mut link = Self::open().await?;
        let mut scratch = [0u8; MTU];
        loop {
            link.tx.send(INIT, &[]).await?;
            let (header, _) = link.rx.recv(&mut scratch).await?;
            if header & (INIT | ACK) == INIT | ACK {
                return Ok(link);
            }
            if header & RESET == 0 {
                return Err(LinkError::CouldNotConnect);
            }
            Timer::after(Duration::from_millis(250)).await;
        }
    }

    /// Same handshake as z-serial `accept`: wait for I, answer I|A. Drops what is queued
    /// and sends an R first, and again after every 5 s without an I, see the module
    /// docs. Anything that isn't an I is dropped.
    pub async fn listen() -> Result<Self, LinkError> {
        let mut link = Self::open().await?;
        let mut scratch = [0u8; MTU];
        let mut attempt = 0;
        loop {
            link.rx.drain().await?;
            link.tx.send(RESET, &[]).await?;
            let rx = &mut link.rx;
            let init = with_timeout(SILENCE, async {
                loop {
                    let (header, _) = rx.recv(&mut scratch).await?;
                    if header & (INIT | ACK) == INIT {
                        return Ok::<_, LinkError>(());
                    }
                }
            })
            .await;
            if let Ok(init) = init {
                init?;
                link.tx.send(INIT | ACK, &[]).await?;
                return Ok(link);
            }
            attempt += 1;
            zenoh::warn!("No device yet, retrying (attempt {})...", attempt);
        }
    }
}

pub struct WasmSerialLinkTx<'a>(&'a mut SerialTx);

pub struct WasmSerialLinkRx<'a>(&'a mut SerialRx);

macro_rules! serial_info {
    ($($link:ty),*) => {$(
        impl ZLinkInfo for $link {
            fn mtu(&self) -> u16 {
                MTU as u16
            }

            fn is_streamed(&self) -> bool {
                false
            }
        }
    )*};
}

serial_info!(WasmSerialLink, WasmSerialLinkTx<'_>, WasmSerialLinkRx<'_>);

impl ZLinkTx for WasmSerialLink {
    async fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        self.tx.send(0, buffer).await
    }
}

impl ZLinkTx for WasmSerialLinkTx<'_> {
    async fn write_all(&mut self, buffer: &[u8]) -> Result<(), LinkError> {
        self.0.send(0, buffer).await
    }
}

impl ZLinkRx for WasmSerialLink {
    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        self.rx.read_data(buffer).await
    }

    async fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        self.rx.read_data(buffer).await.map(|_| ())
    }
}

impl ZLinkRx for WasmSerialLinkRx<'_> {
    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, LinkError> {
        self.0.read_data(buffer).await
    }

    async fn read_exact(&mut self, buffer: &mut [u8]) -> Result<(), LinkError> {
        self.0.read_data(buffer).await.map(|_| ())
    }
}

impl ZLink for WasmSerialLink {
    type Tx<'link>
        = WasmSerialLinkTx<'link>
    where
        Self: 'link;

    type Rx<'link>
        = WasmSerialLinkRx<'link>
    where
        Self: 'link;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (
            WasmSerialLinkTx(&mut self.tx),
            WasmSerialLinkRx(&mut self.rx),
        )
    }
}
