use wasm_bindgen::prelude::*;

/// High-level pipeline helpers that compose decoder + encoder logic.
/// Currently delegates to the encoder's `trim_and_export` implementation
/// (MediaRecorder / captureStream fallback). This module exists so the
/// frontend can call a single entrypoint for trim+export.
#[wasm_bindgen(js_name = "pipeline_trim_and_export")]
pub async fn pipeline_trim_and_export(
    file: web_sys::File,
    trim_start_ms: f64,
    trim_end_ms: f64,
    output_filename: &str,
    progress_callback: &JsValue,
) -> Result<(), JsValue> {
    crate::encoder::trim_and_export(&file, trim_start_ms, trim_end_ms, output_filename, progress_callback).await
}
