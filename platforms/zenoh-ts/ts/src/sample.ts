import { ZBytes } from "./z_bytes.ts";
import { Encoding } from "./encoding.ts";
import { SampleKind, Priority, CongestionControl } from "./enums.ts";
import type { Timestamp } from "./timestamp.ts";

/** A Zenoh sample received via pub/sub. */
export class Sample {
    constructor(
        private readonly _keyExpr: string,
        private readonly _payload: ZBytes,
        private readonly _kind: SampleKind = SampleKind.Put,
        private readonly _encoding: Encoding = Encoding.DEFAULT,
        private readonly _timestamp: Timestamp | undefined = undefined,
        private readonly _priority: Priority = Priority.Data,
        private readonly _congestionControl: CongestionControl = CongestionControl.Drop,
        private readonly _express: boolean = false,
        private readonly _attachment: ZBytes | undefined = undefined,
    ) {}

    /** Construct a Sample from raw WASM values. */
    static fromWasm(
        keyExpr: string,
        payload: Uint8Array,
        encodingId: number,
        kind: number,
    ): Sample {
        return new Sample(
            keyExpr,
            new ZBytes(payload),
            kind as SampleKind,
            new Encoding(encodingId),
        );
    }

    keyexpr(): string {
        return this._keyExpr;
    }

    payload(): ZBytes {
        return this._payload;
    }

    kind(): SampleKind {
        return this._kind;
    }

    encoding(): Encoding {
        return this._encoding;
    }

    timestamp(): Timestamp | undefined {
        return this._timestamp;
    }

    priority(): Priority {
        return this._priority;
    }

    congestionControl(): CongestionControl {
        return this._congestionControl;
    }

    express(): boolean {
        return this._express;
    }

    attachment(): ZBytes | undefined {
        return this._attachment;
    }

    toString(): string {
        return `Sample(keyexpr='${this._keyExpr}', payload=${this._payload.toString()}, kind=${SampleKind[this._kind]})`;
    }
}
