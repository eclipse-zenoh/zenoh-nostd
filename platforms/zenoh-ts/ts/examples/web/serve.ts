#!/usr/bin/env -S deno run --allow-net --allow-read --allow-env
//
// Minimal static file server for the Zenoh web demo.
//
// Usage (from ts/):
//   deno run --allow-net --allow-read --allow-env examples/web/serve.ts
//
// Custom port:
//   PORT=9090 deno run --allow-net --allow-read --allow-env examples/web/serve.ts
//

const PORT = parseInt(Deno.env.get("PORT") ?? "8000");
// Serve from the ts/ root so that the relative import ../../pkg/ inside
// index.html resolves correctly at http://localhost:<PORT>/pkg/…
const ROOT = new URL("../../", import.meta.url).pathname;

const MIME: Record<string, string> = {
    ".html": "text/html; charset=utf-8",
    ".js":   "application/javascript; charset=utf-8",
    ".ts":   "application/javascript; charset=utf-8",
    ".wasm": "application/wasm",
    ".css":  "text/css; charset=utf-8",
    ".json": "application/json",
    ".map":  "application/json",
};

function extOf(path: string): string {
    const i = path.lastIndexOf(".");
    return i >= 0 ? path.slice(i) : "";
}

async function handler(req: Request): Promise<Response> {
    const url  = new URL(req.url);
    let   path = decodeURIComponent(url.pathname);

    // Serve index.html for directory-style requests.
    if (path.endsWith("/")) path += "index.html";

    const filePath = ROOT + path.replace(/^\//, "");

    try {
        const data = await Deno.readFile(filePath);
        const mime = MIME[extOf(filePath)] ?? "application/octet-stream";
        return new Response(data, {
            headers: { "Content-Type": mime },
        });
    } catch {
        return new Response(`404 Not Found: ${path}`, { status: 404 });
    }
}

// onListen fires only after the socket is successfully bound, so the URL
// printed here is always reachable.
Deno.serve(
    {
        port: PORT,
        onListen({ hostname, port }) {
            const host = hostname === "0.0.0.0" ? "localhost" : hostname;
            console.log(`Serving  ${ROOT}`);
            console.log(`Open  →  http://${host}:${port}/examples/web/`);
            console.log(`Router   ws/127.0.0.1:7447  must be reachable`);
        },
    },
    handler,
);
