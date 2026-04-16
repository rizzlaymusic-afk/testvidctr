use crate::webcodecs::VideoEncoder;
use js_sys::{Array, Function, Object, Promise, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
pub fn create_video_encoder(on_chunk: &JsValue, on_error: &JsValue) -> Result<JsValue, JsValue> {
    // Build init object with the JS callbacks.
    let init = Object::new();
    js_sys::Reflect::set(&init, &JsValue::from_str("output"), on_chunk)?;
    js_sys::Reflect::set(&init, &JsValue::from_str("error"), on_error)?;

    // Check for global VideoEncoder; prefer typed wrapper when available.
    let global = js_sys::global();
    let ctor_val = js_sys::Reflect::get(&global, &JsValue::from_str("VideoEncoder"))?;
    if ctor_val.is_undefined() || ctor_val.is_null() {
        web_sys::console::warn_1(&JsValue::from_str(
            "VideoEncoder not supported in this environment",
        ));
        return Ok(JsValue::NULL);
    }

    // If VideoEncoder exists, use the typed `webcodecs` wrapper; otherwise fall back.
    if ctor_val.is_function() {
        let enc = VideoEncoder::new(&JsValue::from(init));
        return Ok(JsValue::from(enc));
    }

    // Dynamic fallback (should be rarely used if constructor exists as a function)
    let ctor_fn: Function = ctor_val.dyn_into()?;
    let args = Array::new();
    args.push(&init);
    let encoder = js_sys::Reflect::construct(&ctor_fn, &args)?;
    Ok(encoder)
}

#[wasm_bindgen]
pub fn configure_encoder(
    encoder: &JsValue,
    width: u32,
    height: u32,
    bitrate_kbps: u32,
) -> Result<(), JsValue> {
    if encoder.is_null() || encoder.is_undefined() {
        return Ok(());
    }
    let config = Object::new();
    js_sys::Reflect::set(
        &config,
        &JsValue::from_str("codec"),
        &JsValue::from_str("avc1.42001E"),
    )?;
    js_sys::Reflect::set(
        &config,
        &JsValue::from_str("width"),
        &JsValue::from_f64(width as f64),
    )?;
    js_sys::Reflect::set(
        &config,
        &JsValue::from_str("height"),
        &JsValue::from_f64(height as f64),
    )?;
    js_sys::Reflect::set(
        &config,
        &JsValue::from_str("bitrate"),
        &JsValue::from_f64(bitrate_kbps as f64 * 1000.0),
    )?;
    js_sys::Reflect::set(
        &config,
        &JsValue::from_str("framerate"),
        &JsValue::from_f64(30.0),
    )?;
    js_sys::Reflect::set(
        &config,
        &JsValue::from_str("latencyMode"),
        &JsValue::from_str("quality"),
    )?;
    // Prefer typed call when `encoder` is a `VideoEncoder` instance
    if let Ok(typed_enc) = encoder.clone().dyn_into::<VideoEncoder>() {
        VideoEncoder::configure(&typed_enc, &config);
        return Ok(());
    }

    // Fallback to dynamic method lookup
    let configure_fn =
        js_sys::Reflect::get(encoder, &JsValue::from_str("configure"))?.dyn_into::<Function>()?;
    configure_fn.call1(encoder, &config)?;
    Ok(())
}

#[wasm_bindgen]
pub fn encode_frame(
    encoder: &JsValue,
    frame: &JsValue,
    force_keyframe: bool,
) -> Result<(), JsValue> {
    if encoder.is_null() || encoder.is_undefined() {
        return Ok(());
    }
    let encode_fn =
        js_sys::Reflect::get(encoder, &JsValue::from_str("encode"))?.dyn_into::<Function>()?;
    // try typed encoder first
    let options = Object::new();
    js_sys::Reflect::set(
        &options,
        &JsValue::from_str("keyFrame"),
        &JsValue::from_bool(force_keyframe),
    )?;
    if let Ok(typed_enc) = encoder.clone().dyn_into::<VideoEncoder>() {
        // call typed encode; ignore returned value
        let _ = VideoEncoder::encode_with_options(&typed_enc, frame, &options);
        return Ok(());
    }
    // attempt to call with options; if it fails, call without
    let _ = encode_fn
        .call2(encoder, frame, &options)
        .or_else(|_| encode_fn.call1(encoder, frame));
    Ok(())
}

#[wasm_bindgen]
pub fn chunks_to_blob_and_download(chunks: Vec<Uint8Array>, filename: &str) -> Result<(), JsValue> {
    let arr = Array::new();
    for c in chunks.iter() {
        arr.push(&JsValue::from(c));
    }
    let blob_parts = web_sys::BlobPropertyBag::new();
    blob_parts.set_type("video/webm");
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&arr, &blob_parts)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;
    let window = web_sys::window().ok_or(JsValue::from_str("No window"))?;
    let document = window.document().ok_or(JsValue::from_str("No document"))?;
    let a = document
        .create_element("a")?
        .dyn_into::<web_sys::HtmlElement>()?;
    a.set_attribute("href", &url)?;
    a.set_attribute("download", filename)?;
    a.click();
    web_sys::Url::revoke_object_url(&url)?;
    Ok(())
}

