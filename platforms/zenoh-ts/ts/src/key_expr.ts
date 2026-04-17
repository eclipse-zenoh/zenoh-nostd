// The ke_intersects / ke_includes functions are imported from the WASM build output.
// During type-checking (before wasm-bindgen runs) we need the pkg/ directory to exist.
// The build script creates pkg/zenoh_ts_wasm.js — import is safe at runtime.
import { ke_intersects, ke_includes } from "../pkg/zenoh_ts_wasm.js";

export type IntoKeyExpr = KeyExpr | string;

export class KeyExpr {
    private readonly _expr: string;

    constructor(expr: IntoKeyExpr) {
        this._expr = expr instanceof KeyExpr ? expr._expr : expr;
    }

    /** Canonicalize and return a KeyExpr (alias for constructor, mirrors zenoh-ts API). */
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
