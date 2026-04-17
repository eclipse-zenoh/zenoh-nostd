/**
 * z_pub: Publish samples on a key expression at 1 Hz.
 *
 * Usage: deno run --allow-net z_pub.ts [-e ws/127.0.0.1:7447] [-k demo/example] [-v "Hello!"]
 */
import { open, Config, ZBytes, Encoding } from "@eclipse-zenoh/zenoh-ts";
import { parseArgs } from "./parse_args.ts";

const { locator, key, value } = parseArgs(Deno.args, {
    key: "demo/example",
    value: "Hello from zenoh-nostd WASM!",
});

console.log(`Opening session on ${locator} ...`);
const session = await open(new Config(locator));

console.log(`Declaring Publisher on '${key}' ...`);
const pub = await session.declarePublisher(key, { encoding: Encoding.TEXT_PLAIN });

let idx = 0;
while (true) {
    await new Promise((r) => setTimeout(r, 1000));
    const payload = new ZBytes(`${value} [${idx++}]`);
    console.log(`>> [Publisher] Putting Data ('${key}'): '${payload}'`);
    await pub.put(payload);
}
