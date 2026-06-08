# Design: Additional Fuzz Targets

## decode_frame

Fuzzes the transport frame decoder. Inputs arbitrary byte sequences through `TransportRx::decode_prefixed()` and `TransportRx::decode_raw()`.

```rust
fuzz_target!(|data: &[u8]| {
    let transport = TransportRx::new([0u8; 65536], 65536, 0, ...);
    // Fuzz via decode_prefixed
    let mut reader = &data[..];
    let _ = transport.decode_prefixed(&mut reader, |_msg| Ok(()));
    // Fuzz via decode_raw
    let _ = transport.decode_raw(data, |_msg| Ok(()));
});
```

## decode_message

Fuzzes message-level decoding after transport framing is removed. Inputs arbitrary byte sequences through `Message::z_decode()`.

```rust
fuzz_target!(|data: &[u8]| {
    let _ = Message::z_decode(&mut &data[..]);
});
```

## Coverage

| Target | Layer | Existing | New |
|--------|-------|----------|-----|
| transport_decode | Transport batch | ✅ | — |
| decode_frame | Transport frame | ❌ | ✅ |
| decode_message | Protocol message | ❌ | ✅ |
| establishment | Server handshake | ✅ | — |
| establishment_client | Client handshake | ✅ | — |
| keyexpr | Key expression | ✅ | — |