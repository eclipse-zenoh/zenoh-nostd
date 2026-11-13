# Working on zenoh-nostd

Start here to build the project, understand its boundaries, or run an application.

| Guide | Read it when you want to… |
| --- | --- |
| [Architecture](architecture.md) | Change a library, integrate it into an application, or add an MCU target |
| [Applications](applications.md) | Run `zenoh-app`, select CLI or headless mode, or test hardware |
| [Agent playbook](SKILLS.md) | Work with Codex, Claude Code, or another coding agent |

## Set up

From the repository root, with Nix flakes enabled:

```sh
nix develop
just --list
just test
```

The [flake](../flake.nix) supplies Rust, cross-compilation targets, the ESP Xtensa
toolchain, `just`, flash tools, `wasm-bindgen`, Python, and `zenohd`. The Xtensa
toolchain is unavailable in this flake on Intel macOS. The library MSRV is
declared in the root [Cargo.toml](../Cargo.toml).

For a first network run, follow the [native quick start](applications.md#native).
No board is needed for library tests or native integration tests.

## Development commands

Run these inside `nix develop`, or use `nix develop -c just <recipe>`.

| Command | Purpose |
| --- | --- |
| `just test` | Core protocol, Sans-I/O, and session tests; accepts a test filter |
| `just check` | Library tests, Clippy, adapters, and CLI/headless application checks |
| `just check-libraries` / `just check-examples` | Check one side of the repository |
| `just check-portability` | Build core libraries independently for ARM and RISC-V |
| `just check-headless` | Check all headless targets and absence of CLI/USB frontend dependencies |
| `just test-cli` / `just test-headless` | Native integration suites, each starting and stopping its own isolated router |
| `just fmt` | Format core libraries, adapters, and examples |
| `just bench` | Run library benchmarks |

The root [justfile](../justfile) imports [examples/justfile](../examples/justfile).
All example recipes work from the root; running `just --list` in `examples/`
shows only example recipes. Plain root `cargo test --workspace` or
`cargo fmt --all` covers only the core workspace, since adapters and examples
have independent workspaces.

`just check` does not flash hardware or link every firmware variant. To check
release linking without a board or real network settings:

```sh
just build-firmware-esp32s3
just build-firmware esp32c6-pico-wifi console-usb
just build-firmware nucleo-h753zi-ethernet alloc
just build-firmware esp32c6-pico-wifi "" headless
```

These verification recipes use placeholder network settings. For firmware you
intend to run, use the [application build recipes](applications.md#microcontrollers).
The [CI workflow](../.github/workflows/ci.yml) defines the full build matrix.
Report build checks separately from tests performed on a physical board.

## Shared agent instructions

The root [AGENTS.md](../AGENTS.md) directs Codex to read [SKILLS.md](SKILLS.md).
[CLAUDE.md](../CLAUDE.md) imports that same file for Claude Code. Keep repository
instructions in `docs/SKILLS.md`; the two entry points contain no separate rules.
This uses [Codex repository instructions](https://learn.chatgpt.com/docs/agent-configuration/agents-md)
and [Claude Code imports](https://code.claude.com/docs/en/memory#import-additional-files).

`SKILLS.md` is a shared repository playbook, not a tool-specific `SKILL.md`
package. It requires no personal configuration or duplicated skill directories.
Maintain current instructions here; past designs and validation logs belong in
Git history.
