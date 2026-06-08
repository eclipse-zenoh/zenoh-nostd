# Tasks: Add Fuzz Targets

## 1. decode_frame

- [x] 1.1 Create `fuzz_targets/decode_frame.rs`
- [x] 1.2 Implement TransportRx setup with fuzz data
- [x] 1.3 Call decode_prefixed and decode_raw in fuzz loop
- [x] 1.4 Register in `fuzz/Cargo.toml`

## 2. decode_message

- [x] 2.1 Create `fuzz_targets/decode_message.rs`
- [x] 2.2 Call `Message::z_decode()` on fuzz data
- [x] 2.3 Register in `fuzz/Cargo.toml`

## 3. Maintenance

- [x] 3.1 Verify `cargo fuzz build` succeeds
- [x] 3.2 Update `docs/fuzzing.md`
- [x] 3.3 Verify all 6 targets listed in verification_plan.md