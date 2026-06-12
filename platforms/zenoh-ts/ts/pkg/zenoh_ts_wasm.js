const __wasm_pending_timers = new Set();
export function cancelAllWasmTimers() {
    for (const id of __wasm_pending_timers) { clearTimeout(id); }
    __wasm_pending_timers.clear();
}
/* @ts-self-types="./zenoh_ts_wasm.d.ts" */

/**
 * Handle returned by declarePublisher.
 */
export class JsPublisher {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(JsPublisher.prototype);
        obj.__wbg_ptr = ptr;
        JsPublisherFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        JsPublisherFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_jspublisher_free(ptr, 0);
    }
    /**
     * Delete notifications are not supported yet by zenoh-nostd.
     * @returns {Promise<void>}
     */
    delete() {
        const ret = wasm.jspublisher_delete(this.__wbg_ptr);
        return ret;
    }
    /**
     * Get the key expression this publisher was declared on.
     * @returns {string}
     */
    key_expr() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.jspublisher_key_expr(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Publish `payload` to this publisher's key expression.
     *
     * QoS (`priority`, `congestion_control`, `express`, `reliability`) is
     * resolved by the TS layer from the publisher's declared options and
     * passed through on every put.
     * @param {Uint8Array} payload
     * @param {number} encoding_id
     * @param {Uint8Array | null} [attachment]
     * @param {number | null} [priority]
     * @param {number | null} [congestion_control]
     * @param {boolean | null} [express]
     * @param {number | null} [reliability]
     * @returns {Promise<void>}
     */
    put(payload, encoding_id, attachment, priority, congestion_control, express, reliability) {
        const ptr0 = passArray8ToWasm0(payload, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(attachment) ? 0 : passArray8ToWasm0(attachment, wasm.__wbindgen_malloc);
        var len1 = WASM_VECTOR_LEN;
        const ret = wasm.jspublisher_put(this.__wbg_ptr, ptr0, len0, encoding_id, ptr1, len1, isLikeNone(priority) ? 0xFFFFFF : priority, isLikeNone(congestion_control) ? 0xFFFFFF : congestion_control, isLikeNone(express) ? 0xFFFFFF : express ? 1 : 0, isLikeNone(reliability) ? 0xFFFFFF : reliability);
        return ret;
    }
    /**
     * Undeclare this publisher (no-op; future: send interest cancellation).
     */
    undeclare() {
        const ptr = this.__destroy_into_raw();
        wasm.jspublisher_undeclare(ptr);
    }
}
if (Symbol.dispose) JsPublisher.prototype[Symbol.dispose] = JsPublisher.prototype.free;

/**
 * Handle returned by declareQuerier.
 */
export class JsQuerier {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(JsQuerier.prototype);
        obj.__wbg_ptr = ptr;
        JsQuerierFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        JsQuerierFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_jsquerier_free(ptr, 0);
    }
    /**
     * Issue a get via this querier. `callback` is called for each reply.
     * The returned `Promise<void>` resolves when ResponseFinal is received
     * or after `timeout_ms` (querier default if 0).
     * @param {Function} callback
     * @param {string | null} [parameters]
     * @param {Uint8Array | null} [payload]
     * @param {number | null} [timeout_ms]
     * @returns {Promise<void>}
     */
    get(callback, parameters, payload, timeout_ms) {
        var ptr0 = isLikeNone(parameters) ? 0 : passStringToWasm0(parameters, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(payload) ? 0 : passArray8ToWasm0(payload, wasm.__wbindgen_malloc);
        var len1 = WASM_VECTOR_LEN;
        const ret = wasm.jsquerier_get(this.__wbg_ptr, callback, ptr0, len0, ptr1, len1, isLikeNone(timeout_ms) ? 0x100000001 : (timeout_ms) >>> 0);
        return ret;
    }
    /**
     * Undeclare this querier (no-op; future: send interest cancellation).
     */
    undeclare() {
        const ptr = this.__destroy_into_raw();
        wasm.jsquerier_undeclare(ptr);
    }
}
if (Symbol.dispose) JsQuerier.prototype[Symbol.dispose] = JsQuerier.prototype.free;

/**
 * An incoming query (from a remote get/querier). The rid is kept private;
 * reply/finalize methods call into the global session.
 */
export class JsQuery {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(JsQuery.prototype);
        obj.__wbg_ptr = ptr;
        JsQueryFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        JsQueryFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_jsquery_free(ptr, 0);
    }
    /**
     * @returns {string}
     */
    get key_expr() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.__wbg_get_jsquery_key_expr(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {string | undefined}
     */
    get parameters() {
        const ret = wasm.__wbg_get_jsquery_parameters(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getStringFromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * @returns {Uint8Array | undefined}
     */
    get payload() {
        const ret = wasm.__wbg_get_jsquery_payload(this.__wbg_ptr);
        let v1;
        if (ret[0] !== 0) {
            v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        }
        return v1;
    }
    /**
     * Finalize this query (sends ResponseFinal once all queryables have replied).
     * @returns {Promise<void>}
     */
    finalize() {
        const ret = wasm.jsquery_finalize(this.__wbg_ptr);
        return ret;
    }
    /**
     * Send a successful reply to this query.
     * @param {string} ke
     * @param {Uint8Array} payload
     * @returns {Promise<void>}
     */
    reply(ke, payload) {
        const ptr0 = passStringToWasm0(ke, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArray8ToWasm0(payload, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.jsquery_reply(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        return ret;
    }
    /**
     * Send an error reply to this query.
     * @param {Uint8Array} payload
     * @returns {Promise<void>}
     */
    reply_err(payload) {
        const ptr0 = passArray8ToWasm0(payload, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.jsquery_reply_err(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * @param {string} arg0
     */
    set key_expr(arg0) {
        const ptr0 = passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_jsquery_key_expr(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * @param {string | null} [arg0]
     */
    set parameters(arg0) {
        var ptr0 = isLikeNone(arg0) ? 0 : passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_jsquery_parameters(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * @param {Uint8Array | null} [arg0]
     */
    set payload(arg0) {
        var ptr0 = isLikeNone(arg0) ? 0 : passArray8ToWasm0(arg0, wasm.__wbindgen_malloc);
        var len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_jsquery_payload(this.__wbg_ptr, ptr0, len0);
    }
}
if (Symbol.dispose) JsQuery.prototype[Symbol.dispose] = JsQuery.prototype.free;

/**
 * Handle returned by declareQueryable.
 */
export class JsQueryable {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(JsQueryable.prototype);
        obj.__wbg_ptr = ptr;
        JsQueryableFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        JsQueryableFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_jsqueryable_free(ptr, 0);
    }
    /**
     * The numeric ID assigned to this queryable.
     * @returns {number}
     */
    id() {
        const ret = wasm.jsqueryable_id(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Undeclare this queryable.
     * @returns {Promise<void>}
     */
    undeclare() {
        const ret = wasm.jsqueryable_undeclare(this.__wbg_ptr);
        return ret;
    }
}
if (Symbol.dispose) JsQueryable.prototype[Symbol.dispose] = JsQueryable.prototype.free;

/**
 * A get reply: either Ok(sample) or Err(sample).
 */
export class JsReply {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(JsReply.prototype);
        obj.__wbg_ptr = ptr;
        JsReplyFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        JsReplyFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_jsreply_free(ptr, 0);
    }
    /**
     * @returns {boolean}
     */
    get is_ok() {
        const ret = wasm.__wbg_get_jsreply_is_ok(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @returns {JsSample}
     */
    get sample() {
        const ret = wasm.__wbg_get_jsreply_sample(this.__wbg_ptr);
        return JsSample.__wrap(ret);
    }
    /**
     * @param {boolean} arg0
     */
    set is_ok(arg0) {
        wasm.__wbg_set_jsreply_is_ok(this.__wbg_ptr, arg0);
    }
    /**
     * @param {JsSample} arg0
     */
    set sample(arg0) {
        _assertClass(arg0, JsSample);
        var ptr0 = arg0.__destroy_into_raw();
        wasm.__wbg_set_jsreply_sample(this.__wbg_ptr, ptr0);
    }
}
if (Symbol.dispose) JsReply.prototype[Symbol.dispose] = JsReply.prototype.free;

/**
 * A received sample (pub/sub data). Cloned fields to avoid lifetime issues.
 */
export class JsSample {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(JsSample.prototype);
        obj.__wbg_ptr = ptr;
        JsSampleFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        JsSampleFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_jssample_free(ptr, 0);
    }
    /**
     * @returns {number}
     */
    get encoding_id() {
        const ret = wasm.__wbg_get_jssample_encoding_id(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {string}
     */
    get key_expr() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.__wbg_get_jssample_key_expr(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * 0 = Put, 1 = Delete
     * @returns {number}
     */
    get kind() {
        const ret = wasm.__wbg_get_jssample_kind(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {Uint8Array}
     */
    get payload() {
        const ret = wasm.__wbg_get_jssample_payload(this.__wbg_ptr);
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * @param {number} arg0
     */
    set encoding_id(arg0) {
        wasm.__wbg_set_jssample_encoding_id(this.__wbg_ptr, arg0);
    }
    /**
     * @param {string} arg0
     */
    set key_expr(arg0) {
        const ptr0 = passStringToWasm0(arg0, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_jssample_key_expr(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * 0 = Put, 1 = Delete
     * @param {number} arg0
     */
    set kind(arg0) {
        wasm.__wbg_set_jssample_kind(this.__wbg_ptr, arg0);
    }
    /**
     * @param {Uint8Array} arg0
     */
    set payload(arg0) {
        const ptr0 = passArray8ToWasm0(arg0, wasm.__wbindgen_malloc);
        const len0 = WASM_VECTOR_LEN;
        wasm.__wbg_set_jssample_payload(this.__wbg_ptr, ptr0, len0);
    }
}
if (Symbol.dispose) JsSample.prototype[Symbol.dispose] = JsSample.prototype.free;

/**
 * Handle to one Zenoh session.  Each open() call creates an independent
 * session backed by its own WebSocket connection to the router.
 */
export class JsSession {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(JsSession.prototype);
        obj.__wbg_ptr = ptr;
        JsSessionFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        JsSessionFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_jssession_free(ptr, 0);
    }
    /**
     * Close this session.
     *
     * Clears the slot and sends a WebSocket close frame.
     */
    close() {
        wasm.jssession_close(this.__wbg_ptr);
    }
    /**
     * Declare a publisher. Returns `Promise<JsPublisher>`.
     *
     * Sends an `Interest(mode=CurrentFuture, options=SUBSCRIBERS, ke=...)` message
     * to the router so it can set up routing for this publisher's key expression.
     * @param {string} key_expr
     * @returns {Promise<JsPublisher>}
     */
    declare_publisher(key_expr) {
        const ptr0 = passStringToWasm0(key_expr, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.jssession_declare_publisher(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Declare a querier for `key_expr` with default `timeout_ms`.
     * Returns `JsQuerier` synchronously.
     * @param {string} key_expr
     * @param {number} timeout_ms
     * @returns {JsQuerier}
     */
    declare_querier(key_expr, timeout_ms) {
        const ptr0 = passStringToWasm0(key_expr, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.jssession_declare_querier(this.__wbg_ptr, ptr0, len0, timeout_ms);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return JsQuerier.__wrap(ret[0]);
    }
    /**
     * Declare a queryable. `callback` is called with a `JsQuery` for each
     * incoming query matching `key_expr`. Returns `Promise<JsQueryable>`.
     * @param {string} key_expr
     * @param {Function} callback
     * @returns {Promise<JsQueryable>}
     */
    declare_queryable(key_expr, callback) {
        const ptr0 = passStringToWasm0(key_expr, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.jssession_declare_queryable(this.__wbg_ptr, ptr0, len0, callback);
        return ret;
    }
    /**
     * Declare a subscriber. `callback` is called with a `JsSample` for each
     * received message matching `key_expr`. Returns `Promise<JsSubscriber>`.
     * @param {string} key_expr
     * @param {Function} callback
     * @returns {Promise<JsSubscriber>}
     */
    declare_subscriber(key_expr, callback) {
        const ptr0 = passStringToWasm0(key_expr, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.jssession_declare_subscriber(this.__wbg_ptr, ptr0, len0, callback);
        return ret;
    }
    /**
     * Delete notifications are not supported yet by zenoh-nostd.
     * @param {string} key_expr
     * @returns {Promise<void>}
     */
    delete(key_expr) {
        const ptr0 = passStringToWasm0(key_expr, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.jssession_delete(this.__wbg_ptr, ptr0, len0);
        return ret;
    }
    /**
     * Issue a get query. `callback` is called with a `JsReply` for each reply.
     * The returned `Promise<void>` resolves when ResponseFinal is received
     * (or after `timeout_ms` milliseconds as a fallback).
     *
     * `target`: 0 = BestMatching (default), 1 = All, 2 = AllComplete.
     * `consolidation`: 0 = Auto (default), 1 = None, 2 = Monotonic, 3 = Latest.
     * @param {string} key_expr
     * @param {string | null | undefined} parameters
     * @param {Uint8Array | null | undefined} payload
     * @param {Function} callback
     * @param {number} timeout_ms
     * @param {number | null} [target]
     * @param {number | null} [consolidation]
     * @returns {Promise<void>}
     */
    get(key_expr, parameters, payload, callback, timeout_ms, target, consolidation) {
        const ptr0 = passStringToWasm0(key_expr, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        var ptr1 = isLikeNone(parameters) ? 0 : passStringToWasm0(parameters, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(payload) ? 0 : passArray8ToWasm0(payload, wasm.__wbindgen_malloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.jssession_get(this.__wbg_ptr, ptr0, len0, ptr1, len1, ptr2, len2, callback, timeout_ms, isLikeNone(target) ? 0xFFFFFF : target, isLikeNone(consolidation) ? 0xFFFFFF : consolidation);
        return ret;
    }
    /**
     * Open a session to a Zenoh router.
     *
     * `locator` examples: `"ws/127.0.0.1:7447"`, `"ws/192.168.1.1:7447"`
     *
     * Each call opens a new, independent WebSocket connection and returns a
     * unique `JsSession` handle.  Up to 4 sessions may be open concurrently.
     *
     * Returns `Promise<JsSession>`.
     * @param {string} locator
     * @returns {Promise<JsSession>}
     */
    static open(locator) {
        const ptr0 = passStringToWasm0(locator, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.jssession_open(ptr0, len0);
        return ret;
    }
    /**
     * Publish data to `key_expr`. Returns `Promise<void>`.
     *
     * `priority`: 1-7 (5 = Data, the default). `congestion_control`: 0 = Drop
     * (default), 1 = Block. `express`: defaults to `false`.
     * @param {string} key_expr
     * @param {Uint8Array} payload
     * @param {number} encoding_id
     * @param {Uint8Array | null} [attachment]
     * @param {number | null} [priority]
     * @param {number | null} [congestion_control]
     * @param {boolean | null} [express]
     * @returns {Promise<void>}
     */
    put(key_expr, payload, encoding_id, attachment, priority, congestion_control, express) {
        const ptr0 = passStringToWasm0(key_expr, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passArray8ToWasm0(payload, wasm.__wbindgen_malloc);
        const len1 = WASM_VECTOR_LEN;
        var ptr2 = isLikeNone(attachment) ? 0 : passArray8ToWasm0(attachment, wasm.__wbindgen_malloc);
        var len2 = WASM_VECTOR_LEN;
        const ret = wasm.jssession_put(this.__wbg_ptr, ptr0, len0, ptr1, len1, encoding_id, ptr2, len2, isLikeNone(priority) ? 0xFFFFFF : priority, isLikeNone(congestion_control) ? 0xFFFFFF : congestion_control, isLikeNone(express) ? 0xFFFFFF : express ? 1 : 0);
        return ret;
    }
}
if (Symbol.dispose) JsSession.prototype[Symbol.dispose] = JsSession.prototype.free;

/**
 * Handle returned by declareSubscriber.
 */
export class JsSubscriber {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(JsSubscriber.prototype);
        obj.__wbg_ptr = ptr;
        JsSubscriberFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        JsSubscriberFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_jssubscriber_free(ptr, 0);
    }
    /**
     * The numeric ID assigned to this subscriber.
     * @returns {number}
     */
    id() {
        const ret = wasm.jssubscriber_id(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Undeclare this subscriber.
     * @returns {Promise<void>}
     */
    undeclare() {
        const ret = wasm.jssubscriber_undeclare(this.__wbg_ptr);
        return ret;
    }
}
if (Symbol.dispose) JsSubscriber.prototype[Symbol.dispose] = JsSubscriber.prototype.free;

/**
 * Returns true if key expression `a` includes all resources matched by `b`.
 * `a includes b` means every key matched by `b` is also matched by `a`.
 * @param {string} a
 * @param {string} b
 * @returns {boolean}
 */
export function ke_includes(a, b) {
    const ptr0 = passStringToWasm0(a, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(b, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.ke_includes(ptr0, len0, ptr1, len1);
    return ret !== 0;
}

/**
 * Returns true if key expression `a` intersects `b` (they could match the same resource).
 * @param {string} a
 * @param {string} b
 * @returns {boolean}
 */
export function ke_intersects(a, b) {
    const ptr0 = passStringToWasm0(a, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(b, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.ke_intersects(ptr0, len0, ptr1, len1);
    return ret !== 0;
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_is_function_3baa9db1a987f47d: function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_object_63322ec0cd6ea4ef: function(arg0) {
            const val = arg0;
            const ret = typeof(val) === 'object' && val !== null;
            return ret;
        },
        __wbg___wbindgen_is_string_6df3bf7ef1164ed3: function(arg0) {
            const ret = typeof(arg0) === 'string';
            return ret;
        },
        __wbg___wbindgen_is_undefined_29a43b4d42920abd: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_throw_6b64449b9b9ed33c: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg__wbg_cb_unref_b46c9b5a9f08ec37: function(arg0) {
            arg0._wbg_cb_unref();
        },
        __wbg_call_a24592a6f349a97e: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.call(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_call_bb28efe6b2f55b86: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg0.call(arg1, arg2, arg3);
            return ret;
        }, arguments); },
        __wbg_clearTimeout_b386a9dd32f7e5e5: function(arg0) {
            __wasm_pending_timers.delete(arg0);
            clearTimeout(arg0);
        },
        __wbg_close_88106990eea7f544: function() { return handleError(function (arg0) {
            arg0.close();
        }, arguments); },
        __wbg_crypto_38df2bab126b63dc: function(arg0) {
            const ret = arg0.crypto;
            return ret;
        },
        __wbg_data_bb9dffdd1e99cf2d: function(arg0) {
            const ret = arg0.data;
            return ret;
        },
        __wbg_debug_c014a160490283dc: function(arg0) {
            console.debug(arg0);
        },
        __wbg_error_2001591ad2463697: function(arg0) {
            console.error(arg0);
        },
        __wbg_error_a6fa202b58aa1cd3: function(arg0, arg1) {
            let deferred0_0;
            let deferred0_1;
            try {
                deferred0_0 = arg0;
                deferred0_1 = arg1;
                console.error(getStringFromWasm0(arg0, arg1));
            } finally {
                wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            }
        },
        __wbg_getRandomValues_c44a50d8cfdaebeb: function() { return handleError(function (arg0, arg1) {
            arg0.getRandomValues(arg1);
        }, arguments); },
        __wbg_get_6011fa3a58f61074: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_instanceof_ArrayBuffer_7c8433c6ed14ffe3: function(arg0) {
            let result;
            try {
                result = arg0 instanceof ArrayBuffer;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Window_cc64c86c8ef9e02b: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_jspublisher_new: function(arg0) {
            const ret = JsPublisher.__wrap(arg0);
            return ret;
        },
        __wbg_jsquery_new: function(arg0) {
            const ret = JsQuery.__wrap(arg0);
            return ret;
        },
        __wbg_jsqueryable_new: function(arg0) {
            const ret = JsQueryable.__wrap(arg0);
            return ret;
        },
        __wbg_jsreply_new: function(arg0) {
            const ret = JsReply.__wrap(arg0);
            return ret;
        },
        __wbg_jssample_new: function(arg0) {
            const ret = JsSample.__wrap(arg0);
            return ret;
        },
        __wbg_jssession_new: function(arg0) {
            const ret = JsSession.__wrap(arg0);
            return ret;
        },
        __wbg_jssubscriber_new: function(arg0) {
            const ret = JsSubscriber.__wrap(arg0);
            return ret;
        },
        __wbg_length_9f1775224cf1d815: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_msCrypto_bd5a034af96bcba6: function(arg0) {
            const ret = arg0.msCrypto;
            return ret;
        },
        __wbg_new_036bd6cd9cea9e73: function(arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return wasm_bindgen__convert__closures_____invoke__h6488d5db925c6877(a, state0.b, arg0, arg1);
                    } finally {
                        state0.a = a;
                    }
                };
                const ret = new Promise(cb0);
                return ret;
            } finally {
                state0.a = 0;
            }
        },
        __wbg_new_0c7403db6e782f19: function(arg0) {
            const ret = new Uint8Array(arg0);
            return ret;
        },
        __wbg_new_227d7c05414eb861: function() {
            const ret = new Error();
            return ret;
        },
        __wbg_new_2a6e9133304ae2bf: function() { return handleError(function (arg0, arg1) {
            const ret = new WebSocket(getStringFromWasm0(arg0, arg1));
            return ret;
        }, arguments); },
        __wbg_new_typed_323f37fd55ab048d: function(arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return wasm_bindgen__convert__closures_____invoke__h6488d5db925c6877(a, state0.b, arg0, arg1);
                    } finally {
                        state0.a = a;
                    }
                };
                const ret = new Promise(cb0);
                return ret;
            } finally {
                state0.a = 0;
            }
        },
        __wbg_new_with_length_8c854e41ea4dae9b: function(arg0) {
            const ret = new Uint8Array(arg0 >>> 0);
            return ret;
        },
        __wbg_node_84ea875411254db1: function(arg0) {
            const ret = arg0.node;
            return ret;
        },
        __wbg_now_36a3148ac47c4ad7: function(arg0) {
            const ret = arg0.now();
            return ret;
        },
        __wbg_performance_e0409977f06d6f6b: function(arg0) {
            const ret = arg0.performance;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_process_44c7a14e11e9f69e: function(arg0) {
            const ret = arg0.process;
            return ret;
        },
        __wbg_prototypesetcall_a6b02eb00b0f4ce2: function(arg0, arg1, arg2) {
            Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), arg2);
        },
        __wbg_queueMicrotask_5d15a957e6aa920e: function(arg0) {
            queueMicrotask(arg0);
        },
        __wbg_queueMicrotask_f8819e5ffc402f36: function(arg0) {
            const ret = arg0.queueMicrotask;
            return ret;
        },
        __wbg_randomFillSync_6c25eac9869eb53c: function() { return handleError(function (arg0, arg1) {
            arg0.randomFillSync(arg1);
        }, arguments); },
        __wbg_require_b4edbdcf3e2a1ef0: function() { return handleError(function () {
            const ret = module.require;
            return ret;
        }, arguments); },
        __wbg_resolve_e6c466bc1052f16c: function(arg0) {
            const ret = Promise.resolve(arg0);
            return ret;
        },
        __wbg_send_186c85704c7f2d00: function() { return handleError(function (arg0, arg1, arg2) {
            arg0.send(getArrayU8FromWasm0(arg1, arg2));
        }, arguments); },
        __wbg_setTimeout_007e3640d2f99831: function(arg0, arg1) {
            const ret = setTimeout(arg0, arg1 >>> 0);
            __wasm_pending_timers.add(ret);
            return ret;
        },
        __wbg_set_binaryType_770e68648ca5e83d: function(arg0, arg1) {
            arg0.binaryType = __wbindgen_enum_BinaryType[arg1];
        },
        __wbg_set_onclose_17fa3bbcc4ba3541: function(arg0, arg1) {
            arg0.onclose = arg1;
        },
        __wbg_set_onerror_da99c4232662a084: function(arg0, arg1) {
            arg0.onerror = arg1;
        },
        __wbg_set_onmessage_c1db358b9c38e3f1: function(arg0, arg1) {
            arg0.onmessage = arg1;
        },
        __wbg_set_onopen_cd47b8fb1d92dee9: function(arg0, arg1) {
            arg0.onopen = arg1;
        },
        __wbg_stack_3b0d974bbf31e44f: function(arg0, arg1) {
            const ret = arg1.stack;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_static_accessor_GLOBAL_8cfadc87a297ca02: function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_THIS_602256ae5c8f42cf: function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_e445c1c7484aecc3: function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_WINDOW_f20e8576ef1e0f17: function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_subarray_f8ca46a25b1f5e0d: function(arg0, arg1, arg2) {
            const ret = arg0.subarray(arg1 >>> 0, arg2 >>> 0);
            return ret;
        },
        __wbg_then_792e0c862b060889: function(arg0, arg1, arg2) {
            const ret = arg0.then(arg1, arg2);
            return ret;
        },
        __wbg_then_8e16ee11f05e4827: function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_versions_276b2795b1c6a219: function(arg0) {
            const ret = arg0.versions;
            return ret;
        },
        __wbg_warn_3cc416af27dbdc02: function(arg0) {
            console.warn(arg0);
        },
        __wbg_wasClean_bd109e45fffa711a: function(arg0) {
            const ret = arg0.wasClean;
            return ret;
        },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 104, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h96308b8e6da1fb31);
            return ret;
        },
        __wbindgen_cast_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 152, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h3a025749c5eccc02);
            return ret;
        },
        __wbindgen_cast_0000000000000003: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("CloseEvent")], shim_idx: 126, ret: Unit, inner_ret: Some(Unit) }, mutable: false }) -> Externref`.
            const ret = makeClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h59e376138512cd76);
            return ret;
        },
        __wbindgen_cast_0000000000000004: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("MessageEvent")], shim_idx: 126, ret: Unit, inner_ret: Some(Unit) }, mutable: false }) -> Externref`.
            const ret = makeClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h59e376138512cd76_3);
            return ret;
        },
        __wbindgen_cast_0000000000000005: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [], shim_idx: 138, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h5b046f0c45fb325d);
            return ret;
        },
        __wbindgen_cast_0000000000000006: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000007: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(U8)) -> NamedExternref("Uint8Array")`.
            const ret = getArrayU8FromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_cast_0000000000000008: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./zenoh_ts_wasm_bg.js": import0,
    };
}

function wasm_bindgen__convert__closures_____invoke__h5b046f0c45fb325d(arg0, arg1) {
    wasm.wasm_bindgen__convert__closures_____invoke__h5b046f0c45fb325d(arg0, arg1);
}

function wasm_bindgen__convert__closures_____invoke__h96308b8e6da1fb31(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h96308b8e6da1fb31(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__h59e376138512cd76(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h59e376138512cd76(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__h59e376138512cd76_3(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h59e376138512cd76_3(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__h3a025749c5eccc02(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen__convert__closures_____invoke__h3a025749c5eccc02(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen__convert__closures_____invoke__h6488d5db925c6877(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen__convert__closures_____invoke__h6488d5db925c6877(arg0, arg1, arg2, arg3);
}


const __wbindgen_enum_BinaryType = ["blob", "arraybuffer"];
const JsPublisherFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_jspublisher_free(ptr >>> 0, 1));
const JsQuerierFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_jsquerier_free(ptr >>> 0, 1));
const JsQueryFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_jsquery_free(ptr >>> 0, 1));
const JsQueryableFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_jsqueryable_free(ptr >>> 0, 1));
const JsReplyFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_jsreply_free(ptr >>> 0, 1));
const JsSampleFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_jssample_free(ptr >>> 0, 1));
const JsSessionFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_jssession_free(ptr >>> 0, 1));
const JsSubscriberFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_jssubscriber_free(ptr >>> 0, 1));

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => wasm.__wbindgen_destroy_closure(state.a, state.b));

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function makeClosure(arg0, arg1, f) {
    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        try {
            return f(state.a, state.b, ...args);
        } finally {
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            wasm.__wbindgen_destroy_closure(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function makeMutClosure(arg0, arg1, f) {
    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            state.a = a;
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            wasm.__wbindgen_destroy_closure(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function passArray8ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 1, 1) >>> 0;
    getUint8ArrayMemory0().set(arg, ptr / 1);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasm;
function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('zenoh_ts_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
