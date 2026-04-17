//! Browser-native WebSocket link for WASM targets.
#![cfg(target_arch = "wasm32")]
//!
//! Uses `web_sys::WebSocket` directly — no extra WebSocket crate required.
//! Storing `ws: web_sys::WebSocket` as a separate field lets us call
//! `ws.close()` independently of the split sink/stream halves.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures_channel::mpsc::{UnboundedSender, unbounded};
use futures_util::{SinkExt as _, StreamExt as _, stream::{SplitSink, SplitStream}};
use js_sys;
use wasm_bindgen::JsCast as _;
use wasm_bindgen::prelude::*;
use web_sys;

use zenoh_nostd::platform::*;

// ── BrowserWs — Sink + Stream over a browser WebSocket ──────────────────────

/// Combined `Sink<Vec<u8>>` + `Stream<Item = Result<Vec<u8>, ()>>` backed by
/// a `web_sys::WebSocket`. Incoming binary frames are forwarded via an mpsc
/// channel set up in [`WasmWsLink::connect`].
struct BrowserWs {
    ws: web_sys::WebSocket,
    rx: futures_channel::mpsc::UnboundedReceiver<Result<Vec<u8>, ()>>,
}

impl futures_util::Stream for BrowserWs {
    type Item = Result<Vec<u8>, ()>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx).poll_next(cx)
    }
}

impl futures_util::Sink<Vec<u8>> for BrowserWs {
    type Error = ();

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), ()>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Vec<u8>) -> Result<(), ()> {
        self.ws.send_with_u8_array(&item).map_err(|_| ())
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), ()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), ()>> {
        let _ = self.ws.close();
        Poll::Ready(Ok(()))
    }
}

// ── WasmWsLink ───────────────────────────────────────────────────────────────

pub struct WasmWsLink {
    sink: SplitSink<BrowserWs, Vec<u8>>,
    stream: SplitStream<BrowserWs>,
    /// Raw browser WebSocket reference (a clone of the one inside BrowserWs).
    /// Stored separately so `close_ws()` can call `.close()` without touching
    /// the mutably-borrowed `sink`/`stream` fields.
    pub ws: web_sys::WebSocket,
    mtu: u16,
}

impl WasmWsLink {
    /// Connect to a WebSocket server at `url` (e.g. `"ws://127.0.0.1:7447"`).
    pub async fn connect(url: &str) -> core::result::Result<Self, LinkError> {
        // Create the browser WebSocket and configure binary transfer mode.
        let ws =
            web_sys::WebSocket::new(url).map_err(|_| LinkError::CouldNotConnect)?;
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        // Unbounded channel: JS callbacks push frames; the BrowserWs stream pops them.
        let (tx, rx) = unbounded::<Result<Vec<u8>, ()>>();

        install_onmessage(&ws, tx.clone());
        install_onclose(&ws, tx);

        // Wait for the `onopen` event before proceeding (returns error on failure).
        wait_for_open(&ws).await?;

        // Clone ws for the close handle (web_sys types are JsCast wrappers — cheap clone).
        let raw_ws = ws.clone();

        let browser_ws = BrowserWs { ws, rx };
        let (sink, stream) = browser_ws.split();

        Ok(Self {
            sink,
            stream,
            ws: raw_ws,
            mtu: u16::MAX,
        })
    }

    /// Close the underlying browser WebSocket.
    ///
    /// Safe to call while the driver's split `sink`/`stream` are still alive
    /// because `ws` is a separate JS reference — it does not alias those fields.
    pub fn close_ws(&self) {
        let _ = self.ws.close();
    }
}

// ── Helper functions ─────────────────────────────────────────────────────────

fn install_onmessage(ws: &web_sys::WebSocket, tx: UnboundedSender<Result<Vec<u8>, ()>>) {
    let onmessage: Closure<dyn Fn(_)> = Closure::new(move |e: web_sys::MessageEvent| {
        let data = e.data();
        if let Some(buf) = data.dyn_ref::<js_sys::ArrayBuffer>() {
            let bytes = js_sys::Uint8Array::new(buf).to_vec();
            let _ = tx.unbounded_send(Ok(bytes));
        }
    });
    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget(); // Keep the closure alive for the WebSocket's lifetime.
}

