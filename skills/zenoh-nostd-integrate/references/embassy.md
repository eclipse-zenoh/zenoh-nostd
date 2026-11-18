# Integrate into an existing Embassy application

Keep the application's HAL, network device, executor, pin assignments and linker
configuration. Read its target and dependency versions before changing features.
No upstream `zenoh-app`, CLI, USB console or ESP-specific crate is required.

## Assemble the integration

1. Select compatible `zenoh-nostd` and `zenoh-embassy` revisions. For fixed TCP,
   disable adapter defaults, enable `tcp` and the medium required by the existing
   network driver (`medium-ip` or `medium-ethernet`). Enable core `embedded` for
   the custom entropy path. Match the adapter's `embassy-net` version: currently 0.8.
2. Reuse the initialized `embassy_net::Stack` and keep its runner polled. Retain
   the project's DHCP/static address setup; wait for network readiness before
   connecting to the router's reachable LAN IPv4 address.
3. Supply suitable entropy through the final application's `getrandom` 0.2 custom
   backend. Reuse its time driver, critical-section implementation and panic handler.
   These belong in the application; feature flags alone do not implement them.
4. Construct `EmbassyLinkManager` with buffer size and socket count chosen from
   the application's RAM budget. Current UDP metadata storage is tied to the
   adapter's buffer-size generic, not an independently selectable queue parameter.
   Count stack socket resources as well as adapter slots. TCP-only excludes UDP
   metadata allocations; do not enable UDP just because an upstream example does.
5. Implement `ZSessionConfig` with explicit protocol buffer, callback counts and
   key/storage sizes. Do not copy example counts as library limits. Choose inline
   callback/future storage large enough for captures and returned futures.
6. Open a local transport, construct `Session::new`, and poll its driver alongside
   operations in one executor/thread context. Allocate callback-captured queues
   before the session so they outlive its transport borrow. Retain bounded owned
   data when receive callbacks return. Finalize deferred queries outside callbacks.

For pub/sub, allow declaration propagation before expecting remote samples. For
queryables, use a bounded nonblocking application queue of owned data/request IDs,
reject on saturation, and finalize once per accepted handler. The built-in owned
query channel currently requires a static session; static storage is not required
for an application-owned queue with scoped sessions.

## Fixed callback storage

A dyn-utils const assertion during callback construction can mean the closure's
captures exceed the alias's inline storage, rather than a lifetime/API failure.
For larger captures, use the public storage re-export; a direct `dyn-utils`
dependency is unnecessary. For example, inside `impl ZSessionConfig`:

```rust
type QueryableCallbacks<'transport, 'resources> =
    zenoh_nostd::session::FixedCapacityQueryableCallbacks<
        'transport,
        'resources,
        Self,
        16,
        zenoh_nostd::session::zenoh::storage::Raw<64>,
    >
where
    Self: 'resources,
    'resources: 'transport;
```

This reserves 16 registry entries/counters and 64 bytes for each callback object;
the application's work queue is configured separately. Alternatively capture one
reference to a context that owns the queue/counters. Increase future storage as
well when an async callback needs it; these are independent parameters. Recheck
the alias defaults in the application's resolved SDK revision.

## Acceptance

- The project's target check and release link succeed with its existing HAL and
  executor; fixed-capacity operation introduces no Zenoh allocator requirement.
- Observe an actual publication in both directions and a finalized query response
  using a reachable peer. Test the queue-full path without blocking the driver.
- For cancellation/write failure, drop operation, driver/session and transport;
  recreate a connection before further network I/O. Keep diagnostics outside the
  cancelled connection scope.
- Report compilation separately from physical-board behavior. If a board or
  router is unavailable, identify that coverage gap; do not fabricate runtime results.

Run Cargo from the application's directory so its `.cargo/config.toml` is loaded.
Use the target/toolchain the project already selects; Xtensa and RISC-V ESP boards
have different requirements. Existing application features may independently need
a heap for the HAL/network stack even when Zenoh uses fixed storage.
