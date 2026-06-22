//
// Copyright (c) 2026 Angelo Corsaro
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   Angelo Corsaro, <kydos@protonmail.com>
//
//
//! zenoh-nostd WASM bindings for TypeScript
//!
//! Exposes a minimal, callback-based API compatible with @eclipse-zenoh/zenoh-ts.
//! The TypeScript wrapper (ts/src/) layers the full zenoh-ts channel/iterator API
//! on top of these low-level bindings.

#![no_std]
extern crate alloc;

use alloc::{boxed::Box, collections::BTreeMap, format, rc::Rc, string::String, vec::Vec};
use core::cell::{Cell, UnsafeCell};
use core::time::Duration;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

use zenoh_nostd::session::{
    AllocGetCallbacks, AllocQueryableCallbacks, AllocSubCallbacks, GetResponse, Resources,
    Session, TransportLinkManager, ZSessionConfig,
};
use zenoh_proto::{
    exts::{QoS, QueryTarget},
    fields::{ConsolidationMode, Reliability},
    keyexpr, Endpoint,
};
use zenoh_wasm::{WasmLink, WasmLinkManager};

// ── WasmConfig ──────────────────────────────────────────────────────────────

struct WasmConfig {
    transports: TransportLinkManager<WasmLinkManager>,
}

type BoxStorage = zenoh_nostd::session::zenoh::storage::Box;

impl ZSessionConfig for WasmConfig {
    type LinkManager = WasmLinkManager;
    type Buff = Vec<u8>;
    type SubCallbacks<'r> = AllocSubCallbacks<'r, BoxStorage, BoxStorage>;
    type GetCallbacks<'r> = AllocGetCallbacks<'r, BoxStorage, BoxStorage>;
    type QueryableCallbacks<'r> = AllocQueryableCallbacks<'r, Self, BoxStorage, BoxStorage>;

    fn buff(&self) -> Vec<u8> {
        alloc::vec![0u8; 32767]
    }

    fn transports(&self) -> &TransportLinkManager<WasmLinkManager> {
        &self.transports
    }
}

// ── Multi-session support ────────────────────────────────────────────────────
//
// Up to MAX_SESSIONS independent Zenoh sessions can be open simultaneously.
// Each JsSession holds a `slot` index that identifies which global entry it
// owns.  WASM is single-threaded so there are no data races.

const MAX_SESSIONS: usize = 4;

struct SessionSlot {
    session: Option<&'static Session<'static, WasmConfig>>,
    ws_close_ptr: *const WasmLink,
}

impl SessionSlot {
    const fn empty() -> Self {
        Self {
            session: None,
            ws_close_ptr: core::ptr::null(),
        }
    }
}

struct GlobalSlots([UnsafeCell<SessionSlot>; MAX_SESSIONS]);
// SAFETY: wasm32-unknown-unknown is single-threaded.
unsafe impl Sync for GlobalSlots {}

static G_SLOTS: GlobalSlots = GlobalSlots([
    UnsafeCell::new(SessionSlot::empty()),
    UnsafeCell::new(SessionSlot::empty()),
    UnsafeCell::new(SessionSlot::empty()),
    UnsafeCell::new(SessionSlot::empty()),
]);

fn slot(idx: usize) -> &'static mut SessionSlot {
    // SAFETY: single-threaded WASM, no concurrent mutation.
    unsafe { &mut *G_SLOTS.0[idx].get() }
}

fn find_free_slot() -> Option<usize> {
    for i in 0..MAX_SESSIONS {
        if slot(i).session.is_none() {
            return Some(i);
        }
    }
    None
}

fn require_session(idx: usize) -> Result<&'static Session<'static, WasmConfig>, JsValue> {
    slot(idx)
        .session
        .ok_or_else(|| JsValue::from_str("No open Zenoh session"))
}

