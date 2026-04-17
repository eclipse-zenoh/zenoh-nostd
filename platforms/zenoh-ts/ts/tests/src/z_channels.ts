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
