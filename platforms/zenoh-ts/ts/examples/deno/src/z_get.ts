/**
 * z_get: Issue a get query and print the replies.
 *
 * Usage: deno run --allow-net z_get.ts [-e ws/127.0.0.1:7447] [-k "demo/example/**"]
 */
import { open, Config } from "@eclipse-zenoh/zenoh-ts";
import { parseArgs } from "./parse_args.ts";

const { locator, key } = parseArgs(Deno.args, { key: "demo/example/**" });

console.log(`Opening session on ${locator} ...`);
const session = await open(new Config(locator));

console.log(`Getting resources matching '${key}' ...`);
const replies = await session.get(key, { timeout: 5_000 });

for await (const reply of replies) {
    if (reply.isOk()) {
        const sample = reply.result();
        console.log(
            `>> Received ('${sample.keyexpr()}'): '${sample.payload()}'`,
        );
    } else {
        console.log(`>> Received error: '${reply.result().payload()}'`);
    }
}

console.log("Done.");
session.close();
