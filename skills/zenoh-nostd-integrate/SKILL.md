---
name: zenoh-nostd-integrate
description: Add or troubleshoot zenoh-nostd in a consuming Rust application on a native host, Embassy MCU, or browser. Covers adapter selection, memory configuration, session lifecycles, verified message exchange, and adding Zenoh commands to an existing embedded-cli console.
---

# Integrate zenoh-nostd

Work in the consuming application and reuse its build environment, executor and
network setup. Inspect its manifests, lockfile, target configuration, allocator
availability, memory budget and access to a peer.

## Choose the relevant recipe

- [Native](references/native.md) for host sockets and time drivers.
- [Embassy](references/embassy.md) for an MCU with an application-owned network stack.
- [WASM](references/wasm.md) for browser builds and WebSocket transport.
- [Embedded CLI](references/embedded-cli.md) when adding Zenoh commands to an existing
  `embedded-cli` console.

For a custom transport, start with `ZLinkManager` and the link traits in the
core crate's platform Rustdoc. Inspect `EmbeddedIOLink` for split async I/O.

## Resolve and read the API

1. Resolve compatible core and adapter versions or Git revisions from the
   dependency graph. Check availability before choosing registry or Git dependencies.
   If present, compare the installed skill's `SOURCE_REVISION` with the SDK revision.
2. Open the public task index in the matching `zenoh-nostd` crate Rustdoc. Use
   `cargo doc --locked -p zenoh-nostd` with the application's target and features,
   or read the matching crate's Rustdoc comments in source. Check the type and
   method contracts used by the task, including errors and cleanup.
3. Use the core feature table and adapter crate docs to select features,
   transports and network media. Check time, entropy and allocator requirements
   against the application's existing runtime. Use the matching platform example
   for setup patterns and verify any required manifest patches.

## Apply the session contracts

- Configure `ZSessionConfig` for packet and registry capacity. Check callback
  capture and future storage independently, and budget adapter buffers and
  application queues. See [fixed callback storage](references/embassy.md#fixed-callback-storage)
  when diagnosing an inline-storage limit.
- Use the scoped connection example in the core Rustdoc. Keep link resources
  alive, poll `Session::run` alongside operations, and check the `Session`
  contract for executor access and drop order.
- Keep receive callbacks short. Copy needed data into bounded owned storage,
  handle saturation, and defer work that locks session state. For queryables,
  check how to reject work when a queue is full and finalize accepted requests.
- Read the operation's normal cleanup contract, including flushing,
  undeclaration and reply completion where applicable. For a cancelled transport
  operation or write failure, discard the connection before awaiting diagnostics
  or reconnecting. Check `GetResponses::cancel` separately for sent queries.

## Verify in the application

Build on the application's target and release-link firmware. Observe the
requested message exchange with a reachable peer, allowing declarations to
propagate before checking delivery. Exercise queue saturation and reconnect when
those paths are involved. Report the contracts and examples used, and distinguish
compilation, observed network behaviour and physical-board coverage.

The entire skill folder can be copied as a unit. Use the exact dependency's source or
the [upstream repository](https://github.com/eclipse-zenoh/zenoh-nostd) to find
revision-matched Rustdoc and examples.
