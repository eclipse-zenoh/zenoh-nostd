/**
 * z_querier: Declare a querier and issue periodic get queries.
 *
 * Usage: deno run --allow-net z_querier.ts [-e ws/127.0.0.1:7447] [-k "demo/example/**"]
 */
import { open, Config } from "@eclipse-zenoh/zenoh-ts";
import { parseArgs } from "./parse_args.ts";

const { locator, key } = parseArgs(Deno.args, { key: "demo/example/**" });

console.log(`Opening session on ${locator} ...`);
const session = await open(new Config(locator));

console.log(`Declaring Querier on '${key}' ...`);
const querier = await session.declareQuerier(key, { timeout: 5_000 });

let i = 0;
while (true) {
    await new Promise((r) => setTimeout(r, 1000));
    console.log(`>> [Querier] Sending query #${i++} on '${key}'`);
    const replies = await querier.get();
    for await (const reply of replies) {
        if (reply.isOk()) {
            const sample = reply.result();
            console.log(
                `   >> Received ('${sample.keyexpr()}'): '${sample.payload()}'`,
            );
        } else {
            console.log(`   >> Received error: '${reply.result().payload()}'`);
        }
    }
}
