# Change: Add decode_frame and decode_message fuzz targets

## Why

Currently have 4 fuzz targets (transport_decode, establishment, establishment_client, keyexpr). Two planned targets are missing: decode_frame (frame-level decoding) and decode_message (message-level decoding after transport framing). These provide coverage at different protocol layers than the existing transport-level decode fuzz target.

## What Changes

- Add `fuzz_targets/decode_frame.rs` — frame-level decoder fuzz
- Add `fuzz_targets/decode_message.rs` — message-level decoder fuzz
- Register both in `fuzz/Cargo.toml`
- Add fuzz corpus directories for new targets
- Update `docs/fuzzing.md` with new target docs
- Spec delta: fuzzing (ADDED: decode_frame, decode_message requirements)

## Impact

- Affected specs: fuzzing
- Affected code: fuzz targets only (no production code changes)
- Risk: Minimal — fuzz targets are additive test infrastructure