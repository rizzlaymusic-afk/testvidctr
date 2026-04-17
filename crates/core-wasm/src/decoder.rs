use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use js_sys::{Array, Function, Object, Promise, Uint8Array};
use crate::webcodecs::{VideoDecoder, EncodedVideoChunk};
use serde_json::json;

#[wasm_bindgen]
pub fn create_video_decoder(on_frame: &JsValue, on_error: &JsValue) -> Result<JsValue, JsValue> {
    let init = Object::new();
    js_sys::Reflect::set(&init, &JsValue::from_str("output"), on_frame)?;
    js_sys::Reflect::set(&init, &JsValue::from_str("error"), on_error)?;

    let global = js_sys::global();
    let ctor_val = js_sys::Reflect::get(&global, &JsValue::from_str("VideoDecoder"))?;
    if ctor_val.is_undefined() || ctor_val.is_null() {
        web_sys::console::warn_1(&JsValue::from_str("VideoDecoder not supported in this environment"));
        return Ok(JsValue::NULL);
    }

    if ctor_val.is_function() {
        let dec = VideoDecoder::new(&JsValue::from(init));
        return Ok(JsValue::from(dec));
    }

    let ctor_fn: Function = ctor_val.dyn_into()?;
    let args = Array::new();
    args.push(&init);
    let decoder = js_sys::Reflect::construct(&ctor_fn, &args)?;
    Ok(decoder)
}

#[wasm_bindgen]
pub fn configure_decoder(decoder: &JsValue, codec: &str, width: u32, height: u32) -> Result<(), JsValue> {
    if decoder.is_null() || decoder.is_undefined() {
        return Ok(());
    }
    let config = Object::new();
    js_sys::Reflect::set(&config, &JsValue::from_str("codec"), &JsValue::from_str(codec))?;
    js_sys::Reflect::set(&config, &JsValue::from_str("codedWidth"), &JsValue::from_f64(width as f64))?;
    js_sys::Reflect::set(&config, &JsValue::from_str("codedHeight"), &JsValue::from_f64(height as f64))?;
    js_sys::Reflect::set(&config, &JsValue::from_str("hardwareAcceleration"), &JsValue::from_str("prefer-hardware"))?;
    if let Ok(typed_dec) = decoder.clone().dyn_into::<VideoDecoder>() {
        VideoDecoder::configure(&typed_dec, &config);
        return Ok(());
    }

    let configure_fn = js_sys::Reflect::get(decoder, &JsValue::from_str("configure"))?.dyn_into::<Function>()?;
    configure_fn.call1(decoder, &config)?;
    Ok(())
}

#[wasm_bindgen]
pub fn decode_chunk(decoder: &JsValue, data: &Uint8Array, timestamp_us: f64, is_keyframe: bool) -> Result<(), JsValue> {
    if decoder.is_null() || decoder.is_undefined() {
        return Ok(());
    }
    let global = js_sys::global();
    let init = Object::new();
    js_sys::Reflect::set(&init, &JsValue::from_str("timestamp"), &JsValue::from_f64(timestamp_us))?;
    js_sys::Reflect::set(&init, &JsValue::from_str("type"), &JsValue::from_str(if is_keyframe { "key" } else { "delta" }))?;
    js_sys::Reflect::set(&init, &JsValue::from_str("data"), &JsValue::from(data))?;

    // If we have a typed VideoDecoder available, construct a typed EncodedVideoChunk and call decode.
    if let Ok(typed_dec) = decoder.clone().dyn_into::<VideoDecoder>() {
        let chunk = EncodedVideoChunk::new(&JsValue::from(init));
        VideoDecoder::decode(&typed_dec, &JsValue::from(chunk));
        return Ok(());
    }

    // Fallback: dynamic EncodedVideoChunk constructor if present
    let chunk_ctor_val = js_sys::Reflect::get(&global, &JsValue::from_str("EncodedVideoChunk"))?;
    let chunk = if chunk_ctor_val.is_undefined() || chunk_ctor_val.is_null() {
        JsValue::from(init)
    } else {
        let chunk_ctor_fn: Function = chunk_ctor_val.dyn_into()?;
        let args = Array::new();
        args.push(&init);
        js_sys::Reflect::construct(&chunk_ctor_fn, &args)?
    };
    let decode_fn = js_sys::Reflect::get(decoder, &JsValue::from_str("decode"))?.dyn_into::<Function>()?;
    decode_fn.call1(decoder, &chunk)?;
    Ok(())
}