struct KeyexprCache(UnsafeCell<BTreeMap<String, &'static keyexpr>>);
// SAFETY: wasm32-unknown-unknown is single-threaded.
unsafe impl Sync for KeyexprCache {}

static G_KEYEXPRS: KeyexprCache = KeyexprCache(UnsafeCell::new(BTreeMap::new()));

// ── Helpers ──────────────────────────────────────────────────────────────────

fn parse_keyexpr<'a>(s: &'a str) -> Result<&'a keyexpr, JsValue> {
    keyexpr::new(s).map_err(|e| JsValue::from_str(&format!("Invalid keyexpr: {e}")))
}

/// Cache key expressions that must outlive the current call.
fn intern_keyexpr(s: &str) -> Result<&'static keyexpr, JsValue> {
    parse_keyexpr(s)?;

    let cache = unsafe { &mut *G_KEYEXPRS.0.get() };
    if let Some(ke) = cache.get(s) {
        return Ok(*ke);
    }

    let owned = String::from(s);
    let leaked: &'static str = Box::leak(owned.clone().into_boxed_str());
    let ke = keyexpr::from_str_unchecked(leaked);
    cache.insert(owned, ke);
    Ok(ke)
}

fn js_err(e: impl core::fmt::Display) -> JsValue {
    JsValue::from_str(&format!("{e}"))
}

/// Build a `QoS` from the optional numeric QoS fields passed by the TS layer.
///
/// `priority`: 1-7 (defaults to 5 = Data). `congestion_control`: 0 = Drop
/// (default), 1 = Block. `express`: defaults to `false`.
fn qos_from(priority: Option<u8>, congestion_control: Option<u8>, express: Option<bool>) -> QoS {
    let priority = priority.unwrap_or(5);
    let block = matches!(congestion_control, Some(1));
    QoS::from_parts(priority, block, express.unwrap_or(false))
}

/// Map the optional numeric reliability passed by the TS layer.
/// 0 = BestEffort, anything else (incl. `None`) = Reliable (the default).
fn reliability_from(reliability: Option<u8>) -> Reliability {
    match reliability {
        Some(0) => Reliability::BestEffort,
        _ => Reliability::Reliable,
    }
}

