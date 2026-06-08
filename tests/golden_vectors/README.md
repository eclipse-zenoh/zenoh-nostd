# Golden Vector Test Data

This directory contains golden vectors for protocol compatibility testing.

## Structure

Each vector is a JSON file with the following structure:

```json
{
    "name": "vle_encode_u32_max",
    "feature": "variable-length-encoding",
    "input_bytes": [255, 255, 255, 255, 15],
    "expected": {
        "type": "u32",
        "value": 4294967295
    },
    "error": null,
    "source": "zenoh-nostd v0.1.0",
    "upstream_version": "eclipse-zenoh main 2026-06"
}
```

For error cases:

```json
{
    "name": "vle_truncated_input",
    "feature": "variable-length-encoding",
    "input_bytes": [255],
    "expected": null,
    "error": {
        "type": "CodecError::CouldNotParseField",
        "description": "Truncated VLE input"
    },
    "source": "zenoh-nostd v0.1.0",
    "upstream_version": "eclipse-zenoh main 2026-06"
}
```

## Vector Naming

Vectors follow the pattern: `<feature>_<scenario>.json`

Examples:
- `vle_encode_max.json`
- `vle_decode_min.json`  
- `keyexpr_valid_simple.json`
- `keyexpr_invalid_empty_chunk.json`
- `transport_batch_single_message.json`

## Test Classes

Each vector belongs to one of these test classes:

1. **encode-decode**: zenoh-nostd encodes → upstream decodes → match
2. **decode-encode**: upstream encodes → zenoh-nostd decodes → match
3. **roundtrip**: zenoh-nostd encode → zenoh-nostd decode → match
4. **invalid**: Known-invalid input → expected error

## Generating Vectors

Vectors should be generated from the reference upstream Zenoh Rust implementation.

```bash
# Plan: add a golden vector generator to tests/
# cargo run --example generate_golden_vectors
```

## Currently Empty

This directory is a placeholder. Vectors will be added as part of ongoing compatibility testing.