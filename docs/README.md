# Working on zenoh-nostd

Start here to build the project, understand its boundaries, or run an application.

| Guide | Read it when you want to… |
| --- | --- |
| [Architecture](architecture.md) | Change a library, integrate it into an application, or add an MCU target |
| [Applications](applications.md) | Run `zenoh-app`, select CLI or headless mode, or test hardware |
| [Development skill](../skills/zenoh-nostd-develop/SKILL.md) | Have an agent change or review this repository |
| [Integration skill](../skills/zenoh-nostd-integrate/SKILL.md) | Have an agent use the library in your own application |

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
| `just check-docs` | Strict public Rustdoc and doctests for core and adapter feature configurations |
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

## Public API reference

Run `just check-docs` to build the reference and validate its examples. Start at
`target/doc/zenoh_nostd/index.html` for the task-to-API index. Cargo prints each
generated reference path: independent adapters normally use their own `target/`,
while `CARGO_TARGET_DIR` can consolidate output, as in the Nix environment.
Use the rendered public re-exports rather than `--document-private-items`.
`cargo doc --open -p zenoh-nostd --no-deps` opens the core reference; enable
`--features alloc` for allocation-specific APIs. Missing public documentation and
broken Rustdoc links fail the strict check and CI. Logging backends are checked
separately because enabling them together defines conflicting macros.

The Rustdoc attached to each API is the source of truth for defaults, errors,
ownership, memory limits and async cleanup. Read it for the exact revision and
features being used. Runnable examples check local data/configuration behavior;
`no_run` networking examples compile but need a running driver and peer to execute.
Both agent skills route to these contracts before generating or changing code.

Run `python3 .github/scripts/check_md.py` from the repository root to check
local Markdown links, skill frontmatter, and discovery symlinks.

## Agent skills

Two tool-neutral skills separate repository development from application integration:

- [zenoh-nostd-develop](../skills/zenoh-nostd-develop/SKILL.md) covers code ownership,
  workspace boundaries, Nix/Just recipes, and checks for contributions.
- [zenoh-nostd-integrate](../skills/zenoh-nostd-integrate/SKILL.md) covers adapters,
  features, runtime/memory requirements, and session lifecycles in a consumer's
  project. It does not require this repository's build environment or example CLI.
  Includes recipes for [native](../skills/zenoh-nostd-integrate/references/native.md),
  [Embassy](../skills/zenoh-nostd-integrate/references/embassy.md),
  [WASM](../skills/zenoh-nostd-integrate/references/wasm.md), and an
  [existing embedded console](../skills/zenoh-nostd-integrate/references/embedded-cli.md).

The canonical files live in `skills/`. `.agents/skills/` and `.claude/skills/`
contain symlinks to those same folders for local Codex and Claude Code discovery.
The root [AGENTS.md](../AGENTS.md) selects the appropriate skill; [CLAUDE.md](../CLAUDE.md)
imports that entry point. Maintain each skill once, without tool-specific copies.
See [Codex skills](https://learn.chatgpt.com/docs/build-skills) and
[Claude Code skills](https://code.claude.com/docs/en/skills#choose-where-skills-load).

Inside this repository, invoke `$zenoh-nostd-develop` in Codex or
`/zenoh-nostd-develop` in Claude Code, or let the matching task select it.

### Use the integration skill in your project

Install the complete integration folder from a committed Zenoh revision that
matches your dependency. From your application's root, set `upstream` to the
checkout at that revision. These commands archive the committed files and store
the full commit ID in `SOURCE_REVISION` beside the installed `SKILL.md`.

```sh
upstream=/path/to/zenoh-nostd
destination=.agents/skills/zenoh-nostd-integrate
revision=$(git -C "$upstream" rev-parse --verify HEAD) &&
  mkdir -p "$(dirname "$destination")" &&
  mkdir "$destination" &&
  git -C "$upstream" archive "$revision:skills/zenoh-nostd-integrate" |
    tar -xf - -C "$destination" &&
  printf '%s\n' "$revision" > "$destination/SOURCE_REVISION"
```

To expose that same copy to Claude Code:

```sh
mkdir -p .claude/skills &&
  test ! -e .claude/skills/zenoh-nostd-integrate &&
  test ! -L .claude/skills/zenoh-nostd-integrate &&
  ln -s ../../.agents/skills/zenoh-nostd-integrate .claude/skills/zenoh-nostd-integrate
```

For Claude Code alone, set `destination=.claude/skills/zenoh-nostd-integrate`
in the first block. The install command expects a new destination.
For an upgrade, stage a fresh copy, review it against the installed copy, and
carry forward local customizations before replacing the folder and
`SOURCE_REVISION`. The installed skill and recipes work without the checkout.
Invoke `$zenoh-nostd-integrate` in Codex or `/zenoh-nostd-integrate` in Claude Code.
For example: “Add a fixed-capacity TCP session to this Embassy application.”

Keep the skill, documentation and examples aligned with the library revision
your application uses. Preserve your application's own agent instructions.