/// JS-native sleep: uses `globalThis.setTimeout` so it works in both browsers
/// and Deno (where `embassy_time::Timer` cannot be used outside embassy tasks).
async fn js_sleep(ms: u32) {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let global = js_sys::global();
        let set_timeout: js_sys::Function =
            js_sys::Reflect::get(&global, &JsValue::from_str("setTimeout"))
                .unwrap_or(JsValue::UNDEFINED)
                .unchecked_into();
        let _ = set_timeout.call2(
            &JsValue::NULL,
            resolve.unchecked_ref(),
            &JsValue::from(ms),
        );
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

fn sample_to_js(s: &zenoh_nostd::session::Sample<'_>) -> JsSample {
    JsSample {
        key_expr: s.keyexpr().as_str().into(),
        payload: s.payload().to_vec(),
        encoding_id: s.encoding_id() as u32,
        kind: 0,
    }
}

// ── Exported types ──────────────────────────────────────────────────────────

/// Handle to one Zenoh session.  Each open() call creates an independent
/// session backed by its own WebSocket connection to the router.
#[wasm_bindgen]
pub struct JsSession {
    /// Index into G_SLOTS for this session.
    slot: usize,
}

/// Handle returned by declarePublisher.
#[wasm_bindgen]
pub struct JsPublisher {
    slot: usize,
    ke: String,
    /// Kept for future `undeclare()` support (sends InterestFinal with this ID).
    #[allow(dead_code)]
    id: u32,
}

/// Handle returned by declareSubscriber.
#[wasm_bindgen]
pub struct JsSubscriber {
    slot: usize,
    id: u32,
}

/// Handle returned by declareQueryable.
#[wasm_bindgen]
pub struct JsQueryable {
    slot: usize,
    id: u32,
}

/// Handle returned by declareQuerier.
#[wasm_bindgen]
pub struct JsQuerier {
    slot: usize,
    ke: String,
    timeout_ms: u32,
}

/// A received sample (pub/sub data). Cloned fields to avoid lifetime issues.
#[derive(Clone)]
#[wasm_bindgen(getter_with_clone)]
pub struct JsSample {
    pub key_expr: String,
    pub payload: Vec<u8>,
    pub encoding_id: u32,
    /// 0 = Put, 1 = Delete
    pub kind: u8,
}

/// An incoming query (from a remote get/querier). The rid is kept private;
/// reply/finalize methods call into the global session.
#[wasm_bindgen(getter_with_clone)]
pub struct JsQuery {
    pub key_expr: String,
    pub parameters: Option<String>,
    pub payload: Option<Vec<u8>>,
    slot: usize,
    rid: u32,
    finalized: bool,
}

/// A get reply: either Ok(sample) or Err(sample).
#[wasm_bindgen(getter_with_clone)]
pub struct JsReply {
    pub is_ok: bool,
    pub sample: JsSample,
}

// ── JsSession ────────────────────────────────────────────────────────────────

#[wasm_bindgen]
impl JsSession {
    /// Open a session to a Zenoh router.
    ///
    /// `locator` examples: `"ws/127.0.0.1:7447"`, `"ws/192.168.1.1:7447"`
    ///
    /// Each call opens a new, independent WebSocket connection and returns a
    /// unique `JsSession` handle.  Up to 4 sessions may be open concurrently.
    ///
    /// Returns `Promise<JsSession>`.
    pub async fn open(locator: String) -> Result<JsSession, JsValue> {
        console_error_panic_hook::set_once();

        let idx = find_free_slot()
            .ok_or_else(|| JsValue::from_str("Too many open Zenoh sessions (max 4)"))?;

        let config: &'static WasmConfig = Box::leak(Box::new(WasmConfig {
            transports: TransportLinkManager::from(WasmLinkManager),
        }));
        let resources: &'static mut Resources<'static, WasmConfig> =
            Box::leak(Box::new(Resources::default()));

        let endpoint =
            Endpoint::try_from(locator.as_str()).map_err(|e| js_err(e))?;

        let transport = config
            .transports()
            .connect(endpoint, config.buff())
            .await
            .map_err(|e| js_err(e))?;

        let transport_ref = resources.init(transport);
        let link_ptr: *const WasmLink = transport_ref.link();

        let session: &'static Session<'static, WasmConfig> =
            Box::leak(Box::new(Session::new(transport_ref)));

        {
            let s = slot(idx);
            s.session = Some(session);
            s.ws_close_ptr = link_ptr;
        }

        // Drive this session's event loop as a wasm_bindgen_futures microtask.
        spawn_local(async move {
            if let Err(_e) = session.run().await {
                #[cfg(feature = "web_console")]
                web_sys::console::error_1(
                    &"[zenoh-nostd] session run loop exited with error".into(),
                );
            }
        });

        Ok(JsSession { slot: idx })
    }

    /// Close this session.
    ///
    /// Clears the slot and sends a WebSocket close frame.
    pub fn close(&self) {
        let s = slot(self.slot);
        s.session = None;

        let link_ptr = s.ws_close_ptr;
        if !link_ptr.is_null() {
            // SAFETY: pointer points into box-leaked Resources, valid for the
            // process lifetime. Single-threaded WASM.
            unsafe { (*link_ptr).close_ws() };
            s.ws_close_ptr = core::ptr::null();
        }
    }

    // ── Put / Delete ─────────────────────────────────────────────────────────

    /// Publish data to `key_expr`. Returns `Promise<void>`.
    ///
    /// `priority`: 1-7 (5 = Data, the default). `congestion_control`: 0 = Drop
    /// (default), 1 = Block. `express`: defaults to `false`.
    pub async fn put(
        &self,
        key_expr: String,
        payload: Vec<u8>,
        encoding_id: u32,
        attachment: Option<Vec<u8>>,
        priority: Option<u8>,
        congestion_control: Option<u8>,
        express: Option<bool>,
    ) -> Result<(), JsValue> {
        let session = require_session(self.slot)?;
        let ke = parse_keyexpr(&key_expr)?;
        let encoding_id =
            u16::try_from(encoding_id).map_err(|_| JsValue::from_str("Encoding id out of range"))?;

        let mut builder = session
            .put(ke, &payload)
            .encoding(zenoh_proto::fields::Encoding {
                id: encoding_id,
                schema: None,
            })
            .qos(qos_from(priority, congestion_control, express));
        if let Some(ref attachment) = attachment {
            builder = builder.attachment(attachment.as_slice());
        }

        builder.finish().await.map_err(|e| js_err(e))
    }

    /// Delete notifications are not supported yet by zenoh-nostd.
    pub async fn delete(&self, key_expr: String) -> Result<(), JsValue> {
        let _ = key_expr;
        Err(JsValue::from_str(
            "Delete is not yet implemented in zenoh-nostd",
        ))
    }

    // ── Publisher ─────────────────────────────────────────────────────────────

    /// Declare a publisher. Returns `Promise<JsPublisher>`.
    ///
    /// Sends an `Interest(mode=CurrentFuture, options=SUBSCRIBERS, ke=...)` message
    /// to the router so it can set up routing for this publisher's key expression.
    pub async fn declare_publisher(&self, key_expr: String) -> Result<JsPublisher, JsValue> {
        let session = require_session(self.slot)?;
        let ke = intern_keyexpr(&key_expr)?;

        let pub_handle = session
            .declare_publisher(ke)
            .finish()
            .await
            .map_err(|e| js_err(e))?;

        let id = pub_handle.id();
        Ok(JsPublisher { slot: self.slot, ke: key_expr, id })
    }

    // ── Subscriber ────────────────────────────────────────────────────────────

    /// Declare a subscriber. `callback` is called with a `JsSample` for each
    /// received message matching `key_expr`. Returns `Promise<JsSubscriber>`.
    pub async fn declare_subscriber(
        &self,
        key_expr: String,
        callback: js_sys::Function,
    ) -> Result<JsSubscriber, JsValue> {
        let session = require_session(self.slot)?;
        let ke = intern_keyexpr(&key_expr)?;

        let sub = session
            .declare_subscriber(ke)
            .callback_sync(move |sample| {
                let js = JsValue::from(sample_to_js(sample));
                let _ = callback.call1(&JsValue::NULL, &js);
            })
            .finish()
            .await
            .map_err(|e| js_err(e))?;

        let id = sub.id();
        Ok(JsSubscriber { slot: self.slot, id })
    }

    // ── Queryable ─────────────────────────────────────────────────────────────

    /// Declare a queryable. `callback` is called with a `JsQuery` for each
    /// incoming query matching `key_expr`. Returns `Promise<JsQueryable>`.
    pub async fn declare_queryable(
        &self,
        key_expr: String,
        callback: js_sys::Function,
    ) -> Result<JsQueryable, JsValue> {
        let session: &'static Session<'static, WasmConfig> = require_session(self.slot)?;
        let ke = intern_keyexpr(&key_expr)?;
        let slot_idx = self.slot;

        let queryable = session
            .declare_queryable(ke)
            .callback_sync(move |query| {
                let js_query = JsQuery {
                    key_expr: query.keyexpr().as_str().into(),
                    parameters: query.parameters().map(|p| p.into()),
                    payload: query.payload().map(|p| p.to_vec()),
                    slot: slot_idx,
                    rid: query.rid(),
                    finalized: false,
                };
                let js = JsValue::from(js_query);
                let _ = callback.call1(&JsValue::NULL, &js);
            })
            .finish()
            .await
            .map_err(|e| js_err(e))?;

        let id = queryable.id();
        Ok(JsQueryable { slot: self.slot, id })
    }

    // ── Querier ───────────────────────────────────────────────────────────────

    /// Declare a querier for `key_expr` with default `timeout_ms`.
    /// Returns `JsQuerier` synchronously.
    pub fn declare_querier(
        &self,
        key_expr: String,
        timeout_ms: u32,
    ) -> Result<JsQuerier, JsValue> {
        let _ = parse_keyexpr(&key_expr)?;
        Ok(JsQuerier { slot: self.slot, ke: key_expr, timeout_ms })
    }

    // ── Get ───────────────────────────────────────────────────────────────────

    /// Issue a get query. `callback` is called with a `JsReply` for each reply.
    /// The returned `Promise<void>` resolves when ResponseFinal is received
    /// (or after `timeout_ms` milliseconds as a fallback).
    ///
    /// `target`: 0 = BestMatching (default), 1 = All, 2 = AllComplete.
    /// `consolidation`: 0 = Auto (default), 1 = None, 2 = Monotonic, 3 = Latest.
    pub async fn get(
        &self,
        key_expr: String,
        parameters: Option<String>,
        payload: Option<Vec<u8>>,
        callback: js_sys::Function,
        timeout_ms: u32,
        target: Option<u8>,
        consolidation: Option<u8>,
    ) -> Result<(), JsValue> {
        let session = require_session(self.slot)?;
        let ke = intern_keyexpr(&key_expr)?;
        let done = Rc::new(Cell::new(false));

        let query_target = match target.unwrap_or(0) {
            1 => QueryTarget::All,
            2 => QueryTarget::AllComplete,
            _ => QueryTarget::BestMatching,
        };

        let query_consolidation = match consolidation.unwrap_or(0) {
            1 => ConsolidationMode::None,
            2 => ConsolidationMode::Monotonic,
            3 => ConsolidationMode::Latest,
            _ => ConsolidationMode::Auto,
        };

        let mut builder = session
            .get(ke)
            .timeout(Duration::from_millis(timeout_ms as u64))
            .target(query_target)
            .consolidation(query_consolidation);

        if let Some(ref params) = parameters {
            builder = builder.parameters(params.as_str());
        }

        if let Some(ref p) = payload {
            builder = builder.payload(p.as_slice());
        }

        let done_callback = done.clone();
        builder
            .callback_sync(move |reply| {
                match reply {
                    GetResponse::Ok(s) => {
                        let js_reply = JsReply { is_ok: true, sample: sample_to_js(s) };
                        let _ = callback.call1(&JsValue::NULL, &JsValue::from(js_reply));
                    }
                    GetResponse::Err(s) => {
                        let js_reply = JsReply { is_ok: false, sample: sample_to_js(s) };
                        let _ = callback.call1(&JsValue::NULL, &JsValue::from(js_reply));
                    }
                    GetResponse::Done => {
                        done_callback.set(true);
                    }
                }
            })
            .finish()
            .await
            .map_err(|e| js_err(e))?;

        // Poll every 5 ms until Done arrives or the timeout expires.
        const STEP_MS: u32 = 5;
        let mut elapsed = 0u32;
        while elapsed < timeout_ms {
            if done.get() {
                break;
            }
            js_sleep(STEP_MS).await;
            elapsed += STEP_MS;
        }

        Ok(())
    }
}

