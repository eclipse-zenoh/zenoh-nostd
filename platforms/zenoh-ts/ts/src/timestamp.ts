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
/** Zenoh node identifier (128-bit, displayed as hex string). */
export class ZenohId {
    constructor(private readonly _hex: string) {}

    toString(): string {
        return this._hex;
    }

    static fromString(s: string): ZenohId {
        return new ZenohId(s);
    }
}

/** A Zenoh timestamp (HLC-based). Not yet propagated through the WASM bindings. */
export class Timestamp {
    constructor(
        private readonly _ntp64: bigint = 0n,
        private readonly _id: ZenohId = new ZenohId("0000000000000000"),
    ) {}

    /** Convert NTP-64 timestamp to a JS Date (approximate). */
    asDate(): Date {
        // NTP epoch is 1900-01-01; subtract to get Unix epoch
        const secondsSinceNtp = Number(this._ntp64 >> 32n);
        const ntp1900OffsetSecs = 2208988800; // seconds between 1900 and 1970
        return new Date((secondsSinceNtp - ntp1900OffsetSecs) * 1000);
    }

    id(): ZenohId {
        return this._id;
    }

    toString(): string {
        return `${this._ntp64.toString(16)}/${this._id}`;
    }
}
