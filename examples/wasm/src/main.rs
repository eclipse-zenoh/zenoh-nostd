#![no_main]
// One target application, with its frontend selected at compile time.
#[cfg(feature = "cli")]
include!("console.rs");
#[cfg(not(feature = "cli"))]
include!("headless.rs");

#[wasm_bindgen::prelude::wasm_bindgen]
pub fn cli_enabled() -> bool {
    cfg!(feature = "cli")
}
