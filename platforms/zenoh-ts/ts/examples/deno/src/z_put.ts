/**
 * z_put: Publish a single sample on a key expression.
 *
 * Usage: deno run --allow-net z_put.ts [-e ws/127.0.0.1:7447] [-k demo/example] [-v "Hello!"]
 */
import { open, Config, ZBytes } from "@eclipse-zenoh/zenoh-ts";
import { parseArgs } from "./parse_args.ts";

const { locator, key, value } = parseArgs(Deno.args, {
    key: "demo/example",
    value: "Hello from zenoh-nostd WASM!",
});

console.log(`Opening session on ${locator} ...`);
const session = await open(new Config(locator));

console.log(`Putting Data ('${key}'): '${value}'`);
await session.put(key, new ZBytes(value));

session.close();
