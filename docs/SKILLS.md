# zenoh-nostd agent playbook

Use this shared guidance when changing this repository. Paths and commands are
relative to the repository root. Read only the task-relevant guide below.

## Find the right layer

| Task | Start with |
| --- | --- |
| Protocol encoding or state machines | `crates/zenoh-proto`, `crates/zenoh-sansio`; [architecture](architecture.md) |
| Session, callbacks, storage, or broker API | `crates/zenoh-nostd`; [application contract](architecture.md#application-contract) |
| Network adapter or new MCU | `platforms/`, target board project; [porting steps](architecture.md#add-an-mcu-target) |
| CLI command or startup scenario | `examples/common/src/cli`, `examples/common/src/scenarios`; [applications](applications.md) |
| Console or board startup | `examples/embassy` and board README; [MCU consoles](../examples/embassy/README.md) |
| Build, test, or documentation | [development commands](README.md#development-commands), root/example justfiles, CI |

## Preserve these boundaries

- Keep core crates as libraries: no HAL, board selection, executor entry point,
  console, linker flags, or application credentials there. Preserve `no_std`
  and fixed-capacity operation; `alloc` remains optional.
- Keep adapters and examples in their independent Cargo workspaces. Their
  isolation prevents target-specific features and logging backends from being
  unified into the core workspace.
- Each target builds one `zenoh-app`, with `cli` enabled by default and optional
  headless startup. UART is the MCU console default; `console-usb` enables `cli`.
  Put reusable scenarios in `examples/common`, shared MCU helpers directly in
  `examples/embassy`, and HAL/pin/toolchain choices in board projects.
- Poll the session driver while operations run. Use scoped transport/session
  locals so the driver is dropped before its transport. Keep the transport borrow
  separate from the link-resource lifetime; do not reset occupied static cells
  to reconnect. Keep current `NoopRawMutex` access in one executor/thread
  context. Receive callbacks must not wait for console I/O.
- Cancellation and transport write errors end the whole connection. Drop the
  operation, driver, session and transport before awaiting diagnostics or cleanup.
  Only successful operations may clean up declarations and reuse a connection.
  Frontend buffers and callback/report queues remain bounded even with `alloc`.
  Changes to a scenario must work through both CLI and typed headless options.

## Build and verify

Use the repository's Nix environment: `nix develop -c just <recipe>`.
Run Cargo from the standalone project's directory when it needs local target
configuration; `--manifest-path` alone does not select its `.cargo/config.toml`.
ESP32-S3 requires `cargo +esp` and release firmware links. C6 and STM32 use stable
Rust. Keep application lockfiles reproducible with `--locked`.

Choose checks that exercise the changed behavior:

- Core changes: `just test`, relevant fixed/`alloc` tests, and
  `just check-portability` for portability/dependency changes.
- Scenario/CLI changes: common-library tests plus `just test-cli` and/or
  `just test-headless`; use `just check-headless` for feature-isolation changes.
- Board/adapter changes: relevant adapter checks and release firmware links
  for affected storage/console modes. See [build recipes](README.md#development-commands).
- Before finishing code changes, run `just check` in Nix when available. It
  includes application checks but not every release link or native integration
  suite. Use `just fmt` to cover all independent Rust projects.
- Documentation-only changes: check local links, anchors, paths, and command
  names against manifests/justfiles; no firmware rebuild is needed.

Firmware verification recipes (`build-firmware*`) use placeholders and never
flash. Firmware for a hardware test must be rebuilt with actual local settings
using the board's build/flash recipes. Keep Wi-Fi credentials out of committed
files and command output. Release the serial terminal before flashing.

Report checks actually run, failures or skipped checks, and physical-board
coverage separately. Keep docs focused on current behavior; put dated test
results in the change description rather than adding permanent validation logs.
