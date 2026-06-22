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
import { ZBytes, type IntoZBytes } from "./z_bytes.ts";
import { type IntoKeyExpr } from "./key_expr.ts";
import { Encoding } from "./encoding.ts";
import { Parameters, Selector } from "./selector.ts";
import type { JsQuery } from "../pkg/zenoh_ts_wasm.js";

/** An incoming query received by a Queryable. */
export class Query {
    constructor(private readonly _inner: JsQuery) {}

    /** The key expression of the query. */
    keyExpr(): string {
        return this._inner.key_expr;
    }

    /** Full selector object: `key_expr[?parameters]`. */
    selector(): Selector {
        return new Selector(this._inner.key_expr, this._inner.parameters ?? undefined);
    }

    /**
     * The parameters of the query as a `Parameters` object.
     * Returns an empty `Parameters` if no parameters were sent.
     */
    parameters(): Parameters {
        return new Parameters(this._inner.parameters ?? "");
    }

    /** The query payload, if any. */
    payload(): ZBytes | undefined {
        const p = this._inner.payload;
        return p ? new ZBytes(p) : undefined;
    }

    /** The query encoding (not yet propagated from WASM). */
    encoding(): Encoding | undefined {
        return undefined;
    }

    /** The query attachment (not yet propagated from WASM). */
    attachment(): ZBytes | undefined {
        return undefined;
    }

    /** Send a successful reply for this query. */
    async reply(keyExpr: IntoKeyExpr, payload: IntoZBytes): Promise<void> {
        const bytes = ZBytes.from(payload).toBytes();
        await this._inner.reply(keyExpr.toString(), bytes);
    }

    /** Send an error reply for this query. */
    async replyErr(error: IntoZBytes): Promise<void> {
        const bytes = ZBytes.from(error).toBytes();
        await this._inner.reply_err(bytes);
    }

    /**
     * Send a delete reply for this query.
     *
     * Not yet implemented: the zenoh-nostd protocol layer has no delete (`Del`)
     * push body, so this throws rather than sending an empty `Put` that a
     * querier would misinterpret as a successful (empty-payload) reply.
     */
    async replyDel(_keyExpr: IntoKeyExpr): Promise<void> {
        throw new Error("Query.replyDel() is not yet implemented in zenoh-nostd");
    }

    /**
     * Finalize this query. Must be called once all replies have been sent.
     * Sends a `ResponseFinal` to the querier.
     */
    async finalize(): Promise<void> {
        await this._inner.finalize();
    }
}
