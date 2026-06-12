/**
 * Test: Selector parameter parsing (pure TypeScript, no network or WASM required).
 */
import { assertEquals } from "jsr:@std/assert";
import { Parameters } from "../../src/index.ts";

// Re-implement the selector parser from session.ts for testing
function parseSelector(sel: string): [string, string | undefined] {
    const i = sel.indexOf("?");
    return i >= 0 ? [sel.slice(0, i), sel.slice(i + 1)] : [sel, undefined];
}

Deno.test("parseSelector: no parameters", () => {
    const [ke, params] = parseSelector("demo/example");
    assertEquals(ke, "demo/example");
    assertEquals(params, undefined);
});

Deno.test("parseSelector: with parameters", () => {
    const [ke, params] = parseSelector("demo/example?foo=bar&baz=1");
    assertEquals(ke, "demo/example");
    assertEquals(params, "foo=bar&baz=1");
});

Deno.test("parseSelector: empty parameters string", () => {
    const [ke, params] = parseSelector("demo/example?");
    assertEquals(ke, "demo/example");
    assertEquals(params, "");
});

Deno.test("parseSelector: wildcard key expression with parameters", () => {
    const [ke, params] = parseSelector("demo/**?encoding=text/plain");
    assertEquals(ke, "demo/**");
    assertEquals(params, "encoding=text/plain");
});

Deno.test("parseSelector: ? in parameters does not split again", () => {
    const [ke, params] = parseSelector("a/b?x=1?y=2");
    assertEquals(ke, "a/b");
    assertEquals(params, "x=1?y=2");
});

// ── Parameters: key/value parsing ──────────────────────────────────────────────

Deno.test("Parameters.get returns the value for a key", () => {
    const p = new Parameters("foo=bar&baz=1");
    assertEquals(p.get("foo"), "bar");
    assertEquals(p.get("baz"), "1");
    assertEquals(p.get("missing"), undefined);
});

Deno.test("Parameters: value containing '=' is not truncated", () => {
    const p = new Parameters("expr=a=b=c&k=v");
    assertEquals(p.get("expr"), "a=b=c");
    assertEquals(p.get("k"), "v");
    assertEquals(p.iter(), [["expr", "a=b=c"], ["k", "v"]]);
    assertEquals(p.values(), ["a=b=c", "v"]);
});

Deno.test("Parameters: bare key (no '=') has empty-string value", () => {
    const p = new Parameters("flag&k=v");
    assertEquals(p.get("flag"), "");
    assertEquals(p.containsKey("flag"), true);
    assertEquals(p.iter(), [["flag", ""], ["k", "v"]]);
});

Deno.test("Parameters: empty value after '='", () => {
    const p = new Parameters("k=");
    assertEquals(p.get("k"), "");
    assertEquals(p.iter(), [["k", ""]]);
});

Deno.test("Parameters: insert/remove round-trip preserves '=' in values", () => {
    const p = new Parameters().insert("expr", "x=y");
    assertEquals(p.get("expr"), "x=y");
    const removed = p.insert("k", "v").remove("k");
    assertEquals(removed.get("expr"), "x=y");
    assertEquals(removed.get("k"), undefined);
});