#[wasm_bindgen]
pub async fn flush_encoder(encoder: &JsValue) -> Result<(), JsValue> {
    if encoder.is_null() || encoder.is_undefined() {
        return Ok(());
    }
    // Try typed flush first when possible
    if let Ok(typed_enc) = encoder.clone().dyn_into::<VideoEncoder>() {
        let promise: Promise = VideoEncoder::flush(&typed_enc);
        JsFuture::from(promise).await?;
        return Ok(());
    }

    let flush_fn =
        js_sys::Reflect::get(encoder, &JsValue::from_str("flush"))?.dyn_into::<Function>()?;
    let p = flush_fn.call0(encoder)?;
    let promise: Promise = p.dyn_into()?;
    JsFuture::from(promise).await?;
    Ok(())
}

#[wasm_bindgen]
pub async fn trim_and_export(
    file: &web_sys::File,
    trim_start_ms: f64,
    trim_end_ms: f64,
    output_filename: &str,
    progress_callback: &JsValue,
) -> Result<(), JsValue> {
    if trim_end_ms <= trim_start_ms {
        return Err(JsValue::from_str("Invalid trim range"));
    }
    let url = web_sys::Url::create_object_url_with_blob(file)?;
    let window = web_sys::window().ok_or(JsValue::from_str("No window"))?;
    let document = window.document().ok_or(JsValue::from_str("No document"))?;
    let video = document
        .create_element("video")?
        .dyn_into::<web_sys::HtmlVideoElement>()?;
    video.set_src(&url);
    video.set_preload("metadata");

    // wait for metadata
    let promise_meta = js_sys::Promise::new(&mut |resolve, reject| {
        let fr = video.clone();
        let resolve_cb = Closure::once_into_js(move || {
            resolve.call0(&JsValue::NULL).ok();
        });
        let reject_cb = Closure::once_into_js(move || {
            reject.call0(&JsValue::NULL).ok();
        });
        fr.set_onloadedmetadata(Some(resolve_cb.as_ref().unchecked_ref()));
        fr.set_onerror(Some(reject_cb.as_ref().unchecked_ref()));
    });
    JsFuture::from(promise_meta).await?;

    // debug: metadata loaded
    web_sys::console::log_1(&JsValue::from_str("encoder: metadata loaded"));

    let start = (trim_start_ms / 1000.0).max(0.0);
    let duration_ms = (trim_end_ms - trim_start_ms).max(0.0);
    video.set_current_time(start);
    // attempt to start playback to drive captureStream frames
    if let Ok(promise) = video.play() {
        let _ = JsFuture::from(promise).await;
        web_sys::console::log_1(&JsValue::from_str("encoder: attempted video.play()"));
    }
    // small settle
    let settle = js_sys::Promise::new(&mut |resolve, _| {
        let callback = Closure::wrap(Box::new(move || {
            resolve.call0(&JsValue::NULL).ok();
        }) as Box<dyn FnMut()>);
        let window = web_sys::window().expect("no window");
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            callback.as_ref().unchecked_ref(),
            200,
        );
        callback.forget();
    });
    JsFuture::from(settle).await?;

    // captureStream fallback via Reflect
    let stream =
        js_sys::Reflect::get(&video, &JsValue::from_str("captureStream")).and_then(|f| {
            if f.is_function() {
                let func: Function = f.dyn_into()?;
                func.call0(&video)
            } else {
                Ok(JsValue::NULL)
            }
        })?;
    if stream.is_null() || stream.is_undefined() {
        return Err(JsValue::from_str("captureStream not supported"));
    }

    // create MediaRecorder
    let media_rec_ctor_val =
        js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("MediaRecorder"))?;
    if media_rec_ctor_val.is_undefined() || media_rec_ctor_val.is_null() {
        return Err(JsValue::from_str("MediaRecorder not available"));
    }
    let media_rec_fn: Function = media_rec_ctor_val.dyn_into()?;
    let opts = Object::new();
    js_sys::Reflect::set(
        &opts,
        &JsValue::from_str("mimeType"),
        &JsValue::from_str("video/webm; codecs=vp9"),
    )?;
    let args = Array::new();
    args.push(&stream);
    args.push(&opts);
    let recorder = js_sys::Reflect::construct(&media_rec_fn, &args)?;

    // collect data
    let recorded = js_sys::Array::new();
    let recorded_clone = recorded.clone();
    let ondata = Closure::wrap(Box::new(move |e: JsValue| {
        web_sys::console::log_1(&JsValue::from_str("encoder: ondataavailable"));
        let data = js_sys::Reflect::get(&e, &JsValue::from_str("data")).unwrap_or(JsValue::NULL);
        recorded_clone.push(&data);
    }) as Box<dyn FnMut(JsValue)>);
    js_sys::Reflect::set(
        &recorder,
        &JsValue::from_str("ondataavailable"),
        ondata.as_ref().unchecked_ref(),
    )?;
    ondata.forget();

    // progress callback initial
    if progress_callback.is_function() {
        progress_callback
            .dyn_ref::<Function>()
            .unwrap()
            .call1(&JsValue::NULL, &JsValue::from_f64(0.0))
            .ok();
    }

    // start recorder
    web_sys::console::log_1(&JsValue::from_str("encoder: starting MediaRecorder"));
    js_sys::Reflect::get(&recorder, &JsValue::from_str("start"))?
        .dyn_into::<Function>()?
        .call0(&recorder)?;

    // wait duration
    let wait = js_sys::Promise::new(&mut |resolve, _| {
        let ms_f = duration_ms.max(0.0).round();
        let ms_i32 = if ms_f > (i32::MAX as f64) {
            i32::MAX
        } else {
            ms_f as i32
        };
        let callback = Closure::wrap(Box::new(move || {
            resolve.call0(&JsValue::NULL).ok();
        }) as Box<dyn FnMut()>);
        let window = web_sys::window().expect("no window");
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            callback.as_ref().unchecked_ref(),
            ms_i32,
        );
        callback.forget();
    });
    JsFuture::from(wait).await?;

    // stop and wait for onstop
    js_sys::Reflect::get(&recorder, &JsValue::from_str("stop"))?
        .dyn_into::<Function>()?
        .call0(&recorder)?;
    let stop_promise = js_sys::Promise::new(&mut |resolve, _| {
        let r = recorder.clone();
        let stop_cb = Closure::once_into_js(move || {
            resolve.call0(&JsValue::NULL).ok();
        });
        js_sys::Reflect::set(
            &r,
            &JsValue::from_str("onstop"),
            stop_cb.as_ref().unchecked_ref(),
        )
        .ok();
    });
    JsFuture::from(stop_promise).await?;
    // recorder stopped
    let recorded_count = recorded.length();
    web_sys::console::log_2(
        &JsValue::from_str("encoder: recorder stopped, chunks:"),
        &JsValue::from_f64(recorded_count as f64),
    );

    // assemble blob
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(
        &recorded,
        &web_sys::BlobPropertyBag::new(),
    )?;
    // debug blob size
    let blob_size = blob.size();
    web_sys::console::log_2(&JsValue::from_str("encoder: assembled blob size:"), &JsValue::from_f64(blob_size as f64));
    let url2 = web_sys::Url::create_object_url_with_blob(&blob)?;
    let a = web_sys::window()
        .ok_or(JsValue::from_str("No window"))?
        .document()
        .ok_or(JsValue::from_str("No document"))?
        .create_element("a")?
        .dyn_into::<web_sys::HtmlElement>()?;
    a.set_attribute("href", &url2)?;
    a.set_attribute("download", output_filename)?;
    a.click();
    web_sys::Url::revoke_object_url(&url)?;
    web_sys::Url::revoke_object_url(&url2)?;
    if progress_callback.is_function() {
        web_sys::console::log_1(&JsValue::from_str("encoder: calling final progress 1.0"));
        progress_callback
            .dyn_ref::<Function>()
            .unwrap()
            .call1(&JsValue::NULL, &JsValue::from_f64(1.0))
            .ok();
    }
    Ok(())
}
