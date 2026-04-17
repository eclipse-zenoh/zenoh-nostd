/**
 * Test: KeyExpr operations.
 * ke_intersects / ke_includes delegate to WASM; tested here via the TS wrapper.
 * The pure string operations (join, concat, toString) are tested without WASM.
 */
import { assertEquals } from "jsr:@std/assert";
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
