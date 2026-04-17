/**
 * z_delete: Send a delete notification on a key expression.
 *
 * Usage: deno run --allow-net z_delete.ts [-e ws/127.0.0.1:7447] [-k demo/example]
 */
import { open, Config } from "@eclipse-zenoh/zenoh-ts";
import { parseArgs } from "./parse_args.ts";

const { locator, key } = parseArgs(Deno.args, { key: "demo/example" });

console.log(`Opening session on ${locator} ...`);
const session = await open(new Config(locator));

console.log(`Deleting Data at '${key}' ...`);
await session.delete(key);

session.close();
