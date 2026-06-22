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
 * z_ping: Latency benchmark — sends a payload and waits for z_pong to echo it back.
 *
 * Usage: deno run --allow-net z_ping.ts [-e ws/127.0.0.1:7447] [-n 100]
 */
import { open, Config, ZBytes, FifoChannel, Sample } from "@eclipse-zenoh/zenoh-ts";
import { parseArgs } from "./parse_args.ts";

const PING_KEY = "test/ping";
const PONG_KEY = "test/pong";

const { locator, n, value } = parseArgs(Deno.args, { n: 100, value: "64" });
const payloadSize = parseInt(value, 10) || 64;
const samples = n || 100;

console.log(`Opening session on ${locator} ...`);
const session = await open(new Config(locator));

const pongChannel = new FifoChannel<Sample>(1);
const sub = await session.declareSubscriber(PONG_KEY, { handler: pongChannel });
const pub = await session.declarePublisher(PING_KEY);

const payload = new ZBytes(new Uint8Array(payloadSize));

// Warmup
for (let i = 0; i < 10; i++) {
    await pub.put(payload);
    await pongChannel.receive();
}

// Measure
const latencies: number[] = [];
for (let i = 0; i < samples; i++) {
    const t0 = performance.now();
    await pub.put(payload);
    await pongChannel.receive();
    latencies.push(performance.now() - t0);
}

latencies.sort((a, b) => a - b);
const mean = latencies.reduce((a, b) => a + b, 0) / latencies.length;
const min  = latencies[0];
const max  = latencies[latencies.length - 1];
const p50  = latencies[Math.floor(latencies.length * 0.5)];
const p99  = latencies[Math.floor(latencies.length * 0.99)];

console.log(`\nResults (${samples} samples, ${payloadSize} byte payload):`);
console.log(`  mean  = ${mean.toFixed(3)} ms`);
console.log(`  min   = ${min.toFixed(3)} ms`);
console.log(`  p50   = ${p50.toFixed(3)} ms`);
console.log(`  p99   = ${p99.toFixed(3)} ms`);
console.log(`  max   = ${max.toFixed(3)} ms`);

await sub.undeclare();
await pub.undeclare();
session.close();
