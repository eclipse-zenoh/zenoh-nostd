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
