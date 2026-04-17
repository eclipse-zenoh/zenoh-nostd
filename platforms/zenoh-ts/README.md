# zenoh-ts

TypeScript/WebAssembly bindings for [zenoh-nostd](../../README.md) — a `#![no_std]`
implementation of the [Zenoh](https://zenoh.io) protocol.

The bindings compile the Rust core to a WASM module and expose a TypeScript API
that is wire-compatible with the upstream
[`@eclipse-zenoh/zenoh-ts`](https://github.com/eclipse-zenoh/zenoh-ts) package,
so the same test files and application code work against both.

---

## Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| [Rust](https://rustup.rs) | 1.91+ | Compile the WASM module |
| [wasm-bindgen-cli](https://rustwasm.github.io/wasm-bindgen/reference/cli.html) | matching `wasm-bindgen` crate | Generate JS/TS glue |
| [just](https://github.com/casey/just) | any recent | Task runner |
| [Deno](https://deno.land) | 2.x | Run TypeScript examples and tests |

Install `wasm-bindgen-cli` once and keep it in sync with the crate version:

```sh
cargo install wasm-bindgen-cli
```

---

## Building

All build commands are run from the `platforms/zenoh-ts/` directory.

### Full build (WASM + TypeScript type-check)

```sh
just build
```

This runs two steps in order:

1. **`just build-wasm`** — compiles the Rust crate to `wasm32-unknown-unknown`,
   runs `wasm-bindgen` to produce `ts/pkg/zenoh_ts_wasm.{js,d.ts,wasm}`, then
   applies a small post-build patch that wraps the embassy-time `setTimeout`
   bindings so outstanding timers can be cancelled on session close.

2. **`just check-ts`** — runs `npx tsc --noEmit` to type-check the TypeScript
   sources without emitting any output.

### Build steps individually

```sh
just build-wasm-bin      # cargo build → .wasm binary only
just build-wasm-bindgen  # wasm-bindgen + patch (implies build-wasm-bin)
just build-wasm          # alias for build-wasm-bindgen
just check-ts            # TypeScript type-check only
```

### Optional: shrink the WASM binary

```sh
# requires binaryen (brew install binaryen  /  apt install binaryen)
just wasm-opt
```

---

## Running Tests

Tests are also run from `platforms/zenoh-ts/`.

### Pure TypeScript tests (no router needed)

These tests cover channels, encoding, key expressions, and selector/parameters
parsing entirely in TypeScript — no network connection required.

```sh
just test-pure
```

### Integration tests (router required)

Start a Zenoh router with WebSocket support in a separate terminal:

```sh
just router
# equivalent: zenohd -l ws/127.0.0.1:10000
```

Then run the integration tests:

```sh
just test-integration
```

The suite runs the same test files used by the upstream
`@eclipse-zenoh/zenoh-ts` package without any source modification:

| Test file | Tests |
|-----------|-------|
| `tests/src/z_api_pub_sub.ts` | Put / Subscribe round-trip |
| `tests/src/z_api_queryable_get.ts` | Get (callback), Get (channel), Querier Get (callback), Querier Get (channel), multiple replies |

### Run everything

```sh
just test   # pure + integration (router must be running)
```

---

## Running Examples

### Deno command-line examples

Start the router first (`just router`), then run examples from `platforms/zenoh-ts/`:

```sh
# Publish at 1 Hz on demo/example
just example z_pub

# Subscribe and print received samples
just example z_sub

# Issue a get and print replies
just example z_get

# Declare a queryable and reply to each query
just example z_queryable

# Custom endpoint or key
just example z_pub -- -e ws/192.168.1.10:7447 -k my/key -v "hello"
```

All examples accept `-e <locator>`, `-k <key-expression>`, and (where applicable)
`-v <value>` flags. Run any example without flags to use the defaults.

### Browser web demo

The web demo is a single HTML page that lets you create publishers, subscribers,
queryables, and issue get queries directly from the browser.

Start the router on port 7447 (the browser WebSocket default):

```sh
just router-ws          # from platforms/zenoh-ts/
# or directly:
zenohd -l ws/127.0.0.1:7447
```

Then serve the example.  The server **must** be launched from `ts/` so that
the relative import `../../pkg/zenoh_ts_wasm.js` resolves correctly:

```sh
# Option A — from platforms/zenoh-ts/ (uses justfile)
just serve-web

# Option B — directly from ts/
cd ts
deno run --allow-net --allow-read examples/web/serve.ts
```

Open **http://localhost:8000/examples/web/** in any modern browser.

The page lets you:
- **Connect** to any Zenoh router via its WebSocket locator
- **Declare publishers** with a key expression, payload text, and an optional
  auto-publish period (ms); use "Publish now" for manual sends
- **Declare subscribers** and watch received samples stream in per-subscriber
  message feeds
- **Declare queryables** with a configurable reply text; incoming queries are
  shown and auto-replied
- **Issue get** queries with selector, parameters, payload, and timeout; replies
  appear colour-coded (green = ok, red = err)
- **Remove** any entity at any time; disconnect cleanly

---

## Hello World

The following examples show the most common patterns. All imports come from
`@eclipse-zenoh/zenoh-ts`, which resolves to `ts/src/index.ts` in the local
import map.

### Publish and subscribe

**publisher.ts**

```typescript
import { open, Config, ZBytes, Encoding } from "@eclipse-zenoh/zenoh-ts";

const session = await open(new Config("ws/127.0.0.1:7447"));
const pub = await session.declarePublisher("hello/world", {
    encoding: Encoding.TEXT_PLAIN,
});

for (let i = 0; i < 5; i++) {
    const msg = new ZBytes(`Hello, Zenoh! [${i}]`);
    console.log(`Putting: ${msg}`);
    await pub.put(msg);
    await new Promise((r) => setTimeout(r, 1000));
}

await pub.undeclare();
await session.close();
```

**subscriber.ts**

```typescript
import { open, Config } from "@eclipse-zenoh/zenoh-ts";

const session = await open(new Config("ws/127.0.0.1:7447"));
const sub = await session.declareSubscriber("hello/world");

console.log("Waiting for messages … (Ctrl+C to stop)");
for await (const sample of sub.receiver()) {
    console.log(`Received on '${sample.keyexpr()}': ${sample.payload()}`);
}
```

Run the subscriber first, then the publisher (both require `--allow-net`):

```sh
deno run --allow-net subscriber.ts &
deno run --allow-net publisher.ts
```

### Get / Queryable (request–reply)

**queryable.ts** — server side

```typescript
import { open, Config, ZBytes } from "@eclipse-zenoh/zenoh-ts";

const session = await open(new Config("ws/127.0.0.1:7447"));
const queryable = await session.declareQueryable("hello/world");

console.log("Serving queries … (Ctrl+C to stop)");
for await (const query of queryable.receiver()) {
    console.log(`Query on '${query.selector()}'`);
    await query.reply(query.keyExpr(), new ZBytes("Hello from the queryable!"));
    await query.finalize();
}
```

**get.ts** — client side

```typescript
import { open, Config } from "@eclipse-zenoh/zenoh-ts";

const session = await open(new Config("ws/127.0.0.1:7447"));
const replies = await session.get("hello/world", { timeout: 5_000 });

for await (const reply of replies) {
    if (reply.isOk()) {
        const sample = reply.result();
        console.log(`Reply from '${sample.keyexpr()}': ${sample.payload()}`);
    }
}

await session.close();
```

### Using `await using` (explicit resource management)

TypeScript 5.2+ and Deno support `await using`, which calls `close()` /
`undeclare()` automatically at the end of the scope:

```typescript
import { open, Config } from "@eclipse-zenoh/zenoh-ts";

await using session = await open(new Config("ws/127.0.0.1:7447"));
await using sub = await session.declareSubscriber("hello/world");

const sample = await sub.receiver()!.receive();
console.log(`Got: ${sample?.payload()}`);
// session and sub are closed automatically here
```

---

## Project Layout

```
platforms/zenoh-ts/
├── Cargo.toml              # Rust crate (zenoh-ts-wasm, cdylib)
├── src/                    # Rust WASM bindings
│   └── lib.rs
├── justfile                # Build / test recipes
├── scripts/
│   └── patch-wasm-js.ts    # Post-build JS patcher (timer tracking)
└── ts/
    ├── tsconfig.json
    ├── src/                # TypeScript API
    │   ├── index.ts        # Public exports
    │   ├── session.ts      # Session, open()
    │   ├── pubsub.ts       # Publisher, Subscriber, Queryable, Querier, Reply
    │   ├── query.ts        # Query (queryable side)
    │   ├── z_bytes.ts      # ZBytes payload type
    │   ├── encoding.ts     # Encoding constants
    │   ├── key_expr.ts     # KeyExpr
    │   ├── selector.ts     # Selector, Parameters
    │   ├── channels.ts     # FifoChannel, RingChannel
    │   └── …
    ├── pkg/                # Generated WASM + JS glue (git-ignored)
    ├── examples/deno/src/  # Runnable Deno examples
    └── tests/src/          # Integration test suites
```

---

## Architecture Notes

- **No Node.js** — the runtime target is Deno. The generated WASM module uses the
  `web` bindgen target and is loaded with a standard `import init from
  "../pkg/zenoh_ts_wasm.js"`.
- **Each `Session.open()` creates an independent WebSocket connection.** The
  router treats each session as a separate peer, which is required for correct
  query routing between sessions in the same process.
- **Embassy-time timers** — the WASM runtime drives timers via `setTimeout`. The
  post-build patch script tracks all outstanding timer IDs so that
  `cancelAllWasmTimers()` can clean them up on `session.close()`, preventing
  resource-leak errors in Deno's test sanitizer.
