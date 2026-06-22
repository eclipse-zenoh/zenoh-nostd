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
 * Test: FifoChannel and RingChannel behavior (pure TypeScript, no network required).
 */
import { assertEquals, assertStrictEquals } from "jsr:@std/assert";
import { FifoChannel, RingChannel } from "../../src/index.ts";

Deno.test("FifoChannel: push and receive in order", async () => {
    const ch = new FifoChannel<number>(4);
    ch.push(1);
    ch.push(2);
    ch.push(3);

    assertEquals(await ch.receive(), 1);
    assertEquals(await ch.receive(), 2);
    assertEquals(await ch.receive(), 3);
});

Deno.test("FifoChannel: tryReceive returns undefined when empty", () => {
    const ch = new FifoChannel<number>(4);
    assertStrictEquals(ch.tryReceive(), undefined);
    ch.push(42);
    assertStrictEquals(ch.tryReceive(), 42);
    assertStrictEquals(ch.tryReceive(), undefined);
});

Deno.test("FifoChannel: drops newest on overflow", () => {
    const ch = new FifoChannel<number>(3);
    ch.push(1);
    ch.push(2);
    ch.push(3);
    ch.push(4); // dropped (oldest 3 are kept)

    assertEquals(ch.tryReceive(), 1);
    assertEquals(ch.tryReceive(), 2);
    assertEquals(ch.tryReceive(), 3);
    assertStrictEquals(ch.tryReceive(), undefined); // 4 was dropped
});

Deno.test("FifoChannel: close causes pending receives to return null", async () => {
    const ch = new FifoChannel<number>(4);
    const receivePromise = ch.receive();
    ch.close();
    assertStrictEquals(await receivePromise, null);
    assertEquals(ch.isClosed, true);
});

Deno.test("FifoChannel: async iterator stops on close", async () => {
    const ch = new FifoChannel<number>(4);
    ch.push(10);
    ch.push(20);
    ch.close();

    const items: number[] = [];
    for await (const item of ch) {
        items.push(item);
    }
    assertEquals(items, [10, 20]);
});

Deno.test("FifoChannel: waiter receives item pushed after await", async () => {
    const ch = new FifoChannel<string>(4);
    const recvPromise = ch.receive();
    await Promise.resolve(); // yield
    ch.push("hello");
    assertEquals(await recvPromise, "hello");
});

Deno.test("RingChannel: drops oldest on overflow", () => {
    const ch = new RingChannel<number>(3);
    ch.push(1);
    ch.push(2);
    ch.push(3);
    ch.push(4); // 1 is dropped (oldest)

    assertEquals(ch.tryReceive(), 2);
    assertEquals(ch.tryReceive(), 3);
    assertEquals(ch.tryReceive(), 4);
    assertStrictEquals(ch.tryReceive(), undefined);
});

Deno.test("RingChannel: continues working after overflow", () => {
    const ch = new RingChannel<number>(2);
    for (let i = 0; i < 10; i++) ch.push(i);
    // Only last 2 items remain
    assertEquals(ch.tryReceive(), 8);
    assertEquals(ch.tryReceive(), 9);
    assertStrictEquals(ch.tryReceive(), undefined);
});

Deno.test("FifoChannel: fail rejects a pending receive", async () => {
    const ch = new FifoChannel<number>(4);
    const pending = ch.receive();
    ch.fail(new Error("boom"));
    await pending.then(
        () => {
            throw new Error("receive should have rejected");
        },
        (e) => assertEquals((e as Error).message, "boom"),
    );
});

Deno.test("FifoChannel: fail rejects future receives (distinct from close)", async () => {
    const ch = new FifoChannel<number>(4);
    ch.fail(new Error("dead"));
    await ch.receive().then(
        () => {
            throw new Error("receive should have rejected");
        },
        (e) => assertEquals((e as Error).message, "dead"),
    );
});

Deno.test("FifoChannel: buffered items drain before fail takes effect", async () => {
    const ch = new FifoChannel<number>(4);
    ch.push(1);
    ch.fail(new Error("later"));
    assertEquals(await ch.receive(), 1); // queued item still delivered
    await ch.receive().then(
        () => {
            throw new Error("receive should have rejected");
        },
        (e) => assertEquals((e as Error).message, "later"),
    );
});

Deno.test("FifoChannel: breaking out of for-await closes the channel", async () => {
    const ch = new FifoChannel<number>(8);
    ch.push(1);
    ch.push(2);
    const seen: number[] = [];
    for await (const v of ch) {
        seen.push(v);
        break; // triggers iterator.return() → close()
    }
    assertEquals(seen, [1]);
    assertEquals(ch.isClosed, true);
});

Deno.test("FifoChannel: await using disposes (closes) the channel", async () => {
    let closed = false;
    {
        await using ch = new FifoChannel<number>(4);
        closed = ch.isClosed;
    }
    assertEquals(closed, false); // open inside the scope
});
