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
 * Test: KeyExpr operations.
 * ke_intersects / ke_includes delegate to WASM; tested here via the TS wrapper.
 * The pure string operations (join, concat, toString) are tested without WASM.
 */
import { assertEquals, assertThrows } from "jsr:@std/assert";
import { KeyExpr } from "../../src/index.ts";

// ── String operations (no WASM) ───────────────────────────────────────────────

Deno.test("KeyExpr.toString returns the expression string", () => {
    const ke = new KeyExpr("demo/example");
    assertEquals(ke.toString(), "demo/example");
});

Deno.test("KeyExpr.join appends with separator", () => {
    const ke = new KeyExpr("demo").join("example");
    assertEquals(ke.toString(), "demo/example");
});

Deno.test("KeyExpr.concat appends without separator", () => {
    const ke = new KeyExpr("demo/example").concat("/**");
    assertEquals(ke.toString(), "demo/example/**");
});

Deno.test("KeyExpr: constructed from another KeyExpr", () => {
    const a = new KeyExpr("a/b");
    const b = new KeyExpr(a);
    assertEquals(b.toString(), "a/b");
});

Deno.test("KeyExpr.autocanonize is an identity for well-formed expressions", () => {
    const ke = KeyExpr.autocanonize("a/b/c");
    assertEquals(ke.toString(), "a/b/c");
});

// ── Validation (no WASM) ───────────────────────────────────────────────────────

Deno.test("KeyExpr accepts well-formed wildcard expressions", () => {
    for (const valid of ["a", "a/b/c", "a/*/b", "a/**", "a/**/b", "*", "**", "a/b$*", "a$*/b"]) {
        new KeyExpr(valid); // must not throw
    }
});

Deno.test("KeyExpr rejects malformed expressions", () => {
    for (const bad of ["", "a/", "/a", "a//b", "a/*x", "a/x*", "a/?", "a/#", "a/x$y", "$*"]) {
        assertThrows(() => new KeyExpr(bad), Error, "Invalid key expression");
    }
});

Deno.test("KeyExpr.autocanonize rejects non-canonical '**' usage", () => {
    assertThrows(() => KeyExpr.autocanonize("a/**/**"), Error, "Invalid key expression");
});
