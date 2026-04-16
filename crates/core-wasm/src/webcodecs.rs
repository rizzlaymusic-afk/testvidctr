use js_sys::Promise;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = VideoEncoder)]
    pub type VideoEncoder;

    #[wasm_bindgen(constructor, js_class = "VideoEncoder")]
    pub fn new(init: &JsValue) -> VideoEncoder;

    #[wasm_bindgen(method, js_name = configure)]
    pub fn configure(this: &VideoEncoder, config: &JsValue);

    #[wasm_bindgen(method, js_name = encode)]
    pub fn encode_with_options(this: &VideoEncoder, frame: &JsValue, options: &JsValue) -> JsValue;

    #[wasm_bindgen(method)]
    pub fn flush(this: &VideoEncoder) -> Promise;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = VideoDecoder)]
    pub type VideoDecoder;

    #[wasm_bindgen(constructor, js_class = "VideoDecoder")]
    pub fn new(init: &JsValue) -> VideoDecoder;

    #[wasm_bindgen(method)]
    pub fn configure(this: &VideoDecoder, config: &JsValue);

    #[wasm_bindgen(method, js_name = decode)]
    pub fn decode(this: &VideoDecoder, chunk: &JsValue);

    #[wasm_bindgen(method)]
    pub fn flush(this: &VideoDecoder) -> Promise;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = EncodedVideoChunk)]
    pub type EncodedVideoChunk;

    #[wasm_bindgen(constructor, js_class = "EncodedVideoChunk")]
    pub fn new(init: &JsValue) -> EncodedVideoChunk;
}
