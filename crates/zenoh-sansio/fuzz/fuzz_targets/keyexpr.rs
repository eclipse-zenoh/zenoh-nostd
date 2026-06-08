#![no_main]

use libfuzzer_sys::fuzz_target;

// Use zenoh_proto re-exported through zenoh-sansio
fuzz_target!(|data: &[u8]| {
    // Fuzz keyexpr parsing with arbitrary byte sequences.
    // The parser accepts &str - convert arbitrary bytes to a string
    // by replacing invalid UTF-8 sequences with the replacement character.
    if let Ok(s) = core::str::from_utf8(data) {
        // Must never panic, only return Ok or Err
        let _ = zenoh_proto::keyexpr::new(s);
    }
});
