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
 * z_queryable: Declare a queryable and reply to each incoming query.
 *
 * Usage: deno run --allow-net z_queryable.ts [-e ws/127.0.0.1:7447] [-k "demo/example/**"]
 */
import { open, Config, ZBytes } from "@eclipse-zenoh/zenoh-ts";
import { parseArgs } from "./parse_args.ts";

const { locator, key } = parseArgs(Deno.args, { key: "demo/example/**" });

console.log(`Opening session on ${locator} ...`);
const session = await open(new Config(locator));

console.log(`Declaring Queryable on '${key}' ...`);
const queryable = await session.declareQueryable(key);

console.log("Waiting for queries ... (Ctrl+C to stop)");
for await (const query of queryable.receiver()) {
    console.log(`>> [Queryable] Received Query ('${query.selector()}')`);
    await query.reply(query.keyExpr(), new ZBytes("Hello from zenoh-nostd WASM queryable!"));
    await query.finalize();
}