// ── JsPublisher ──────────────────────────────────────────────────────────────

#[wasm_bindgen]
impl JsPublisher {
    /// Get the key expression this publisher was declared on.
    pub fn key_expr(&self) -> String {
        self.ke.clone()
    }

    /// Publish `payload` to this publisher's key expression.
    ///
    /// QoS (`priority`, `congestion_control`, `express`, `reliability`) is
    /// resolved by the TS layer from the publisher's declared options and
    /// passed through on every put.
    pub async fn put(
        &self,
        payload: Vec<u8>,
        encoding_id: u32,
        attachment: Option<Vec<u8>>,
        priority: Option<u8>,
        congestion_control: Option<u8>,
        express: Option<bool>,
        reliability: Option<u8>,
    ) -> Result<(), JsValue> {
        let session = require_session(self.slot)?;
        let ke = parse_keyexpr(&self.ke)?;
        let encoding_id =
            u16::try_from(encoding_id).map_err(|_| JsValue::from_str("Encoding id out of range"))?;

        let mut builder = session
            .put(ke, &payload)
            .encoding(zenoh_proto::fields::Encoding {
                id: encoding_id,
                schema: None,
            })
            .qos(qos_from(priority, congestion_control, express))
            .reliability(reliability_from(reliability));
        if let Some(ref attachment) = attachment {
            builder = builder.attachment(attachment.as_slice());
        }

        builder.finish().await.map_err(|e| js_err(e))
    }

