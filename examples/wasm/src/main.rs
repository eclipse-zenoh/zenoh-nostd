#![no_main]
include!("headless.rs");

#[wasm_bindgen::prelude::wasm_bindgen]
pub fn cli_enabled() -> bool {
    false
}
