import { KeyExpr, type IntoKeyExpr } from "./key_expr.ts";

// ── Parameters ────────────────────────────────────────────────────────────────

/**
 * Represents the parameters portion of a Zenoh selector (the part after `?`).
 *
 * Compatible with `@eclipse-zenoh/zenoh-ts` `Parameters` class.
 */
export class Parameters {
    private readonly _raw: string;

    constructor(params: string = "") {
        this._raw = params;
    }

    toString(): string {
        return this._raw;
    }

    /**
     * Split a `key=value` pair on the first `=` only, so values may themselves
     * contain `=`. A bare `key` (no `=`) yields an empty-string value.
     */
    private static splitPair(pair: string): [string, string] {
        const i = pair.indexOf("=");
        return i < 0 ? [pair, ""] : [pair.slice(0, i), pair.slice(i + 1)];
    }

    /** Returns the value for the given key, or undefined if not present. */
    get(key: string): string | undefined {
        for (const pair of this._raw.split("&")) {
            const [k, v] = Parameters.splitPair(pair);
            if (k === key) return v;
        }
        return undefined;
    }

    containsKey(key: string): boolean {
        return this.get(key) !== undefined;
    }

    isEmpty(): boolean {
        return this._raw === "";
    }

    insert(key: string, value?: string): Parameters {
        const pair = value !== undefined ? `${key}=${value}` : key;
        return new Parameters(this._raw ? `${this._raw}&${pair}` : pair);
    }

    remove(key: string): Parameters {
        const parts = this._raw.split("&").filter((p) => !p.startsWith(key + "=") && p !== key);
        return new Parameters(parts.join("&"));
    }

    iter(): [string, string][] {
        if (this._raw === "") return [];
        return this._raw.split("&").map((pair) => Parameters.splitPair(pair));
    }

    values(): string[] {
        return this.iter().map(([, v]) => v);
    }
}

// ── Selector ──────────────────────────────────────────────────────────────────

/**
 * A Zenoh selector: a key expression plus optional parameters.
 *
 * `new Selector("a/b", "key=value")` or `new Selector(ke, params)`.
 *
 * Compatible with `@eclipse-zenoh/zenoh-ts` `Selector` class.
 */
export class Selector {
    private readonly _ke: KeyExpr;
    private readonly _params: Parameters;

    constructor(ke: IntoKeyExpr, params?: string | Parameters) {
        this._ke = ke instanceof KeyExpr ? ke : new KeyExpr(ke);
        if (params === undefined) {
            this._params = new Parameters();
        } else if (params instanceof Parameters) {
            this._params = params;
        } else {
            this._params = new Parameters(params);
        }
    }

    keyExpr(): KeyExpr {
        return this._ke;
    }

    parameters(): Parameters {
        return this._params;
    }

    /** Returns the full selector string: `keyexpr[?parameters]`. */
    toString(): string {
        const p = this._params.toString();
        return p ? `${this._ke.toString()}?${p}` : this._ke.toString();
    }

    static from(input: string): Selector {
        const i = input.indexOf("?");
        if (i >= 0) {
            return new Selector(input.slice(0, i), input.slice(i + 1));
        }
        return new Selector(input);
    }
}
