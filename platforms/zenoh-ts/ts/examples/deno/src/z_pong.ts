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
 * z_pong: Latency benchmark counterpart — echoes back every payload received on test/ping.
 *
 * Usage: deno run --allow-net z_pong.ts [-e ws/127.0.0.1:7447]
 */
import { open, Config } from "@eclipse-zenoh/zenoh-ts";
import { parseArgs } from "./parse_args.ts";

const PING_KEY = "test/ping";
const PONG_KEY = "test/pong";

const { locator } = parseArgs(Deno.args);

console.log(`Opening session on ${locator} ...`);
const session = await open(new Config(locator));

const pub = await session.declarePublisher(PONG_KEY);
const sub = await session.declareSubscriber(PING_KEY);

console.log("Waiting for pings ... (Ctrl+C to stop)");
for await (const sample of sub.receiver()) {
    await pub.put(sample.payload());
}