fn install_onclose(ws: &web_sys::WebSocket, tx: UnboundedSender<Result<Vec<u8>, ()>>) {
    let onclose: Closure<dyn Fn(_)> = Closure::new(move |e: web_sys::CloseEvent| {
        if !e.was_clean() {
            web_sys::console::warn_1(
                &JsValue::from_str("WebSocket CloseEvent wasClean() == false"),
            );
        }
        // Signal EOF to the session run loop.
        let _ = tx.unbounded_send(Err(()));
    });
    ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
    onclose.forget();
}

async fn wait_for_open(ws: &web_sys::WebSocket) -> core::result::Result<(), LinkError> {
    // Channel capacity 1: first event (onopen=Ok, onerror=Err) wins.
    let (ok_tx, mut rx) = futures_channel::mpsc::channel::<core::result::Result<(), ()>>(1);
    let err_tx = ok_tx.clone();

    let mut open_sender = ok_tx;
    let onopen: Closure<dyn FnMut()> = Closure::new(move || {
        let _ = open_sender.try_send(Ok(()));
    });
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    let mut error_sender = err_tx;
    let onerror: Closure<dyn FnMut(JsValue)> = Closure::new(move |_: JsValue| {
        let _ = error_sender.try_send(Err(()));
    });
    ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    onerror.forget();

    rx.next().await
        .unwrap_or(Err(()))
        .map_err(|()| LinkError::CouldNotConnect)
}

// ── Split halves ─────────────────────────────────────────────────────────────

pub struct WasmWsLinkTx<'a> {
    sink: &'a mut SplitSink<BrowserWs, Vec<u8>>,
    mtu: u16,
}

pub struct WasmWsLinkRx<'a> {
    stream: &'a mut SplitStream<BrowserWs>,
    mtu: u16,
}

impl ZLinkInfo for WasmWsLink {
    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn is_streamed(&self) -> bool {
        false
    }
}

impl ZLinkInfo for WasmWsLinkTx<'_> {
    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn is_streamed(&self) -> bool {
        false
    }
}

impl ZLinkInfo for WasmWsLinkRx<'_> {
    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn is_streamed(&self) -> bool {
        false
    }
}

impl ZLinkTx for WasmWsLink {
    async fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        self.sink
            .send(buffer.to_vec())
            .await
            .map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkTx for WasmWsLinkTx<'_> {
    async fn write_all(&mut self, buffer: &[u8]) -> core::result::Result<(), LinkError> {
        self.sink
            .send(buffer.to_vec())
            .await
            .map_err(|_| LinkError::LinkTxFailed)
    }
}

impl ZLinkRx for WasmWsLink {
    async fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        match self.stream.next().await {
            Some(Ok(bytes)) => {
                let len = bytes.len().min(buffer.len());
                buffer[..len].copy_from_slice(&bytes[..len]);
                Ok(len)
            }
            _ => Err(LinkError::LinkRxFailed),
        }
    }

    async fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        self.read(buffer).await.map(|_| ())
    }
}

impl ZLinkRx for WasmWsLinkRx<'_> {
    async fn read(&mut self, buffer: &mut [u8]) -> core::result::Result<usize, LinkError> {
        match self.stream.next().await {
            Some(Ok(bytes)) => {
                let len = bytes.len().min(buffer.len());
                buffer[..len].copy_from_slice(&bytes[..len]);
                Ok(len)
            }
            _ => Err(LinkError::LinkRxFailed),
        }
    }

    async fn read_exact(&mut self, buffer: &mut [u8]) -> core::result::Result<(), LinkError> {
        self.read(buffer).await.map(|_| ())
    }
}

impl ZLink for WasmWsLink {
    type Tx<'link>
        = WasmWsLinkTx<'link>
    where
        Self: 'link;

    type Rx<'link>
        = WasmWsLinkRx<'link>
    where
        Self: 'link;

    fn split(&mut self) -> (Self::Tx<'_>, Self::Rx<'_>) {
        (
            WasmWsLinkTx { sink: &mut self.sink, mtu: self.mtu },
            WasmWsLinkRx { stream: &mut self.stream, mtu: self.mtu },
        )
    }
}
