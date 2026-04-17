use wasm_bindgen::prelude::*;
pub mod decoder;
pub mod encoder;
pub mod pipeline;
pub mod types;
pub mod utils;
pub mod webcodecs;
pub mod webcodecs_pipeline;

#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
    utils::log("FlashCut WASM Core initialisiert ✓");
}

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
