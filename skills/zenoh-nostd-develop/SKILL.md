---
name: zenoh-nostd-develop
description: Change, review, test, or port the zenoh-nostd repository itself, including core libraries, platform adapters, and example applications.
---

# Develop zenoh-nostd

Work from the zenoh-nostd repository root. Load only the guide relevant to the
change. Repository paths are relative to the checkout root; Markdown links are
relative to this skill's canonical directory.

## Find the right layer

| Task | Start with |
| --- | --- |
| Protocol encoding or state machines | `crates/zenoh-proto`, `crates/zenoh-sansio`; [architecture](../../docs/architecture.md) |
| Session, callbacks, storage, or broker API | `crates/zenoh-nostd`; [application contract](../../docs/architecture.md#application-contract) |
| Network adapter or new MCU | `platforms/`, board project; [porting steps](../../docs/architecture.md#add-an-mcu-target) |
| CLI command or startup scenario | `examples/common/src/cli`, `examples/common/src/scenarios`; [applications](../../docs/applications.md) |
| Console or board startup | `examples/embassy`; [MCU consoles and board guides](../../examples/embassy/README.md) |
| Build, test, or documentation | [development commands](../../docs/README.md#development-commands), root/example justfiles, CI |

Resolve repository paths from the checkout being changed. When this skill is
loaded through a symlink, resolve its canonical directory before following links.

Before changing a public API or its callers, read its Rustdoc at the current
revision, including method contracts and feature gates. Start with the task index
in `crates/zenoh-nostd/src/lib.rs`; for adapters, start with their `src/lib.rs`.
Treat those contracts as the API reference. Update them alongside behavior changes:
defaults, errors, capacities, lifetimes, callback locking, cancellation and cleanup.
Compile examples as doctests; use `no_run` only when execution needs external I/O,
and distinguish compilation from runtime evidence. Run `just check-docs` to catch
missing public documentation, broken links and invalid examples across libraries.

## Preserve repository boundaries

- Keep core crates as `no_std` libraries with fixed-capacity operation and optional
  `alloc`. HALs, board selection, executor entry points, console drivers, linker
  settings, and credentials belong to applications.
- Keep adapters and examples in their independent Cargo workspaces so target
  features and logging backends do not unify into the core workspace.
- Each target builds one `zenoh-app`: default `cli`, optional headless startup,
  UART by default on MCUs, and `console-usb` enabling `cli` when supported.
  Reuse scenarios in `examples/common`, MCU helpers directly in `examples/embassy`,
  and family setup under its family directory; keep pins/HAL policy in board projects.
- Poll the session driver alongside operations. Preserve separate transport and
  link-resource lifetimes and drop the driver/session before its transport.
  Static cells are one-shot storage, not a reconnect mechanism. Keep current
  `NoopRawMutex` access in one executor/thread context.
- Keep receive callbacks short: they run under the session-state lock. Queue
  bounded data rather than awaiting console output, full channels, or session
  operations that need that lock. Finalize accepted query work outside callbacks.
- Cancellation or transport write errors end the connection. Drop operation,
  driver, session and transport before awaiting diagnostics or other cleanup.
  Only successful operations may clean up declarations and retain a connection.
  Example frontend buffers remain bounded with `alloc`; scenario changes must
  work through both CLI and typed headless entry points.

## Build and verify the change

Use the repository Nix environment: `nix develop -c just <recipe>`. Run Cargo
from a standalone project's directory when it needs its `.cargo/config.toml`;
`--manifest-path` alone does not select that configuration. ESP32-S3 uses
`cargo +esp` and release links; C6 and STM32 use stable Rust. Preserve application
lockfiles and use `--locked` for reproducible builds.

| Changed area | Relevant verification |
| --- | --- |
| Core | `just test`, affected fixed/`alloc` tests; `just check-portability` for dependency/portability changes |
| Scenarios or CLI | Common-library tests, `just test-cli` and/or `just test-headless`; `just check-headless` for feature isolation |
| Board or adapter | Affected adapter checks and release links for changed storage/console modes |
| Public API docs | `just check-docs`; check public re-exports and rendered contracts for affected features |
| Other documentation or skills | Check links, anchors, paths, recipe names, skill frontmatter and discovery links |

Before finishing code changes, run `just check` in Nix and `just fmt`; the recipes
cover the independent projects. `just check` does not link every firmware variant
or run every native integration suite. Documentation-only changes need no firmware
rebuild. See the [command guide](../../docs/README.md#development-commands) for scope.

Firmware verification recipes (`build-firmware*`) use placeholders and never flash.
For hardware tests, rebuild with actual local settings using the board recipes,
keep credentials out of Git/output, and release the serial terminal before flashing.
Report checks actually run and physical-board coverage separately; compilation
and mock USB tests do not establish networking, enumeration, or reconnect behavior.
Keep dated validation logs in PR descriptions rather than permanent contributor docs.