    /// Delete notifications are not supported yet by zenoh-nostd.
    pub async fn delete(&self) -> Result<(), JsValue> {
        Err(JsValue::from_str(
            "Delete is not yet implemented in zenoh-nostd",
        ))
    }

    /// Undeclare this publisher (no-op; future: send interest cancellation).
    pub fn undeclare(self) {}
}

// ── JsSubscriber ─────────────────────────────────────────────────────────────

#[wasm_bindgen]
impl JsSubscriber {
    /// The numeric ID assigned to this subscriber.
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Undeclare this subscriber.
    pub async fn undeclare(&self) -> Result<(), JsValue> {
        if slot(self.slot).session.is_none() {
            return Ok(());
        }

        let session = require_session(self.slot)?;
        session
            .undeclare_subscriber(self.id)
            .await
            .map_err(|e| js_err(e))
    }
}

// ── JsQueryable ──────────────────────────────────────────────────────────────

#[wasm_bindgen]
impl JsQueryable {
    /// The numeric ID assigned to this queryable.
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Undeclare this queryable.
    pub async fn undeclare(&self) -> Result<(), JsValue> {
        if slot(self.slot).session.is_none() {
            return Ok(());
        }

        let session = require_session(self.slot)?;
        session
            .undeclare_queryable(self.id)
            .await
            .map_err(|e| js_err(e))
    }
}

