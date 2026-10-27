# Core crates and platform adapters. Example recipes are imported below so
# their existing command names remain available from the repository root.

check: check-libraries check-examples

check-libraries: check-portability
    cargo clippy -p zenoh-nostd
    cargo clippy -p zenoh-nostd --features=log
    cargo clippy -p zenoh-nostd --features=defmt
    cargo clippy -p zenoh-nostd --features=web_console
    cargo clippy -p zenoh-nostd --features=alloc
    cd platforms/zenoh-std && just check
    cd platforms/zenoh-wasm && just check
    cd platforms/zenoh-embassy && just check
    cargo test -p zenoh-nostd && cargo test -p zenoh-nostd --features=alloc

# Link the core libraries for both architectures without board/example feature unification.
check-portability:
    set -eu; for target in thumbv7em-none-eabihf riscv32imac-unknown-none-elf; do for features in embedded embedded,defmt embedded,log embedded,alloc embedded,alloc,defmt; do cargo build --locked -p zenoh-proto -p zenoh-sansio -p zenoh-nostd --target "$target" --no-default-features --features "$features"; done; done

fmt: fmt-libraries fmt-examples

fmt-libraries:
    cargo fmt --all
    cd platforms/zenoh-std && cargo fmt
    cd platforms/zenoh-wasm && cargo fmt
    cd platforms/zenoh-embassy && cargo fmt

fix: fix-libraries fix-examples

fix-libraries:
    cargo clippy -p zenoh-nostd --fix --lib --allow-dirty --allow-staged
    cd platforms/zenoh-std && just fix
    cd platforms/zenoh-wasm && just fix
    cd platforms/zenoh-embassy && just fix

loc-proto:
    tokei --files crates/zenoh-proto crates/zenoh-derive/src/codec* --exclude crates/zenoh-proto/src/tests*

test filter="":
    cargo test {{ filter }} -p zenoh-proto -p zenoh-sansio -p zenoh-nostd

bench filter="bench":
    cargo test -p zenoh-proto {{ filter }} --profile=release -- --nocapture --ignored --test-threads=1

flood *args:
    cargo run -p zenoh-sansio --release --no-default-features --features=std,log --example z_flood -- {{ args }}

drain *args:
    cargo run -p zenoh-sansio --release --no-default-features --features=std,log --example z_drain -- {{ args }}

import 'examples/justfile'
