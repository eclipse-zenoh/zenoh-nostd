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
// Public address (e.g. zenoh.corsaro.me):
//   HOST=zenoh.corsaro.me PORT=8000 deno run --allow-net --allow-read --allow-env examples/web/serve.ts
//   The server binds on 0.0.0.0:PORT and rewrites the default router locator in
//   index.html to ws/zenoh.corsaro.me:7447 so visitors get the right default.
//   Make sure DNS points to this machine, the firewall is open on PORT, and
//   zenohd is running with -l ws/0.0.0.0:7447.
//

const PORT = parseInt(Deno.env.get("PORT") ?? "8000");
// When HOST is set to a hostname or public IP the server:
//   1. Binds on 0.0.0.0 (all interfaces) so the OS routes traffic in.
//   2. Rewrites the default router locator in index.html to ws/HOST:7447
//      so browsers connecting from the public internet get the right default.
// When HOST is not set the server stays local (ws/127.0.0.1:7447 default).
const HOST = Deno.env.get("HOST") ?? "";

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
        const mime = MIME[extOf(filePath)] ?? "application/octet-stream";

        // When HOST is set, patch the default router locator in index.html so
        // visitors connecting from the public internet get ws/HOST:7447 rather
        // than the local ws/127.0.0.1:7447 that is baked into the source file.
        if (HOST && filePath.endsWith("index.html")) {
            let html = await Deno.readTextFile(filePath);
            html = html.replace(
                'value="ws/127.0.0.1:7447"',
                `value="ws/${HOST}:7447"`,
            );
            return new Response(html, { headers: { "Content-Type": mime } });
        }

        const data = await Deno.readFile(filePath);
        return new Response(data, { headers: { "Content-Type": mime } });
    } catch {
        return new Response(`404 Not Found: ${path}`, { status: 404 });
    }
}

// onListen fires only after the socket is successfully bound, so the URL
// printed here is always reachable.
Deno.serve(
    {
        port: PORT,
        hostname: "0.0.0.0",
        onListen({ port }) {
            const displayHost = HOST || "localhost";
            console.log(`Serving  ${ROOT}`);
            console.log(`Open  →  http://${displayHost}:${port}/examples/web/`);
            console.log(`Router   ws/${displayHost}:7447  must be reachable`);
        },
    },
    handler,
);
