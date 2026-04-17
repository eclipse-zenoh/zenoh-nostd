import type { JsPublisher, JsSubscriber, JsQueryable, JsQuerier, JsReply as WasmReply } from "../pkg/zenoh_ts_wasm.js";
import { ZBytes, type IntoZBytes } from "./z_bytes.ts";
import { Encoding } from "./encoding.ts";
import { SampleKind } from "./enums.ts";
import { Sample } from "./sample.ts";
import { Query } from "./query.ts";
import { FifoChannel, type ChannelReceiver } from "./channels.ts";
import { Parameters } from "./selector.ts";

// ── ReplyError ────────────────────────────────────────────────────────────────

/**
 * An error reply received from a queryable.
 * Compatible with `@eclipse-zenoh/zenoh-ts` `ReplyError` class.
 */
export class ReplyError {
    constructor(
        private readonly _payload: ZBytes,
        private readonly _encoding: Encoding,
    ) {}

    payload(): ZBytes {
        return this._payload;
    }

    encoding(): Encoding {
        return this._encoding;
    }
}

// ── Publisher ─────────────────────────────────────────────────────────────────

export interface PublisherOptions {
    encoding?: Encoding;
    priority?: number;
    congestionControl?: number;
    express?: boolean;
    reliability?: number;
}

export class Publisher {
    constructor(
        private readonly _handle: JsPublisher,
        private readonly _opts?: PublisherOptions,
    ) {}

    keyExpr(): string {
        return this._handle.key_expr();
    }

    /** Publish `payload` to this publisher's key expression. */
    async put(payload: IntoZBytes, opts?: { encoding?: Encoding; attachment?: IntoZBytes }): Promise<void> {
        const bytes = ZBytes.from(payload).toBytes();
        const encId = opts?.encoding?.id ?? this._opts?.encoding?.id ?? 0;
        const attach = opts?.attachment ? ZBytes.from(opts.attachment).toBytes() : undefined;
        await this._handle.put(bytes, encId, attach ?? null);
    }

    /** Send a delete notification via this publisher. */
    async delete(): Promise<void> {
        await this._handle.delete();
    }

    /** Undeclare this publisher. */
    async undeclare(): Promise<void> {
        await this._handle.undeclare();
    }

    [Symbol.asyncDispose](): Promise<void> {
        return this.undeclare();
    }
}

// ── Subscriber ────────────────────────────────────────────────────────────────

export class Subscriber {
    constructor(
        private readonly _handle: JsSubscriber,
        private readonly _channel: FifoChannel<Sample> | null,
    ) {}

    /**
     * Returns the channel for iterating over received samples, or `undefined`
     * if the subscriber was created with a direct callback.
     */
    receiver(): FifoChannel<Sample> | undefined {
        return this._channel ?? undefined;
    }

    async undeclare(): Promise<void> {
        this._channel?.close();
        await this._handle.undeclare();
    }

    [Symbol.asyncDispose](): Promise<void> {
        return this.undeclare();
    }
}

// ── Queryable ─────────────────────────────────────────────────────────────────

export class Queryable {
    constructor(
        private readonly _handle: JsQueryable,
        private readonly _channel: FifoChannel<Query> | null,
    ) {}

    /**
     * Returns the channel for iterating over incoming queries, or `undefined`
     * if the queryable was created with a direct callback.
     */
    receiver(): FifoChannel<Query> | undefined {
        return this._channel ?? undefined;
    }

    async undeclare(): Promise<void> {
        this._channel?.close();
        await this._handle.undeclare();
    }

    [Symbol.asyncDispose](): Promise<void> {
        return this.undeclare();
    }
}

// ── Reply ─────────────────────────────────────────────────────────────────────

/**
 * A get/querier reply. `result()` returns either a `Sample` (for success) or
 * a `ReplyError` (for error replies).
 *
 * Compatible with `@eclipse-zenoh/zenoh-ts` `Reply` class.
 */
export class Reply {
    constructor(
        private readonly _isOk: boolean,
        private readonly _keyExpr: string,
        private readonly _payload: ZBytes,
        private readonly _encoding: Encoding,
    ) {}

    /** Returns `true` if this is a successful reply. */
    isOk(): boolean {
        return this._isOk;
    }

    /**
     * Returns the reply data as `Sample` (on success) or `ReplyError` (on error).
     *
     * @example
     * ```ts
     * const result = reply.result();
     * if (result instanceof ReplyError) { ... }
     * else { console.log(result.payload().toString()); }
     * ```
     */
    result(): Sample | ReplyError {
        if (this._isOk) {
            return new Sample(this._keyExpr, this._payload, SampleKind.Put, this._encoding);
        }
        return new ReplyError(this._payload, this._encoding);
    }

    static fromWasm(js: WasmReply): Reply {
        return new Reply(
            js.is_ok,
            js.sample.key_expr,
            new ZBytes(js.sample.payload),
            new Encoding(js.sample.encoding_id),
        );
    }
}

// ── Querier ───────────────────────────────────────────────────────────────────

export interface QuerierGetOptions {
    payload?: IntoZBytes;
    parameters?: string | Parameters;
    timeout?: number;
    handler?: (reply: Reply) => void;
}

export class Querier {
    constructor(
        private readonly _handle: JsQuerier,
        private readonly _ke: string,
        private readonly _defaultTimeoutMs: number,
    ) {}

    keyExpr(): string {
        return this._ke;
    }

    /**
     * Issue a get via this querier.
     *
     * - When `opts.handler` is provided: calls it for each reply, returns `undefined`.
     * - When no handler: returns a `ChannelReceiver<Reply>` that closes after the timeout.
     */
    async get(opts?: QuerierGetOptions): Promise<ChannelReceiver<Reply> | undefined> {
        const timeoutMs = opts?.timeout ?? this._defaultTimeoutMs;
        const rawParams = opts?.parameters !== undefined
            ? opts.parameters.toString()
            : undefined;
        const payloadBytes = opts?.payload
            ? ZBytes.from(opts.payload).toBytes()
            : undefined;

        if (typeof opts?.handler === "function") {
            // Callback mode: fire handler for each reply, return undefined.
            const handler = opts.handler;
            const wasmCb = (jsReply: WasmReply) => {
                handler(Reply.fromWasm(jsReply));
            };
            const done = this._handle.get(
                wasmCb as unknown as (r: WasmReply) => void,
                rawParams ?? null,
                payloadBytes ?? null,
                timeoutMs,
            );
            done.catch(console.error);
            return undefined;
        }

        // Channel mode: return receiver.
        const channel = new FifoChannel<Reply>(256);
        const wasmCb = (jsReply: WasmReply) => {
            channel.push(Reply.fromWasm(jsReply));
        };
        const done = this._handle.get(
            wasmCb as unknown as (r: WasmReply) => void,
            rawParams ?? null,
            payloadBytes ?? null,
            timeoutMs,
        );
        done.then(() => channel.close()).catch(() => channel.close());
        return channel;
    }

    async undeclare(): Promise<void> {
        await this._handle.undeclare();
    }

    [Symbol.asyncDispose](): Promise<void> {
        return this.undeclare();
    }
}
