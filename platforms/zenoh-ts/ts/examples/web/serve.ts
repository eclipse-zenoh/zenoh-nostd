#!/usr/bin/env -S deno run --allow-net --allow-read --allow-env
//
// Static file server for the Zenoh web demo.
//
// Local usage (from ts/):
//   deno run --allow-net --allow-read --allow-env examples/web/serve.ts
//   Opens http://localhost:8000/examples/web/ — router at ws/127.0.0.1:7447.
//
// Custom port:
//   PORT=9090 deno run --allow-net --allow-read --allow-env examples/web/serve.ts
//
// Public address (e.g. zenoh.mysite.me):
//   HOST=zenoh.mysite.me PORT=8000 \
//     deno run --allow-net --allow-read --allow-env examples/web/serve.ts
//   The default router locator in index.html is rewritten to ws/HOST:7447.
//   Make sure DNS points to this machine, port 8000 and port 7447 are open,
//   and zenohd is running with -l ws/0.0.0.0:7447.
//

const PORT = parseInt(Deno.env.get("PORT") ?? "8000");
// When HOST is set, index.html is served with the default router locator
// rewritten to ws/HOST:7447 so visitors connect to the right router.
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

    // When HOST is set, patch the default router locator so the browser
    // connects to the router at the same public host on port 7447.
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

Deno.serve(
  {
    port: PORT,
    hostname: "0.0.0.0",
    onListen({ port }) {
      const displayHost = HOST || "localhost";
      const routerHost  = HOST || "127.0.0.1";
      console.log(`Serving  ${ROOT}`);
      console.log(`Open  →  http://${displayHost}:${port}/examples/web/`);
      console.log(`Router   ws/${routerHost}:7447  must be reachable`);
    },
  },
  handler,
);
