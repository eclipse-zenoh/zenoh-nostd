# TypeScript API — Improvement Plan

**Target:** `platforms/zenoh-ts/ts/src/` — the hand-written TypeScript layer wrapping the
wasm-bindgen core, published as `@eclipse-zenoh/zenoh-ts` and claiming API-compatibility with
the upstream Eclipse zenoh-ts library.

**Method:** A guru-level review of every source file, independently re-verified against the code
by a second adversarial reviewer (which corrected three findings and added nine). All file:line
references below point at the current source.

**Legend:**
- 🟥 **High** — wrong behavior with no error surfaced, or blocks real consumption
- 🟧 **Medium** — correctness/ergonomics/honesty issue
- 🟨 **Low** — cleanup / style / minor type mismatch
- 🦀 **WASM-blocked** — the proper fix needs the Rust/WASM core, not just TS

---

## Tier 1 — Correctness bugs & silent-failure footguns

### 🟥 #2 — QoS options declared but silently dropped
`PutOptions` declares `priority`, `congestionControl`, `express`
(`session.ts:57-63`) and `PublisherOptions` declares `priority`, `congestionControl`, `express`,
`reliability` (`pubsub.ts:33-39`), but:
- `Session.put()` forwards only `encoding` + `attachment` (`session.ts:149-156`).
- `Publisher.put()` reads only `encoding`; the stored `_opts` is otherwise dead (`pubsub.ts:52-57`).
- The WASM `put` signature has no parameter for them (`pkg/zenoh_ts_wasm.d.ts:182`, `:22`).

A user setting `CongestionControl.Block` silently gets `Drop` semantics with no error.

**Fix:** wire them through the WASM `put` (🦀), or — until the core accepts them — **remove them
from the option types** so the API stops promising what it can't deliver.

