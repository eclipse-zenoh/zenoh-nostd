/**
 * Test: Selector parameter parsing (pure TypeScript, no network or WASM required).
 */
import { assertEquals } from "jsr:@std/assert";

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