// ── JsQuerier ────────────────────────────────────────────────────────────────

#[wasm_bindgen]
impl JsQuerier {
    /// Issue a get via this querier. `callback` is called for each reply.
    /// The returned `Promise<void>` resolves when ResponseFinal is received
    /// or after `timeout_ms` (querier default if 0).
    pub async fn get(
        &self,
        callback: js_sys::Function,
        parameters: Option<String>,
        payload: Option<Vec<u8>>,
        timeout_ms: Option<u32>,
    ) -> Result<(), JsValue> {
        let session = require_session(self.slot)?;
        let ke = intern_keyexpr(&self.ke)?;
        let tms = timeout_ms.unwrap_or(self.timeout_ms);
        let done = Rc::new(Cell::new(false));

        let mut builder = session
            .get(ke)
            .timeout(Duration::from_millis(tms as u64));

        if let Some(ref params) = parameters {
            builder = builder.parameters(params.as_str());
        }

        if let Some(ref p) = payload {
            builder = builder.payload(p.as_slice());
        }

        let done_callback = done.clone();
        builder
            .callback_sync(move |reply| {
                match reply {
                    GetResponse::Ok(s) => {
                        let js_reply = JsReply { is_ok: true, sample: sample_to_js(s) };
                        let _ = callback.call1(&JsValue::NULL, &JsValue::from(js_reply));
                    }
                    GetResponse::Err(s) => {
                        let js_reply = JsReply { is_ok: false, sample: sample_to_js(s) };
                        let _ = callback.call1(&JsValue::NULL, &JsValue::from(js_reply));
                    }
                    GetResponse::Done => {
                        done_callback.set(true);
                    }
                }
            })
            .finish()
            .await
            .map_err(|e| js_err(e))?;

        const STEP_MS: u32 = 5;
        let mut elapsed = 0u32;
        while elapsed < tms {
            if done.get() {
                break;
            }
            js_sleep(STEP_MS).await;
            elapsed += STEP_MS;
        }

        Ok(())
    }

