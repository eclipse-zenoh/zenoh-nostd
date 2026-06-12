import init, { JsSession as WasmSession, JsReply as WasmReply, cancelAllWasmTimers } from "../pkg/zenoh_ts_wasm.js";
import { Config } from "./config.ts";
import { type IntoKeyExpr } from "./key_expr.ts";
import { ZBytes, type IntoZBytes } from "./z_bytes.ts";
import { Encoding } from "./encoding.ts";
import { FifoChannel, type ChannelReceiver } from "./channels.ts";
import { Sample } from "./sample.ts";
import { Query } from "./query.ts";
import {
    Publisher,
    Subscriber,
    Queryable,
    Querier,
    Reply,
    type PublisherOptions,
} from "./pubsub.ts";
import { Liveliness } from "./liveliness.ts";
import { ZenohId } from "./timestamp.ts";
import { Selector, Parameters } from "./selector.ts";

// ── WASM initialization ───────────────────────────────────────────────────────

let _wasmInitialized = false;
let _openSessionCount = 0;

async function ensureWasm(): Promise<void> {
    if (!_wasmInitialized) {
        await init();
        _wasmInitialized = true;
    }
}

// ── Selector / parameters extraction ─────────────────────────────────────────

function extractKeParams(
    selector: IntoKeyExpr | Selector,
    optsParams?: string | Parameters,
): [string, string | undefined] {
    const raw = selector instanceof Selector
        ? selector.toString()
        : selector.toString();

    const i = raw.indexOf("?");
    const ke = i >= 0 ? raw.slice(0, i) : raw;
    const selParams = i >= 0 ? raw.slice(i + 1) : undefined;

    // opts.parameters takes precedence over selector parameters.
    const params = optsParams !== undefined
        ? optsParams.toString()
        : selParams;

    return [ke, params];
}

// ── Public option types ───────────────────────────────────────────────────────

export interface PutOptions {
    encoding?: Encoding;
    priority?: number;
    congestionControl?: number;
    express?: boolean;
    attachment?: IntoZBytes;
}

export interface GetOptions {
    payload?: IntoZBytes;
    encoding?: Encoding;
    parameters?: string | Parameters;
    timeout?: number;
    target?: number;
    consolidation?: number;
    attachment?: IntoZBytes;
    /** When provided, called for each reply and the function returns `undefined`. */
    handler?: (reply: Reply) => void;
}

export interface SubscriberOptions {
    handler?: FifoChannel<Sample> | ((s: Sample) => void) | ((s: Sample) => Promise<void>);
}

export interface QueryableOptions {
    handler?: ((q: Query) => void) | ((q: Query) => Promise<void>);
    complete?: boolean;
}

export interface QuerierOptions {
    timeout?: number;
    target?: number;
    consolidation?: number;
    localRouting?: boolean;
}

// ── Session ───────────────────────────────────────────────────────────────────

export class Session {
    private _closed = false;
    private _wasm: WasmSession;

    private constructor(wasm: WasmSession) {
        this._wasm = wasm;
        _openSessionCount += 1;
    }

    /**
     * Open a Zenoh session.
     *
     * Each call creates an independent WebSocket connection to the router.
     *
     * @param config - A `Config` object or locator string (e.g. `"ws/127.0.0.1:7447"`).
     *   Defaults to `ws/127.0.0.1:7447`.
     */
    static async open(config?: Config | string): Promise<Session> {
        await ensureWasm();
        const cfg = Config.from(config);
        const wasm = await WasmSession.open(cfg.locator);
        return new Session(wasm);
    }

    /** Close this session and its underlying WebSocket connection. */
    async close(): Promise<void> {
        if (this._closed) return;
        this._closed = true;
        if (_openSessionCount > 0) {
            _openSessionCount -= 1;
        }
        // The timer tracker is process-global inside the generated WASM glue, so
        // only drain it once the last live session is closed.
        if (_openSessionCount === 0) {
            cancelAllWasmTimers();
        }
        this._wasm.close();
    }

    isClosed(): boolean {
        return this._closed;
    }

    // ── Session info ──────────────────────────────────────────────────────────

    async info(): Promise<{ zid(): ZenohId }> {
        return {
            zid: () => ZenohId.fromString("wasm-zenoh-nostd"),
        };
    }

    // ── Put / Delete ──────────────────────────────────────────────────────────

    /** Publish data to `keyExpr`. */
    async put(keyExpr: IntoKeyExpr, payload: IntoZBytes, opts?: PutOptions): Promise<void> {
        const bytes = ZBytes.from(payload).toBytes();
        const encId = opts?.encoding?.id ?? 0;
        const attach = opts?.attachment
            ? ZBytes.from(opts.attachment).toBytes()
            : null;
        await this._wasm.put(
            keyExpr.toString(),
            bytes,
            encId,
            attach,
            opts?.priority,
            opts?.congestionControl,
            opts?.express,
        );
    }

    /** Send a delete notification for `keyExpr`. */
    async delete(keyExpr: IntoKeyExpr, _opts?: PutOptions): Promise<void> {
        await this._wasm.delete(keyExpr.toString());
    }

    // ── Publisher ─────────────────────────────────────────────────────────────

    /** Declare a publisher on `keyExpr`. */
    async declarePublisher(keyExpr: IntoKeyExpr, opts?: PublisherOptions): Promise<Publisher> {
        const handle = await this._wasm.declare_publisher(keyExpr.toString());
        return new Publisher(handle, opts);
    }

    // ── Subscriber ────────────────────────────────────────────────────────────

