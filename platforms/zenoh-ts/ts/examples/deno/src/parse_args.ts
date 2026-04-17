/**
 * Simple command-line argument parser for Deno examples.
 * Mirrors the argument conventions used in @eclipse-zenoh/zenoh-ts examples.
 */

export interface ExampleArgs {
    /** Zenoh locator, e.g. ws/127.0.0.1:7447 */
    locator: string;
    /** Key expression */
    key: string;
    /** Payload value */
    value: string;
    /** Count (iterations or samples) */
    n: number;
}

function flag(args: string[], short: string, long: string): string | undefined {
    for (let i = 0; i < args.length - 1; i++) {
        if (args[i] === `-${short}` || args[i] === `--${long}`) {
            return args[i + 1];
        }
    }
    return undefined;
}

export function parseArgs(
    argv: string[],
    defaults: Partial<ExampleArgs> = {},
): ExampleArgs {
    return {
        locator: flag(argv, "e", "endpoint") ?? defaults.locator ?? "ws/127.0.0.1:7447",
        key:     flag(argv, "k", "key")      ?? defaults.key     ?? "demo/example",
        value:   flag(argv, "v", "value")    ?? defaults.value   ?? "Hello from zenoh-nostd WASM!",
        n:       parseInt(flag(argv, "n", "number") ?? String(defaults.n ?? 0), 10),
    };
}
