<p align="center">
  <img src="https://zenoh.io/img/zenoh-dragon-small.png" height="121">
</p>

<h1 align="center">zenoh-nostd</h1>
<p align="center">
  <strong>Zero Network Overhead. Async. No std. No alloc. Pure Rust.</strong>
</p>
<p align="center">
  <code>async</code> • <code>bare-metal</code> • <code>no_std</code> • <code>zenoh</code>
</p>
<p align="center">
  <a href="https://github.com/eclipse-zenoh/zenoh-nostd/actions/workflows/ci.yml"><img src="https://github.com/eclipse-zenoh/zenoh-nostd/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <img src="https://img.shields.io/badge/rustc-1.91.0%2B-orange.svg" alt="MSRV 1.91.0" />
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-EPL--2.0%20OR%20Apache--2.0-blue.svg" alt="License: EPL-2.0 OR Apache-2.0" /></a>
</p>

## Overview

`zenoh-nostd` is an async Rust implementation of the Zenoh protocol for
`no_std` systems. It uses fixed-capacity storage by default; optional `alloc`
adds heap-backed storage and broker support. Applications provide their
transport, runtime/time integration, entropy, and memory configuration.

The core libraries are independent of MCU hardware. Transport adapters and
standalone applications support native hosts, browsers, and Embassy-based
ESP32-S3, ESP32-C6-Pico, and NUCLEO-H753ZI targets.
The NUCLEO-H753ZI firmware has not been tested on a physical board.

This project is in early development. The minimum supported Rust version is
1.91.0, declared in [Cargo.toml](Cargo.toml).

## Start working

From the repository root, with Nix flakes enabled:

```sh
nix develop
just test
just check
```

The Nix environment provides the Rust toolchains, cross-compilation targets,
flash tools, and a Zenoh router. All Just commands are available from the root;
`just --list` shows them. The [contributor guide](docs/README.md) explains which
checks to run and how the independent Cargo projects fit together.

## Documentation

- [Getting started and development commands](docs/README.md)
- [Architecture, integration contract, and porting](docs/architecture.md)
- [Running applications: CLI, headless, MCU, and browser](docs/applications.md)
- [Example projects and board guides](examples/README.md)
- [Shared agent playbook](docs/SKILLS.md), loaded through `AGENTS.md` for Codex
  and `CLAUDE.md` for Claude Code

## Use the library

```toml
[dependencies]
zenoh-nostd = { git = "https://github.com/eclipse-zenoh/zenoh-nostd", default-features = false }
```

See the [application contract](docs/architecture.md#application-contract) and
[example projects](examples/README.md) for complete runtime and transport setup.
Each example builds one `zenoh-app`; `cli` is enabled by default and can be
removed for an unattended startup scenario. CLI, HAL, and board configuration
remain outside the core libraries.

## Limitations

- Serial network transport is not implemented. UART/USB example consoles are
  command interfaces. ([#11](https://github.com/eclipse-zenoh/zenoh-nostd/issues/11))
- The Interest protocol is not implemented.
  ([#46](https://github.com/eclipse-zenoh/zenoh-nostd/issues/46))
