use wasm_bindgen::prelude::*;
use web_sys::console;

pub fn log(msg: &str) {
    console::log_1(&JsValue::from_str(msg));
}
pub fn warn(msg: &str) {
    console::warn_1(&JsValue::from_str(msg));
}
pub fn error(msg: &str) {
    console::error_1(&JsValue::from_str(msg));
}

pub fn ms_to_timecode(ms: f64) -> String {
    let total_secs = (ms / 1000.0) as u64;
    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    let millis = (ms % 1000.0) as u64;
    format!("{:02}:{:02}.{:03}", minutes, seconds, millis)
}