    /// Undeclare this querier (no-op; future: send interest cancellation).
    pub fn undeclare(self) {}
}

// ── JsQuery ──────────────────────────────────────────────────────────────────

#[wasm_bindgen]
impl JsQuery {
    /// Send a successful reply to this query.
    pub async fn reply(&self, ke: String, payload: Vec<u8>) -> Result<(), JsValue> {
        let session = require_session(self.slot)?;
        let ke_ref = parse_keyexpr(&ke)?;
        session
            .reply(self.rid, ke_ref, &payload)
            .await
            .map_err(|e| js_err(e))
    }

    /// Send an error reply to this query.
    pub async fn reply_err(&self, payload: Vec<u8>) -> Result<(), JsValue> {
        let session = require_session(self.slot)?;
        let ke_ref = parse_keyexpr(&self.key_expr)?;
        session
            .err(self.rid, ke_ref, &payload)
            .await
            .map_err(|e| js_err(e))
    }

    /// Finalize this query (sends ResponseFinal once all queryables have replied).
    pub async fn finalize(&mut self) -> Result<(), JsValue> {
        if !self.finalized {
            let session = require_session(self.slot)?;
            session
                .finalize(self.rid)
                .await
                .map_err(|e| js_err(e))?;
            self.finalized = true;
        }
        Ok(())
    }
}

// ── JsSample ─────────────────────────────────────────────────────────────────

#[wasm_bindgen]
impl JsSample {
    // Fields are already pub via getter_with_clone; no extra methods needed.
}

// ── JsReply ──────────────────────────────────────────────────────────────────

#[wasm_bindgen]
impl JsReply {
    // Fields are already pub via getter_with_clone.
}

// ── Key-expression utilities ─────────────────────────────────────────────────

/// Returns true if key expression `a` intersects `b` (they could match the same resource).
#[wasm_bindgen]
pub fn ke_intersects(a: &str, b: &str) -> bool {
    match (keyexpr::new(a), keyexpr::new(b)) {
        (Ok(ka), Ok(kb)) => ka.intersects(kb),
        _ => false,
    }
}

/// Returns true if key expression `a` includes all resources matched by `b`.
/// `a includes b` means every key matched by `b` is also matched by `a`.
#[wasm_bindgen]
pub fn ke_includes(a: &str, b: &str) -> bool {
    if keyexpr::new(a).is_err() || keyexpr::new(b).is_err() {
        return false;
    }
    if a == b {
        return true;
    }
    let a_chunks: alloc::vec::Vec<&str> = a.split('/').collect();
    let b_chunks: alloc::vec::Vec<&str> = b.split('/').collect();
    ke_includes_chunks(&a_chunks, &b_chunks)
}

/// Recursive chunk-level inclusion check.
fn ke_includes_chunks(a: &[&str], b: &[&str]) -> bool {
    match a.first() {
        None => b.is_empty(),
        Some(&"**") => {
            ke_includes_chunks(&a[1..], b)
                || (!b.is_empty() && ke_includes_chunks(a, &b[1..]))
        }
        Some(&"*") => match b.first() {
            None => false,
            Some(&"**") => false,
            Some(_) => ke_includes_chunks(&a[1..], &b[1..]),
        },
        Some(ac) => match b.first() {
            None => false,
            Some(bc) if ac == bc => ke_includes_chunks(&a[1..], &b[1..]),
            _ => false,
        },
    }
}
