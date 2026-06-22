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
/**
 * Test: closed/undeclared guards and honest stubs (zenoh-nostd specific).
 * Requires a running router at ws/127.0.0.1:10000.
 */
import { assertRejects } from "jsr:@std/assert";
import { Config, Session } from "../../src/index.ts";

const LOC = "ws/127.0.0.1:10000";

// Let the WASM WebSocket finish closing before the test ends, so Deno's resource
// sanitizer does not flag the in-flight close (matches the integration tests).
function settle() {
    return new Promise((r) => setTimeout(r, 200));
}

Deno.test("Session: operations reject after close", async () => {
    const session = await Session.open(new Config(LOC));
    await session.close();
    await session.close(); // idempotent

    await assertRejects(() => session.put("a/b", "x"), Error, "Session is closed");
    await assertRejects(() => session.get("a/b"), Error, "Session is closed");
    await assertRejects(() => session.declarePublisher("a/b"), Error, "Session is closed");
    await assertRejects(() => session.declareSubscriber("a/b"), Error, "Session is closed");
    await assertRejects(() => session.declareQueryable("a/b"), Error, "Session is closed");
    await assertRejects(() => session.declareQuerier("a/b"), Error, "Session is closed");

    await settle();
});

Deno.test("Publisher: put rejects after undeclare; undeclare is idempotent", async () => {
    const session = await Session.open(new Config(LOC));
    try {
        const pub = await session.declarePublisher("a/b");
        await pub.undeclare();
        await pub.undeclare(); // idempotent, no throw
        await assertRejects(() => pub.put("x"), Error, "Publisher is undeclared");
    } finally {
        await session.close();
        await settle();
    }
});

Deno.test("Querier: get rejects after undeclare", async () => {
    const session = await Session.open(new Config(LOC));
    try {
        const querier = await session.declareQuerier("a/b");
        await querier.undeclare();
        await querier.undeclare(); // idempotent
        await assertRejects(() => querier.get(), Error, "Querier is undeclared");
    } finally {
        await session.close();
        await settle();
    }
});

Deno.test("Session.info is an honest stub (throws, not a fake id)", async () => {
    const session = await Session.open(new Config(LOC));
    try {
        await assertRejects(() => session.info(), Error, "not yet implemented");
    } finally {
        await session.close();
        await settle();
    }
});
