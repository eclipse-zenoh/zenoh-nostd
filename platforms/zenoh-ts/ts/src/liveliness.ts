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
import type { Session } from "./session.ts";
import { FifoChannel } from "./channels.ts";
import type { Sample } from "./sample.ts";

/** Liveliness token (stub — not yet implemented in zenoh-nostd). */
export class LivelinessToken {
    async undeclare(): Promise<void> {
        throw new Error("Liveliness not yet implemented in zenoh-nostd");
    }
}

/** Liveliness subscriber (stub). */
export class LivelinessSubscriber {
    receiver(): FifoChannel<Sample> {
        throw new Error("Liveliness not yet implemented in zenoh-nostd");
    }

    async undeclare(): Promise<void> {
        throw new Error("Liveliness not yet implemented in zenoh-nostd");
    }
}

/** Liveliness API — stub; mirrors zenoh-ts surface. */
export class Liveliness {
    constructor(private readonly _session: Session) {
        void this._session; // suppress unused warning
    }

    async declareToken(_keyExpr: string): Promise<LivelinessToken> {
        throw new Error("Liveliness not yet implemented in zenoh-nostd");
    }

    async declareSubscriber(
        _keyExpr: string,
        _opts?: { history?: boolean },
    ): Promise<LivelinessSubscriber> {
        throw new Error("Liveliness not yet implemented in zenoh-nostd");
    }

    async get(_keyExpr: string, _timeout?: number): Promise<FifoChannel<Sample>> {
        throw new Error("Liveliness not yet implemented in zenoh-nostd");
    }
}
