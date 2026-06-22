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
 * z_sub: Subscribe to a key expression and print received samples.
 *
 * Usage: deno run --allow-net z_sub.ts [-e ws/127.0.0.1:7447] [-k "demo/example/**"]
 */
import { open, Config } from "@eclipse-zenoh/zenoh-ts";
import { parseArgs } from "./parse_args.ts";

const { locator, key } = parseArgs(Deno.args, { key: "demo/example/**" });

console.log(`Opening session on ${locator} ...`);
const session = await open(new Config(locator));

console.log(`Declaring Subscriber on '${key}' ...`);
const sub = await session.declareSubscriber(key);

for await (const sample of sub.receiver()) {
    console.log(
        `>> [Subscriber] Received ('${sample.keyexpr()}'): '${sample.payload()}'`,
    );
}
