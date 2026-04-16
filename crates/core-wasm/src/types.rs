use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrimRange {
    pub start_ms: f64,
    pub end_ms: f64,
}

#[wasm_bindgen]
impl TrimRange {
    #[wasm_bindgen(constructor)]
    pub fn new(start_ms: f64, end_ms: f64) -> TrimRange {
        TrimRange { start_ms, end_ms }
    }
    pub fn duration_ms(&self) -> f64 {
        self.end_ms - self.start_ms
    }
    pub fn is_valid(&self) -> bool {
        self.start_ms >= 0.0 && self.end_ms > self.start_ms
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VideoMetadata {
    pub duration_ms: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub codec: String,
}

impl VideoMetadata {
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(self).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WasmError {
    #[error("File konnte nicht gelesen werden: {0}")]
    FileReadError(String),
    #[error("Codec nicht unterstützt: {0}")]
    UnsupportedCodec(String),
    #[error("VideoDecoder Fehler: {0}")]
    DecoderError(String),
    #[error("VideoEncoder Fehler: {0}")]
    EncoderError(String),
    #[error("Ungültiger TrimRange: start={0}ms, end={1}ms")]
    InvalidTrimRange(f64, f64),
}

impl From<WasmError> for JsValue {
    fn from(e: WasmError) -> JsValue {
        JsValue::from_str(&e.to_string())
    }
}
