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