#[wasm_bindgen]
pub fn draw_frame_to_canvas(frame: JsValue, canvas: JsValue) -> Result<(), JsValue> {
    if frame.is_null() || canvas.is_null() {
        return Ok(());
    }
    let get_ctx_fn = js_sys::Reflect::get(&canvas, &JsValue::from_str("getContext"))?.dyn_into::<Function>()?;
    let ctx = get_ctx_fn.call1(&canvas, &JsValue::from_str("2d"))?;
    if ctx.is_undefined() { return Err(JsValue::from_str("Canvas 2D Context not available")); }
    let draw_with_video = js_sys::Reflect::get(&ctx, &JsValue::from_str("drawImageWithVideoFrame"));
    match draw_with_video {
        Ok(f) => {
            if f.is_function() {
                f.dyn_into::<Function>()?.call2(&ctx, &frame, &JsValue::from_f64(0.0))?;
            } else {
                js_sys::Reflect::get(&ctx, &JsValue::from_str("drawImage"))?.dyn_into::<Function>()?.call2(&ctx, &frame, &JsValue::from_f64(0.0))?;
            }
        }
        Err(_) => {
            js_sys::Reflect::get(&ctx, &JsValue::from_str("drawImage"))?.dyn_into::<Function>()?.call2(&ctx, &frame, &JsValue::from_f64(0.0))?;
        }
    }
    if let Ok(close_fn) = js_sys::Reflect::get(&frame, &JsValue::from_str("close")) {
        if close_fn.is_function() { close_fn.dyn_into::<Function>()?.call0(&frame)?; }
    }
    Ok(())
}

#[wasm_bindgen]
pub async fn flush_decoder(decoder: &JsValue) -> Result<(), JsValue> {
    if decoder.is_null() || decoder.is_undefined() { return Ok(()); }
    if let Ok(typed_dec) = decoder.clone().dyn_into::<VideoDecoder>() {
        let promise: Promise = VideoDecoder::flush(&typed_dec);
        JsFuture::from(promise).await?;
        return Ok(());
    }

    let flush_fn = js_sys::Reflect::get(decoder, &JsValue::from_str("flush"))?.dyn_into::<Function>()?;
    let p = flush_fn.call0(decoder)?;
    let promise: Promise = p.dyn_into()?;
    JsFuture::from(promise).await?;
    Ok(())
}

#[wasm_bindgen]
pub async fn read_video_metadata(file: web_sys::File) -> Result<JsValue, JsValue> {
    let url = web_sys::Url::create_object_url_with_blob(&file)?;
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window object"))?;
    let document = window.document().ok_or_else(|| JsValue::from_str("No document"))?;
    let video = document.create_element("video")?.dyn_into::<web_sys::HtmlVideoElement>()?;
    video.set_src(&url);
    video.set_preload("metadata");

    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        // Use one clone for the callback and another for registering the handler
        let v_for_cb = video.clone();
        let v_for_set = v_for_cb.clone();
        let cb = Closure::once_into_js(move || {
            let meta = json!({
                "duration_ms": v_for_cb.duration() * 1000.0,
                "width": v_for_cb.video_width(),
                "height": v_for_cb.video_height(),
                "fps": 30.0,
                "mime_type": "",
                "file_name": "",
                "file_size": 0u64,
            });
            let json = serde_json::to_string(&meta).unwrap_or_default();
            resolve.call1(&JsValue::NULL, &JsValue::from_str(&json)).ok();
        });
        v_for_set.set_onloadedmetadata(Some(cb.as_ref().unchecked_ref()));
    });

    let result = JsFuture::from(promise).await?;
    web_sys::Url::revoke_object_url(&url)?;
    Ok(result)
}