### 🟥 #N4 — `delete()` is a silent no-op
`Session.delete()` (`session.ts:159-161`) and `Publisher.delete()` (`pubsub.ts:60-62`) `await`
WASM methods that are documented no-ops (`pkg/zenoh_ts_wasm.d.ts:157` "Delete notifications are not
supported yet by zenoh-nostd"; `:13`). The promise resolves successfully; nothing hits the wire.

**Fix:** make the no-op visible — `console.warn` once, or throw — rather than resolving as success. (🦀 for real delete support.)

### 🟧 #10 — `Parameters` truncates values containing `=`
`selector.ts:23-24` and `:50-53` use `pair.split("=", 2)`. JS `String.split(sep, limit)` discards
the remainder, so `"k=a=b=c".split("=",2)` → `["k","a"]` — the value `a=b=c` is silently truncated
to `a`.

**Fix:** `const i = pair.indexOf("="); k = pair.slice(0,i); v = pair.slice(i+1)`. Add a test.
*(The earlier "no percent-decoding" concern was dropped — upstream zenoh does not URL-decode either.)*

### 🟧 #14 — Delete-kind replies misreported as Put
`Reply.fromWasm()` (`pubsub.ts:164-171`) reads `key_expr`/`payload`/`encoding_id` but **not**
`js.sample.kind`, which exists (`pkg/zenoh_ts_wasm.d.ts:115`). `Reply.result()` then hardcodes
`SampleKind.Put` (`pubsub.ts:157-162`). Delete replies are silently reported as Put.

**Fix:** read `kind` in `fromWasm`, thread it into `result()`.

### 🟧 #7 — Dual-cased enums corrupt reverse-mapping
Every enum doubles each variant, e.g. `SampleKind { Put=0, PUT=0, Delete=1, DELETE=1 }`
(`enums.ts:32-37`; same pattern `:1-82`). For a numeric enum the **last** key wins the reverse map,
so `SampleKind[0] === "PUT"`. `Sample.toString()` uses `SampleKind[this._kind]`
(`sample.ts:72`) and therefore prints `kind=PUT` / `kind=DELETE`, not `Put`/`Delete`. It also
doubles every autocomplete entry.

**Fix:** pick one convention. Recommended: `as const` objects + union types (matches modern
upstream, eliminates reverse-map ambiguity). Minimal alternative: collapse to single-cased numeric enums.

---

## Tier 2 — Resource lifecycle & API honesty

### 🟧 #9 / #N8 — Channel iterator leaks & shared-queue stealing
`FifoChannel[Symbol.asyncIterator]()` (`channels.ts:59-69`) returns an object with only `next()`:
- No `return()`/`throw()` — `break`/early-return out of `for await` never `close()`s the channel,
  so the subscriber keeps filling it (leak).
- Each call shares `_queue`/`_waiters` (`channels.ts:16-17`), so two iterators steal each other's items.
- No `[Symbol.asyncDispose]`, so `await using` won't auto-close.

**Fix:** add `return()` that calls `close()`; add `[Symbol.asyncDispose]`; document single-consumer.

### 🟥🦀 #8 — Hardcoded `setTimeout(r, 100)` race band-aid
`declareQueryable` sleeps 100 ms "to give the router time to process the DeclareQueryable"
(`session.ts:250-252`). This is a latency tax on every declaration and still races on a slow
router/network.

**Fix:** await an ack from the WASM layer (🦀). Short-term: document loudly + file a core issue.

### 🟧🦀 #11 — `KeyExpr` neither canonicalizes nor validates
`KeyExpr` constructor just stores the string (`key_expr.ts:11-13`); `autocanonize()` is
`return new KeyExpr(expr)` (`key_expr.ts:16-18`) despite the docstring "Canonicalize and return".
No syntax validation anywhere, though the WASM exports `ke_intersects`/`ke_includes`
(`pkg/zenoh_ts_wasm.d.ts:206-211`).

**Fix:** wire to a WASM canonicalize/validate export (🦀, preferred), or fix the docstring to stop
claiming behavior it doesn't have.

### 🟧 #12 — No closed/undeclared guards
`Session._closed` is set in `close()` but never checked by `put`/`get`/`declare*`
(`session.ts:96`, `:120-132`, `:149`, `:166`, `:180`, `:222`, `:259`, `:279`). `isClosed()` is
advisory only (`session.ts:134`). `Publisher`/`Subscriber`/`Queryable`/`Querier` have no
"undeclared" flag — use-after-`undeclare()` hits freed WASM with an opaque error.

**Fix:** guard public methods; throw a clear `"session closed"` / `"already undeclared"`.

### 🟧 #N3 — Errors swallowed on both get paths
- Callback mode: `done.catch(console.error)` (`session.ts:305`, `pubsub.ts:221`) and the function
  returns `undefined` *before* the query runs — the caller can never observe a transport error.
- Channel mode: `done.then(() => channel.close()).catch(() => channel.close())`
  (`session.ts:324`, `pubsub.ts:236`) — a connection failure is indistinguishable from normal
  completion.

**Fix:** provide an error propagation path (surface to channel/handler), not just `console.error`.

### 🟧 #6 / #19 / #N9 — Stubs presented as working
- `info().zid()` returns a hardcoded **non-hex** `"wasm-zenoh-nostd"` through an unvalidated
  `ZenohId` (`session.ts:140-144`, `timestamp.ts:1-12`).
- `Liveliness` every method throws (`liveliness.ts:8`, `:15`, `:19`, `:29`, `:36`, `:41`).
- `Timestamp`/`ZenohId` are inert holders never populated; `Query.encoding()`/`attachment()` always
  return `undefined` (`query.ts:36-43`).
- `Encoding.withSchema(...)` (`encoding.ts:85`) is dropped at the boundary — only `id` is sent
  (`session.ts:151`, `pubsub.ts:54`) and inbound samples rebuild `new Encoding(id)` with
  `schema=undefined` (`sample.ts:31`), losing the schema on round-trip.

**Fix:** ensure each stub throws or is typed/doc'd as unimplemented rather than returning
plausible-looking junk. (🦀 to actually populate them.)

### 🟧 #13 — `replyDel` sends an empty Put, not a delete
`Query.replyDel()` calls `this._inner.reply(ke, new Uint8Array(0))` (`query.ts:58-60`). The WASM
`JsQuery` only exposes `reply` (Put) and `reply_err` (`pkg/zenoh_ts_wasm.d.ts:63`, `:67`) — no
delete-reply primitive. On the wire it is indistinguishable from an empty-payload Put.

**Fix:** add a delete-reply primitive (🦀); meanwhile fix the misleading docstring.

---

## Tier 3 — Type-system craftsmanship

### 🟧 #1 — Option fields typed `number` instead of the enums
`PutOptions.priority`/`congestionControl` (`session.ts:59-60`), `GetOptions.target`/`consolidation`
(`session.ts:70-71`), `QuerierOptions.target`/`consolidation` (`session.ts:88-89`),
`PublisherOptions.priority`/`congestionControl`/`reliability` (`pubsub.ts:36-38`) are all `number`,
while `Priority`/`CongestionControl`/`QueryTarget`/`ConsolidationMode`/`Reliability` are defined and
exported (`enums.ts`, `index.ts:29-39`).

**Fix:** use the enum types. (Lands naturally with #7.)

### 🟧 #5 — `get()`/`Querier.get()` return `T | undefined` discriminated at runtime
`Session.get()` returns `Promise<ChannelReceiver<Reply> | undefined>` (`session.ts:279-282`) and
`Querier.get()` likewise (`pubsub.ts:200`), keyed on whether `handler` was passed. Channel-mode
callers must handle a `undefined` that never occurs.

**Fix:** function **overloads** — `handler` present → `Promise<undefined>`; absent →
`Promise<ChannelReceiver<Reply>>`. Matches upstream.

### 🟨 #4 — WASM-boundary casts
`as unknown as (...)` appears at `session.ts:210`, `:249`, `:300`, `:319` and `pubsub.ts:216`,
`:231`; the inline `jsPayload as {key_expr...}` at `session.ts:200-201`. Root cause: wasm-bindgen
emits callbacks as bare `Function` (`pkg/zenoh_ts_wasm.d.ts:41`, `:149`, `:154`, `:167`).

**Fix:** one small typed adapter module for the callbacks; move the payload shape into
`Sample.fromWasm`.

### 🟨 #17 / #N1 / #N2 — Handler-model asymmetry & channel quirks
- `SubscriberOptions.handler` accepts `FifoChannel<Sample> | fn` (`session.ts:77-79`), but
  `QueryableOptions.handler` only `fn` (`session.ts:81-84`; channel always created internally,
  `:237`), and `GetOptions.handler`/`QuerierGetOptions.handler` only `fn`
  (`session.ts:74`, `pubsub.ts:180`).
- `FifoChannel.tryReceive()` ignores `_waiters` and closed state (`channels.ts:43-45`).
- FIFO overflow silently drops the **newest** item with no signal (`channels.ts:23-31`, doc `:13`) —
  diverges from upstream's backpressure model.

**Fix:** unify the handler type across `declare*`/`get`; fix `tryReceive`; document (or change) the
drop-newest policy.

### 🟨 #N5 — `ZBytes` is not actually immutable
The constructor stores the same `Uint8Array` reference (`z_bytes.ts:6-14`) and `toBytes()` returns
it (`:25-27`), so callers can mutate the backing buffer of a value type presented as immutable.

**Fix:** defensive copy on construct, or return a read-only view.

### 🟨 #N7 — No cancellation / AbortSignal
`get`/`Querier.get` accept only `timeout` (`session.ts:284`, `pubsub.ts:201`); there is no
`AbortSignal`, and pending `receive()` promises are only resolved by `close()` returning `null`
(`channels.ts:37-39`) — no rejection path.

**Fix:** consider accepting `AbortSignal` on `get`/`declare*`. Larger design item — propose first.

### 🟨 Batched low-effort cleanups
- **#3** — `Session.delete()` takes `PutOptions` but ignores it (param `_opts`, `session.ts:159`);
  `encoding` is meaningless for delete.
- **#15** — dead identical ternary in `extractKeParams`:
  `selector instanceof Selector ? selector.toString() : selector.toString()` (`session.ts:39-41`).
- **#16** — `Sample.keyexpr()` lowercase (`sample.ts:35`) vs `keyExpr()` elsewhere
  (`query.ts:12`, `pubsub.ts:47`, `:190`). **Keep** the lowercase name — it matches upstream — but
  note the internal inconsistency.
- **#18** — `ZBytes` constructor accepts `number[]` (`z_bytes.ts:6`) but `IntoZBytes` omits it
  (`z_bytes.ts:1`) and `from()` casts it away (`:16-19`).
- **#21** — `skipLibCheck: true` (`tsconfig.json:15`) hides drift between hand-written calls and the
  generated wasm `.d.ts`. *(Reframed from the original claim — the `Function` callbacks are
  sidestepped by the source-level casts in #4, not by `skipLibCheck`.)*

---

## Tier 4 — Packaging (blocks real npm consumption)

### 🟥 #20 / #N6 — Package ships raw TS and never emits a dist
- `main`/`exports` point at raw `./src/index.ts` (`package.json:6-9`).
- Every import uses explicit `.ts` specifiers (e.g. `session.ts:2-19`), requiring
  `allowImportingTsExtensions` + `noEmit` (`tsconfig.json:16-17`).
- `build`/`check` are `tsc --noEmit` (`package.json:12-14`) — nothing is ever emitted, so
  `declaration`/`declarationMap`/`sourceMap`/`outDir` (`tsconfig.json:11-13`) are dead settings.
- `exports` has no `"types"` and no `import`/`require` split; `pkg/` (the wasm) is `exclude`d
  (`tsconfig.json:20`) and absent from `exports`/`files`.

Net: the published artifact is consumable by Deno/bundlers (`deno.json`) but **not** by plain Node
or `tsc` (no JS, no `.d.ts`, `.ts` specifiers illegal under NodeNext).

**Decision required:** is this package **Deno-only by design** (then document it and drop the dead
`dist` config), or must it be a **publishable npm artifact** (then emit JS + `.d.ts`, rewrite `.ts`
specifiers, add `exports.types` + conditional exports, ship `pkg/`)? This gates a whole workstream.

---

## Open decisions before execution

1. **Tier 1 + Tier 2 pure-TS fixes** (bug fixes, lifecycle guards, error propagation) — proceed with all?
2. **Enum redesign (#7):** `as const` + union types (recommended), or collapse to single-cased numeric enums?
3. **Dropped QoS options (#2):** remove from the types now, or keep with a `@deprecated`/"not yet wired" doc?
4. **Packaging (#20):** Deno-only by design, or a publishable npm artifact?

## WASM-blocked items (TS-honesty fix now, full fix needs the Rust core)
#2 (QoS wiring) · #8 (declare ack) · #11 (canonicalize/validate) · #13 (delete-reply) ·
#N4 (delete) · #6/#19/#N9 (populate stubs).
