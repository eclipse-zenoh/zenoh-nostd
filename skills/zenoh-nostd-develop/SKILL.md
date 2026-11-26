---
name: zenoh-nostd-develop
description: Change, review, test, or port the zenoh-nostd repository itself, including core libraries, platform adapters, and example applications.
---

# Develop zenoh-nostd

Work from the zenoh-nostd repository root. Load only the guide relevant to the
change. Repository paths are relative to the checkout root; Markdown links are
relative to this skill's canonical directory.

## Before editing

1. Read [CONTRIBUTING.md](../../CONTRIBUTING.md), any applicable `AGENTS.md`,
   and the [development environment setup](../../docs/README.md#set-up).
   Use that setup and check the affected project's toolchain, target, Cargo
   features, and workspace boundaries before choosing build commands.
2. Find the owning code, callers, and existing tests. Read the relevant
   Rustdoc and examples at this checkout when changing public behavior.
3. Keep the patch to one concern. Reuse existing code and interfaces where they
   fit. Keep unrelated refactors and formatting out of the diff. For a new
   feature, public API change, or large refactor, follow the contributor guide's
   process for agreeing on the approach.

## Find the right layer

| Task | Start with |
| --- | --- |
| Protocol encoding or state machines | `crates/zenoh-proto`, `crates/zenoh-sansio`; [architecture](../../docs/architecture.md) |
| Session, callbacks, storage, or broker API | `crates/zenoh-nostd`; [application contract](../../docs/architecture.md#application-contract) |
| Network adapter or new MCU | `platforms/`, board project; [porting steps](../../docs/architecture.md#add-an-mcu-target) |
| Reusable CLI schema or inbox | [`crates/zenoh-cli`](../../crates/zenoh-cli/README.md); [consumer console recipe](../zenoh-nostd-integrate/references/embedded-cli.md) |
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
- Keep shared command definitions and the bounded inbox in `zenoh-cli`.
  Applications choose which commands to support, convert parsed arguments to
  owned requests, and execute them using their own session and transport policy.
- For session changes, check the Rustdoc for driver polling, transport and
  resource lifetimes, callback locking, and cleanup after cancellation or write
  failure. Keep example callbacks short and queues bounded with `alloc` as well
  as with fixed storage. Verify scenario changes through both CLI and typed
  headless entry points.

## Review the Rust change

Review the affected paths before finalizing the patch:

- **Concurrency and cancellation.** Check which task or executor owns state,
  what is held across each await, and how callbacks, backpressure, and a
  cancelled future affect progress and cleanup.
- **Errors and partial work.** Trace capacity and I/O failures to the caller.
  Check error propagation, state left behind when an operation fails midway,
  and whether retry or reconnect starts from a valid state.
- **Ownership and lifetimes.** Trace borrows, callback captures, and drop order
  through success and failure. Check that queued data outlives the producer's
  buffer and that the session releases its transport borrow before reuse.

## Update coverage and guidance

- Extend the closest test suite for changed behavior. For a bug fix, add a
  regression test and verify that it fails before the fix and passes after it
  when practical. Cover the normal path and relevant boundaries, such as full
  buffers, invalid inputs, timeout, cancellation, or partial I/O. Use explicit
  readiness and isolated resources for network or concurrency tests.
- Update affected Rustdoc and examples with public API or behavior changes.
  Check the example through its normal entry point with the relevant features.
  Update repository guides when setup or commands change, and update any skill
  whose workflow, links, or verification steps the change affects. Link from
  skills to the API contract in Rustdoc.

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
