---
name: zenoh-nostd-integrate
description: Add or troubleshoot zenoh-nostd in a consuming Rust application on a native host, Embassy MCU, or browser. Covers adapter selection, memory configuration, session lifecycles, and verified message exchange.
---

# Integrate zenoh-nostd

Work in the application's repository and preserve its runtime, networking, and
build conventions. This skill is self-contained: it can be copied without the
upstream checkout. Repository development tools, the example CLI, and example
scenarios are optional references, not application dependencies.

## Choose a concrete workflow

Load only the recipe matching the application's environment:

- [Native](references/native.md): select host dependencies and timer integration,
  then verify pub/sub/query behavior in the application.
- [Embassy](references/embassy.md): reuse an application's HAL/network stack and
  executor with bounded storage and target-specific build verification.
- [WASM](references/wasm.md): browser dependencies, WebSocket endpoints, and runtime
  checks for handshake, message exchange and teardown.

For example, “add a subscriber to this Embassy application” should preserve its
existing setup; “create a minimal native demo” should follow the native recipe.
Each recipe describes the observable results needed to finish that task.

## Establish the application context

Inspect its manifests, lockfile, target/toolchain configuration and existing
network/executor setup. Identify `std` versus `no_std`, allocator availability,
transport needs, memory limits, and whether a router is reachable. Ask only for
missing decisions that affect integration; do not assume an ESP board or Nix.

Resolve the exact zenoh-nostd version or Git revision from the application's
dependency graph. Use source, Rustdoc and examples for that revision; the API is
still evolving. Keep core and adapter revisions compatible. Check whether the
needed crates are published before choosing registry versus Git dependencies;
do not assume all platform adapters are available from crates.io.

The contracts below describe the current API. Check names and behavior against
the resolved revision before producing code or changing dependencies.

## Select transport and features

| Application | Starting point |
| --- | --- |
| Native host | `zenoh-std` and `StdLinkManager`; supply host time/runtime integration |
| Browser/WASM | `zenoh-wasm` and `WasmLinkManager`; router WebSocket endpoint; listening/brokering unsupported |
| Embassy networking | `zenoh-embassy` and `EmbassyLinkManager`; application-owned initialized network stack |
| Other network/runtime | Implement `ZLinkManager` and link traits, or adapt established async readers/writers with `EmbeddedIOLink` |

Use explicit dependency features. Core defaults are empty: `alloc` adds storage
alternatives and broker support without enabling `std`. Allocated operation needs
an application allocator; networking/HALs may require their own heap even when
Zenoh uses fixed storage. Choose logging at the application level. `cli` and
`console-usb` belong to example applications, not the core library.

For `zenoh-embassy`, select only needed TCP/UDP capabilities and a network medium
matching the driver; its defaults enable both transports. The application owns
Wi-Fi/Ethernet initialization and DHCP or static addressing. Current endpoints
use IPv4 addresses, and UDP listening is unsupported. A device's loopback address
cannot reach a router on another machine.

## Supply runtime and memory policy

- Provide an Embassy-compatible time driver while using the application's
  executor. The core uses Embassy futures/synchronization/timers without selecting
  a board or executor architecture. Firmware still owns critical-section/panic
  handlers, linker configuration, and hardware startup.
- For bare-metal custom entropy, enable `embedded` and register a suitable
  `getrandom` 0.2 custom backend in the final application; enabling the feature
  alone does not provide entropy. Check the resolved revision's backend requirements.
- Implement `ZSessionConfig`: choose the link manager, protocol buffer, and
  subscription/get/queryable registries independently. Count active subscriptions,
  pending outgoing gets, and queryable declarations; a wildcard is one registration.
  Fixed queryable storage separately bounds tracked incoming requests.
- Choose registry entry counts, key capacities in UTF-8 bytes, callback object
  storage and callback-future storage. Fixed tables reject excess entries/keys.
  `alloc` does not automatically switch fixed aliases to allocated maps; select
  `Alloc*Callbacks` explicitly when wanted. Socket buffers, UDP packet metadata,
  owned receive data, and application queues need their own memory budgets.

Callback entry count does not size the callback object. Current fixed aliases
reserve 16 inline bytes for captures and 128 for the returned future. A closure
capturing several queues/counters may exceed that even when the registry has free
entries. Select larger fixed storage or capture a single reference to preallocated
context; see [fixed callback storage](references/embassy.md#fixed-callback-storage).

## Drive and clean up the connection

Open a handshaken transport through `TransportLinkManager`, then borrow it with
`Session::new(&mut transport)`. Keep link resources alive for the transport and
poll `session.run()` alongside operations. Use a connection scope that drops
operations/driver/session before the transport. MCU sessions can use ordinary
locals; `zenoh::session!` and static builders are optional, one-shot alternatives.
Current `NoopRawMutex` access stays in one executor/thread context.

Callbacks borrow receive data and execute sequentially while session state is
locked. Copy needed data into owned bounded storage before returning. Avoid
awaiting slow output/full queues, or methods that reacquire session state such as
undeclaration, get completion/cancellation, and query finalization. Handle queue
saturation explicitly rather than blocking receive progress.

- Publish with `put(...).finish().await`; success is not remote acknowledgment.
  Flush adapter-owned TX buffers for normal finite completion while driving the
  session. Subscriptions need propagation time before publications are expected.
- Explicitly undeclare subscribers/queryables after successful work. Dropping a
  handle alone does not unregister callbacks. The supplied registries copy keys,
  but public handles may still borrow the caller's key text.
- A get can receive multiple replies before finalization or timeout. Observe
  completion or use its local `cancel` method after sending; timeout cleanup is
  lazy. This local cancellation is distinct from interrupting a transport future.
  A prior publication is not stored for `get` unless a queryable/storage serves it.
- For queryables, queue owned request data and its request ID, then reply and
  finalize once per accepted handler outside the callback. Reject work that
  cannot be queued. Dropping a query does not finalize it. The built-in owned-query
  channel requires a static session; scoped sessions can queue application-owned
  data/IDs. Inspect conversion-failure handling when choosing the built-in channel.
- Cancelling an operation or encountering a transport write error requires
  discarding the whole connection before further network I/O or awaited diagnostics.
  Recreate transport/session to continue; do not resume an interrupted write or
  attempt declaration cleanup on that connection.

## Verify in the consuming project

Use its build environment and target configuration; run Cargo from the directory
whose target configuration should apply. Check feature compatibility and perform
a release link for firmware, then exercise a minimal publication/subscription and
query/reply exchange with a reachable peer when available. Confirm bounded-queue
behavior and the teardown/reconnect path the application uses. Distinguish compile
checks from runtime and physical-board tests, and report what was actually run.
There is no requirement to run the upstream Nix/Just matrix in a consumer project.

For further detail, locate the matching revision in the
[upstream repository](https://github.com/eclipse-zenoh/zenoh-nostd). Consult its
`docs/architecture.md` application contract, public API Rustdoc and closest platform
example. Inspect that example's manifest for target-specific patches/features;
copy only the integration pieces the application needs. Do not import upstream
repository instructions or replace the application's own project layout.
