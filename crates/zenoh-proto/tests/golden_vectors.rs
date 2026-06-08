#![allow(dead_code)]

use serde_json::Value;
use std::fs;
use std::path::Path;

use zenoh_proto::*;

#[derive(serde::Deserialize)]
struct GoldenVector {
    name: String,
    feature: String,
    input_bytes: Vec<u8>,
    expected: Option<Value>,
    error: Option<ErrorInfo>,
    source: String,
    upstream_version: String,
    test_class: String,
}

#[derive(serde::Deserialize)]
struct ErrorInfo {
    r#type: String,
    description: String,
}

#[test]
fn golden_vectors_vle_roundtrip() {
    let vectors_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/golden_vectors");

    let mut tested = 0;

    for entry in fs::read_dir(&vectors_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        let content = fs::read_to_string(&path).unwrap();
        let vector: GoldenVector = serde_json::from_str(&content).unwrap();

        if vector.feature != "variable-length-encoding" {
            continue;
        }

        tested += 1;

        let mut reader = &vector.input_bytes[..];

        match vector.test_class.as_str() {
            "roundtrip" => {
                let result = u64::z_decode(&mut reader);
                if let Some(expected) = &vector.expected {
                    let decoded = result.unwrap_or_else(|e| {
                        panic!("VLE vector '{}' expected Ok but got {:?}", vector.name, e)
                    });
                    let expected_val = expected
                        .get("value")
                        .and_then(|v| v.as_u64())
                        .unwrap_or_else(|| {
                            panic!(
                                "VLE vector '{}' missing or invalid 'value' field in expected",
                                vector.name
                            )
                        });
                    assert_eq!(
                        decoded, expected_val,
                        "VLE vector '{}' value mismatch",
                        vector.name
                    );
                }
            }
            "invalid" => {
                let result = u64::z_decode(&mut reader);
                assert!(
                    result.is_err(),
                    "VLE vector '{}' expected Err but got Ok({:?})",
                    vector.name,
                    result.unwrap()
                );
            }
            other => panic!("Unknown test_class: {}", other),
        }
    }

    assert!(
        tested > 0,
        "No VLE golden vectors found in {:?}",
        vectors_dir
    );
    eprintln!("Verified {} VLE golden vectors", tested);
}

#[test]
fn golden_vectors_keyexpr_roundtrip() {
    let vectors_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/golden_vectors");

    let mut tested = 0;

    for entry in fs::read_dir(&vectors_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }

        let content = fs::read_to_string(&path).unwrap();
        let vector: GoldenVector = serde_json::from_str(&content).unwrap();

        if vector.feature != "key-expression" {
            continue;
        }

        tested += 1;

        match vector.test_class.as_str() {
            "roundtrip" => {
                let s = String::from_utf8(vector.input_bytes.clone()).unwrap();
                let result = keyexpr::new(&s);
                let ke = result.unwrap_or_else(|e| {
                    panic!(
                        "Keyexpr vector '{}' expected Ok but got {:?}",
                        vector.name, e
                    )
                });
                if let Some(expected) = &vector.expected {
                    let expected_val = expected
                        .get("value")
                        .and_then(|v| v.as_str())
                        .unwrap_or_else(|| {
                            panic!(
                                "Keyexpr vector '{}' missing or invalid 'value' field in expected",
                                vector.name
                            )
                        });
                    assert_eq!(
                        ke.as_str(),
                        expected_val,
                        "Keyexpr vector '{}' value mismatch",
                        vector.name
                    );
                }
            }
            other => panic!("Unknown test_class: {}", other),
        }
    }

    assert!(
        tested > 0,
        "No keyexpr golden vectors found in {:?}",
        vectors_dir
    );
    eprintln!("Verified {} keyexpr golden vectors", tested);
}
