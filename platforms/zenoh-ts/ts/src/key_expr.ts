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
// The ke_intersects / ke_includes functions are imported from the WASM build output.
// During type-checking (before wasm-bindgen runs) we need the pkg/ directory to exist.
// The build script creates pkg/zenoh_ts_wasm.js — import is safe at runtime.
import { ke_intersects, ke_includes } from "../pkg/zenoh_ts_wasm.js";

export type IntoKeyExpr = KeyExpr | string;

// Character codes used by the validator.
const SLASH = 0x2f; // '/'
const STAR = 0x2a; // '*'
const DOLLAR = 0x24; // '$'
const HASH = 0x23; // '#'
const QMARK = 0x3f; // '?'

/**
 * Validate that `s` is a well-formed, canonical key expression.
 *
 * This is a faithful port of `keyexpr::new` in the Rust `zenoh-proto` crate: it
 * accepts only already-canonical expressions and throws otherwise. The
 * zenoh-nostd core does not rewrite (canonicalize) expressions, so non-canonical
 * forms (such as a double-glob immediately followed by another double-glob) are
 * rejected rather than collapsed.
 */
function validateKeyexpr(s: string): void {
    const fail = (reason: string): never => {
        throw new Error(`Invalid key expression '${s}': ${reason}`);
    };

    if (s.length === 0 || s.endsWith("/")) fail("empty chunk");

    const n = s.length;
    const at = (k: number): number | undefined => (k < n ? s.charCodeAt(k) : undefined);

    let chunkStart = 0;
    let i = 0;
    while (i < n) {
        const c = s.charCodeAt(i);
        if (c > SLASH && c !== QMARK) {
            // Ordinary character.
            i += 1;
        } else if (c === SLASH) {
            if (i === chunkStart) fail("empty chunk");
            i += 1;
            chunkStart = i;
        } else if (c === STAR) {
            if (i !== chunkStart) fail("'*' must occupy a whole chunk");
            const c1 = at(i + 1);
            if (c1 === undefined) break; // trailing '*'
            if (c1 === SLASH) {
                i += 2;
                chunkStart = i;
            } else if (c1 === STAR) {
                const c2 = at(i + 2);
                if (c2 === undefined) break; // trailing '**'
                if (c2 === SLASH) {
                    if (at(i + 3) === STAR) fail("non-canonical '**' usage");
                    i += 3;
                    chunkStart = i;
                } else {
                    fail("'*' must occupy a whole chunk");
                }
            } else {
                fail("'*' must occupy a whole chunk");
            }
        } else if (c === DOLLAR) {
            if (at(i + 1) !== STAR) fail("'$' must be followed by '*'");
            const c2 = at(i + 2);
            if (c2 === DOLLAR) fail("'$' after '$*'");
            if ((c2 === SLASH || c2 === undefined) && i === chunkStart) fail("lone '$*' chunk");
            if (c2 === undefined) break;
            i += 2;
        } else if (c === HASH || c === QMARK) {
            fail("'#' and '?' are not allowed");
        } else {
            i += 1;
        }
    }
}

export class KeyExpr {
    private readonly _expr: string;

    constructor(expr: IntoKeyExpr) {
        if (expr instanceof KeyExpr) {
            // Already validated on its own construction.
            this._expr = expr._expr;
        } else {
            validateKeyexpr(expr);
            this._expr = expr;
        }
    }

    /**
     * Validate `expr` and return a `KeyExpr`, mirroring the zenoh-ts API.
     *
     * Note: the zenoh-nostd core validates rather than rewrites, so this throws
     * on non-canonical input instead of canonicalizing it.
     */
    static autocanonize(expr: IntoKeyExpr): KeyExpr {
        return new KeyExpr(expr);
    }

    toString(): string {
        return this._expr;
    }

    /** Returns `true` if this key expression and `other` could match the same resource. */
    intersects(other: IntoKeyExpr): boolean {
        return ke_intersects(this._expr, other.toString());
    }

    /** Returns `true` if every resource matched by `other` is also matched by this key expression. */
    includes(other: IntoKeyExpr): boolean {
        return ke_includes(this._expr, other.toString());
    }

    /** Append a suffix separated by `/`. */
    join(other: IntoKeyExpr): KeyExpr {
        return new KeyExpr(`${this._expr}/${other}`);
    }

    /** Append `suffix` without adding a separator. */
    concat(suffix: string): KeyExpr {
        return new KeyExpr(`${this._expr}${suffix}`);
    }
}
