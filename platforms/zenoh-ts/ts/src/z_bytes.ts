export type IntoZBytes = ZBytes | Uint8Array | string;

export class ZBytes {
    private readonly _data: Uint8Array;

    constructor(data: Uint8Array | string | number[] = new Uint8Array()) {
        if (typeof data === "string") {
            this._data = new TextEncoder().encode(data);
        } else if (Array.isArray(data)) {
            this._data = new Uint8Array(data);
        } else {
            this._data = data;
        }
    }

    static from(x: IntoZBytes): ZBytes {
        if (x instanceof ZBytes) return x;
        return new ZBytes(x as Uint8Array | string);
    }

    static empty(): ZBytes {
        return new ZBytes();
    }

    toBytes(): Uint8Array {
        return this._data;
    }

    /** Decode payload as a UTF-8 string. */
    toString(): string {
        return new TextDecoder().decode(this._data);
    }

    len(): number {
        return this._data.length;
    }

    isEmpty(): boolean {
        return this._data.length === 0;
    }

    /** Deserialize a JSON payload. */
    deserialize<T>(): T {
        return JSON.parse(this.toString()) as T;
    }

    /** Serialize a value to JSON ZBytes. */
    static serialize<T>(value: T): ZBytes {
        return new ZBytes(JSON.stringify(value));
    }
}
