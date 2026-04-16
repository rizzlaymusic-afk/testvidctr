use leptos::*;
use wasm_bindgen::prelude::*;
mod components;
mod state;
use components::app::App;

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    leptos::mount_to_body(App);
    // Dev startup marker to confirm wasm initialized in browser console
    web_sys::console::log_1(&"flashcut wasm loaded".into());
}
