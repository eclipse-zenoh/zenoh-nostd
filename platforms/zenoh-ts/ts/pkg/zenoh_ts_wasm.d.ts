/* tslint:disable */
/* eslint-disable */

/**
 * Handle returned by declarePublisher.
 */
export class JsPublisher {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Delete notifications are not supported yet by zenoh-nostd.
     */
    delete(): Promise<void>;
    /**
     * Get the key expression this publisher was declared on.
     */
    key_expr(): string;
    /**
     * Publish `payload` to this publisher's key expression.
     */
    put(payload: Uint8Array, encoding_id: number, attachment?: Uint8Array | null): Promise<void>;
    /**
     * Undeclare this publisher (no-op; future: send interest cancellation).
     */
    undeclare(): void;
}

/**
 * Handle returned by declareQuerier.
 */
export class JsQuerier {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Issue a get via this querier. `callback` is called for each reply.
     * The returned `Promise<void>` resolves when ResponseFinal is received
     * or after `timeout_ms` (querier default if 0).
     */
    get(callback: Function, parameters?: string | null, payload?: Uint8Array | null, timeout_ms?: number | null): Promise<void>;
    /**
     * Undeclare this querier (no-op; future: send interest cancellation).
     */
    undeclare(): void;
}

/**
 * An incoming query (from a remote get/querier). The rid is kept private;
 * reply/finalize methods call into the global session.
 */
export class JsQuery {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Finalize this query (sends ResponseFinal once all queryables have replied).
     */
    finalize(): Promise<void>;
    /**
     * Send a successful reply to this query.
     */
    reply(ke: string, payload: Uint8Array): Promise<void>;
    /**
     * Send an error reply to this query.
     */
    reply_err(payload: Uint8Array): Promise<void>;
    key_expr: string;
    get parameters(): string | undefined;
    set parameters(value: string | null | undefined);
    get payload(): Uint8Array | undefined;
    set payload(value: Uint8Array | null | undefined);
}

/**
 * Handle returned by declareQueryable.
 */
export class JsQueryable {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The numeric ID assigned to this queryable.
     */
    id(): number;
    /**
     * Undeclare this queryable.
     */
    undeclare(): Promise<void>;
}

/**
 * A get reply: either Ok(sample) or Err(sample).
 */
export class JsReply {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    is_ok: boolean;
    sample: JsSample;
}

/**
 * A received sample (pub/sub data). Cloned fields to avoid lifetime issues.
 */
export class JsSample {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    encoding_id: number;
    key_expr: string;
    /**
     * 0 = Put, 1 = Delete
     */
    kind: number;
    payload: Uint8Array;
}

/**
 * Handle to one Zenoh session.  Each open() call creates an independent
 * session backed by its own WebSocket connection to the router.
 */
export class JsSession {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Close this session.
     *
     * Clears the slot and sends a WebSocket close frame.
     */
    close(): void;
    /**
     * Declare a publisher. Returns `Promise<JsPublisher>`.
     *
     * Sends an `Interest(mode=CurrentFuture, options=SUBSCRIBERS, ke=...)` message
     * to the router so it can set up routing for this publisher's key expression.
     */
    declare_publisher(key_expr: string): Promise<JsPublisher>;
    /**
     * Declare a querier for `key_expr` with default `timeout_ms`.
     * Returns `JsQuerier` synchronously.
     */
    declare_querier(key_expr: string, timeout_ms: number): JsQuerier;
    /**
     * Declare a queryable. `callback` is called with a `JsQuery` for each
     * incoming query matching `key_expr`. Returns `Promise<JsQueryable>`.
     */
    declare_queryable(key_expr: string, callback: Function): Promise<JsQueryable>;
    /**
     * Declare a subscriber. `callback` is called with a `JsSample` for each
     * received message matching `key_expr`. Returns `Promise<JsSubscriber>`.
     */
    declare_subscriber(key_expr: string, callback: Function): Promise<JsSubscriber>;
    /**
     * Delete notifications are not supported yet by zenoh-nostd.
     */
    delete(key_expr: string): Promise<void>;
    /**
     * Issue a get query. `callback` is called with a `JsReply` for each reply.
     * The returned `Promise<void>` resolves when ResponseFinal is received
     * (or after `timeout_ms` milliseconds as a fallback).
     *
     * `target`: 0 = BestMatching (default), 1 = All, 2 = AllComplete.
     * `consolidation`: 0 = Auto (default), 1 = None, 2 = Monotonic, 3 = Latest.
     */
    get(key_expr: string, parameters: string | null | undefined, payload: Uint8Array | null | undefined, callback: Function, timeout_ms: number, target?: number | null, consolidation?: number | null): Promise<void>;
    /**
     * Open a session to a Zenoh router.
     *
     * `locator` examples: `"ws/127.0.0.1:7447"`, `"ws/192.168.1.1:7447"`
     *
     * Each call opens a new, independent WebSocket connection and returns a
     * unique `JsSession` handle.  Up to 4 sessions may be open concurrently.
     *
     * Returns `Promise<JsSession>`.
     */
    static open(locator: string): Promise<JsSession>;
    /**
     * Publish data to `key_expr`. Returns `Promise<void>`.
     */
    put(key_expr: string, payload: Uint8Array, encoding_id: number, attachment?: Uint8Array | null): Promise<void>;
}

