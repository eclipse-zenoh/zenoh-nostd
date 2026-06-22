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
