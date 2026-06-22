#!/usr/bin/env deno run --allow-read --allow-write
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
//
// Post-build patch for the wasm-bindgen generated JS glue.
//
// Wraps the setTimeout/clearTimeout bindings used by the embassy-time WASM
// driver so that all outstanding timer IDs are tracked.  Exposes a
// `cancelAllWasmTimers()` export that can be called on session close to
// prevent Deno's sanitize-ops check from failing due to leaked timers.
//

const filePath = Deno.args[0];
if (!filePath) {
    console.error("Usage: patch-wasm-js.ts <path/to/zenoh_ts_wasm.js>");
    Deno.exit(1);
}

// Derive the .d.ts path from the .js path (same directory, same base name).
const dtsPath = filePath.replace(/\.js$/, ".d.ts");

let src = await Deno.readTextFile(filePath);

// --- 1. Add the timer-tracking Set at the very top ---
const HEADER = `const __wasm_pending_timers = new Set();
export function cancelAllWasmTimers() {
    for (const id of __wasm_pending_timers) { clearTimeout(id); }
    __wasm_pending_timers.clear();
}
`;
if (!src.includes("__wasm_pending_timers")) {
    src = HEADER + src;
}

// --- 2. Patch the setTimeout binding ---
// Pattern: __wbg_setTimeout_XXXX: function(arg0, arg1) {
//              const ret = setTimeout(arg0, arg1 >>> 0);
//              return ret;
//          },
src = src.replace(
    /(__wbg_setTimeout_[0-9a-f]+:\s*function\s*\(arg0,\s*arg1\)\s*\{)\s*(\n\s*const ret = setTimeout\(arg0, arg1 >>> 0\);\n\s*return ret;\n\s*\},)/,
    "$1\n            const ret = setTimeout(arg0, arg1 >>> 0);\n            __wasm_pending_timers.add(ret);\n            return ret;\n        },",
);

// --- 3. Patch the clearTimeout binding ---
// Pattern: __wbg_clearTimeout_XXXX: function(arg0) {
//              clearTimeout(arg0);
//          },
src = src.replace(
    /(__wbg_clearTimeout_[0-9a-f]+:\s*function\s*\(arg0\)\s*\{)\s*(\n\s*clearTimeout\(arg0\);\n\s*\},)/,
    "$1\n            __wasm_pending_timers.delete(arg0);\n            clearTimeout(arg0);\n        },",
);

await Deno.writeTextFile(filePath, src);
console.log(`Patched: ${filePath}`);

// --- 4. Add cancelAllWasmTimers() to the .d.ts declaration file ---
// wasm-bindgen has no knowledge of the JS-level wrapper, so we append
// the declaration ourselves so that TypeScript can see the export.
let dts = await Deno.readTextFile(dtsPath);
const DTS_DECL = "\nexport function cancelAllWasmTimers(): void;\n";
if (!dts.includes("cancelAllWasmTimers")) {
    dts += DTS_DECL;
    await Deno.writeTextFile(dtsPath, dts);
    console.log(`Patched: ${dtsPath}`);
}