/**
 * Handle returned by declareSubscriber.
 */
export class JsSubscriber {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The numeric ID assigned to this subscriber.
     */
    id(): number;
    /**
     * Undeclare this subscriber.
     */
    undeclare(): Promise<void>;
}

/**
 * Returns true if key expression `a` includes all resources matched by `b`.
 * `a includes b` means every key matched by `b` is also matched by `a`.
 */
export function ke_includes(a: string, b: string): boolean;

/**
 * Returns true if key expression `a` intersects `b` (they could match the same resource).
 */
export function ke_intersects(a: string, b: string): boolean;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly _embassy_time_now: () => bigint;
    readonly __wbg_get_jsquery_key_expr: (a: number) => [number, number];
    readonly __wbg_get_jsquery_parameters: (a: number) => [number, number];
    readonly __wbg_get_jsquery_payload: (a: number) => [number, number];
    readonly __wbg_get_jsreply_is_ok: (a: number) => number;
    readonly __wbg_get_jsreply_sample: (a: number) => number;
    readonly __wbg_get_jssample_encoding_id: (a: number) => number;
    readonly __wbg_get_jssample_kind: (a: number) => number;
    readonly __wbg_get_jssample_payload: (a: number) => [number, number];
    readonly __wbg_jspublisher_free: (a: number, b: number) => void;
    readonly __wbg_jsquerier_free: (a: number, b: number) => void;
    readonly __wbg_jsquery_free: (a: number, b: number) => void;
    readonly __wbg_jsqueryable_free: (a: number, b: number) => void;
    readonly __wbg_jsreply_free: (a: number, b: number) => void;
    readonly __wbg_jssample_free: (a: number, b: number) => void;
    readonly __wbg_jssession_free: (a: number, b: number) => void;
    readonly __wbg_jssubscriber_free: (a: number, b: number) => void;
    readonly __wbg_set_jsquery_key_expr: (a: number, b: number, c: number) => void;
    readonly __wbg_set_jsquery_parameters: (a: number, b: number, c: number) => void;
    readonly __wbg_set_jsquery_payload: (a: number, b: number, c: number) => void;
    readonly __wbg_set_jsreply_is_ok: (a: number, b: number) => void;
    readonly __wbg_set_jsreply_sample: (a: number, b: number) => void;
    readonly __wbg_set_jssample_encoding_id: (a: number, b: number) => void;
    readonly __wbg_set_jssample_kind: (a: number, b: number) => void;
    readonly __wbg_set_jssample_payload: (a: number, b: number, c: number) => void;
    readonly jspublisher_delete: (a: number) => any;
    readonly jspublisher_key_expr: (a: number) => [number, number];
    readonly jspublisher_put: (a: number, b: number, c: number, d: number, e: number, f: number) => any;
    readonly jspublisher_undeclare: (a: number) => void;
    readonly jsquerier_get: (a: number, b: any, c: number, d: number, e: number, f: number, g: number) => any;
    readonly jsquery_finalize: (a: number) => any;
    readonly jsquery_reply: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly jsquery_reply_err: (a: number, b: number, c: number) => any;
    readonly jsqueryable_id: (a: number) => number;
    readonly jsqueryable_undeclare: (a: number) => any;
    readonly jssession_close: (a: number) => void;
    readonly jssession_declare_publisher: (a: number, b: number, c: number) => any;
    readonly jssession_declare_querier: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly jssession_declare_queryable: (a: number, b: number, c: number, d: any) => any;
    readonly jssession_declare_subscriber: (a: number, b: number, c: number, d: any) => any;
    readonly jssession_delete: (a: number, b: number, c: number) => any;
    readonly jssession_get: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: any, i: number, j: number, k: number) => any;
    readonly jssession_open: (a: number, b: number) => any;
    readonly jssession_put: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => any;
    readonly jssubscriber_undeclare: (a: number) => any;
    readonly ke_includes: (a: number, b: number, c: number, d: number) => number;
    readonly ke_intersects: (a: number, b: number, c: number, d: number) => number;
    readonly __wbg_get_jssample_key_expr: (a: number) => [number, number];
    readonly jsquerier_undeclare: (a: number) => void;
    readonly jssubscriber_id: (a: number) => number;
    readonly __wbg_set_jssample_key_expr: (a: number, b: number, c: number) => void;
    readonly _embassy_time_schedule_wake: (a: bigint, b: number) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h3a025749c5eccc02: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h6488d5db925c6877: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h96308b8e6da1fb31: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h59e376138512cd76: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h59e376138512cd76_3: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h5b046f0c45fb325d: (a: number, b: number) => void;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;

export function cancelAllWasmTimers(): void;