    /**
     * Declare a subscriber on `keyExpr`.
     *
     * If `opts.handler` is a `FifoChannel`, received samples are pushed into it.
     * If `opts.handler` is a function, it is called for each sample.
     * If `opts.handler` is omitted, a default `FifoChannel<Sample>(256)` is created.
     */
    async declareSubscriber(
        keyExpr: IntoKeyExpr,
        opts?: SubscriberOptions,
    ): Promise<Subscriber> {
        const ke = keyExpr.toString();
        let channel: FifoChannel<Sample> | null = null;
        let cbFn: ((s: Sample) => void) | null = null;

        if (typeof opts?.handler === "function") {
            const fn = opts.handler;
            cbFn = (s: Sample) => {
                const result = fn(s);
                if (result instanceof Promise) result.catch(console.error);
            };
        } else {
            channel = (opts?.handler instanceof FifoChannel)
                ? opts.handler
                : new FifoChannel<Sample>(256);
        }

        const wasmCallback = (jsPayload: unknown) => {
            const js = jsPayload as { key_expr: string; payload: Uint8Array; encoding_id: number; kind: number };
            const sample = Sample.fromWasm(js.key_expr, js.payload, js.encoding_id, js.kind);
            if (cbFn) {
                cbFn(sample);
            } else {
                channel!.push(sample);
            }
        };

        const handle = await this._wasm.declare_subscriber(ke, wasmCallback as unknown as ((s: unknown) => void));
        return new Subscriber(handle, channel);
    }

    // ── Queryable ─────────────────────────────────────────────────────────────

    /**
     * Declare a queryable on `keyExpr`.
     *
     * If `opts.handler` is a function, it is called for each incoming query.
     * Otherwise a `FifoChannel<Query>(64)` is created.
     */
    async declareQueryable(
        keyExpr: IntoKeyExpr,
        opts?: QueryableOptions,
    ): Promise<Queryable> {
        const ke = keyExpr.toString();
        let channel: FifoChannel<Query> | null = null;
        let cbFn: ((q: Query) => void) | null = null;

        if (typeof opts?.handler === "function") {
            const fn = opts.handler;
            cbFn = (q: Query) => {
                const result = fn(q);
                if (result instanceof Promise) result.catch(console.error);
            };
        } else {
            channel = new FifoChannel<Query>(64);
        }

        const wasmCallback = (jsQuery: unknown) => {
            const q = new Query(jsQuery as import("../pkg/zenoh_ts_wasm.js").JsQuery);
            if (cbFn) {
                cbFn(q);
            } else {
                channel!.push(q);
            }
        };

        const handle = await this._wasm.declare_queryable(ke, wasmCallback as unknown as ((q: unknown) => void));
        // Give the router time to process the DeclareQueryable before returning,
        // so that a get() issued immediately after finds the queryable registered.
        await new Promise<void>((r) => setTimeout(r, 100));
        return new Queryable(handle, channel);
    }

    // ── Querier ───────────────────────────────────────────────────────────────

    /** Declare a querier for `keyExpr`. */
    async declareQuerier(
        keyExpr: IntoKeyExpr,
        opts?: QuerierOptions,
    ): Promise<Querier> {
        const ke = keyExpr.toString();
        const timeoutMs = opts?.timeout ?? 30_000;
        const handle = this._wasm.declare_querier(ke, timeoutMs);
        if (handle instanceof Error) throw handle;
        return new Querier(handle, ke, timeoutMs);
    }

    // ── Get ───────────────────────────────────────────────────────────────────

    /**
     * Issue a get query for `selector`.
     *
     * - When `opts.handler` is provided: calls it for each reply and returns `undefined`.
     * - When no handler: returns a `ChannelReceiver<Reply>` that closes when the
     *   query is finalized by the remote queryable (or the timeout expires).
     */
    async get(
        selector: IntoKeyExpr | Selector,
        opts?: GetOptions,
    ): Promise<ChannelReceiver<Reply> | undefined> {
        const [ke, params] = extractKeParams(selector, opts?.parameters);
        const timeoutMs = opts?.timeout ?? 30_000;
        const payloadBytes = opts?.payload ? ZBytes.from(opts.payload).toBytes() : undefined;

        const target = opts?.target ?? undefined;
        const consolidation = opts?.consolidation ?? undefined;

        if (typeof opts?.handler === "function") {
            // Callback mode: fire-and-forget, return undefined.
            const handler = opts.handler;
            const wasmCb = (jsReply: unknown) => {
                handler(Reply.fromWasm(jsReply as WasmReply));
            };
            const done = this._wasm.get(
                ke,
                params ?? null,
                payloadBytes ?? null,
                wasmCb as unknown as ((r: unknown) => void),
                timeoutMs,
                target,
                consolidation,
            );
            done.catch(console.error);
            return undefined;
        }

        // Channel mode: return receiver.
        const channel = new FifoChannel<Reply>(256);
        const wasmCb = (jsReply: unknown) => {
            channel.push(Reply.fromWasm(jsReply as WasmReply));
        };

        const done = this._wasm.get(
            ke,
            params ?? null,
            payloadBytes ?? null,
            wasmCb as unknown as ((r: unknown) => void),
            timeoutMs,
            target,
            consolidation,
        );
        done.then(() => channel.close()).catch(() => channel.close());

        return channel;
    }

    // ── Liveliness ────────────────────────────────────────────────────────────

    /** Access the Liveliness API (stub — not yet implemented in zenoh-nostd). */
    liveliness(): Liveliness {
        return new Liveliness(this);
    }

    [Symbol.asyncDispose](): Promise<void> {
        return this.close();
    }
}

/**
 * Convenience function: open a session.
 *
 * @example
 * ```ts
 * import { open } from "@eclipse-zenoh/zenoh-ts";
 * const session = await open("ws/127.0.0.1:7447");
 * ```
 */
export async function open(config?: Config | string): Promise<Session> {
    return Session.open(config);
}
