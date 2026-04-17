use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use js_sys::Promise;
use wasm_bindgen_futures::JsFuture;

// Inline JS implementation of a rVFC + WebCodecs pipeline.
// This function attempts to use `VideoEncoder` and `requestVideoFrameCallback`
// to encode a trimmed segment of `file` to a WebM blob and trigger a download.
// It falls back to a canvas + MediaRecorder path if chunk bytes are not available.
#[wasm_bindgen(inline_js = r#"
export async function rvfc_trim_and_export(file, trim_start_ms, trim_end_ms, output_filename, progress_cb) {
  if (!file) throw new Error('No file provided');
  const url = URL.createObjectURL(file);
  const video = document.createElement('video');
  video.src = url;
  video.preload = 'metadata';
  video.muted = true;
  video.playsInline = true;

  await new Promise((resolve, reject) => {
    video.onloadedmetadata = () => resolve();
    video.onerror = () => reject('Failed to load metadata');
  });

  const width = video.videoWidth || 640;
  const height = video.videoHeight || 480;
  const start_s = Math.max(0, trim_start_ms / 1000);
  const end_s = Math.min(video.duration || (trim_end_ms/1000), trim_end_ms / 1000);
  const duration_ms = Math.max(0, (end_s - start_s) * 1000);

  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d');

  const chunks = [];

  // Configure VideoEncoder if available
  let encoderSupported = typeof VideoEncoder !== 'undefined';
  let encoder = null;
  if (encoderSupported) {
    try {
      encoder = new VideoEncoder({
        output: (chunk, meta) => {
          try {
            if (typeof chunk.copyTo === 'function') {
              const bytes = new Uint8Array(chunk.byteLength);
              chunk.copyTo(bytes);
              chunks.push(bytes);
            } else if (typeof chunk.toArrayBuffer === 'function') {
              // promise based fallback
              chunk.toArrayBuffer().then(buf => chunks.push(new Uint8Array(buf))).catch(()=>{});
            } else {
              console.warn('EncodedVideoChunk bytes not available on this platform');
            }
          } catch (e) {
            console.warn('Error extracting chunk bytes', e);
          }
        },
        error: (e) => console.error('VideoEncoder error', e),
      });
      encoder.configure({ codec: 'vp8', width, height, bitrate: 2_000_000, framerate: 30 });
    } catch (e) {
      console.warn('VideoEncoder setup failed', e);
      encoder = null;
    }
  }

  // Seek to start
  video.currentTime = start_s;
  await new Promise((resolve) => {
    let resolved = false;
    const onseek = () => { if (!resolved) { resolved = true; resolve(); } };
    video.onseeked = onseek;
    // timeout safety
    setTimeout(onseek, 500);
  });

  try { await video.play(); } catch(e) { /* ignore autoplay errors */ }

  let done = false;

  function frameCallback(now, metadata) {
    try {
      ctx.drawImage(video, 0, 0, width, height);
      // create ImageBitmap and feed encoder when available
      if (encoder) {
        createImageBitmap(canvas).then(bitmap => {
          try { encoder.encode(bitmap); } catch(e) { console.warn('encode error', e); }
          try { if (bitmap.close) bitmap.close(); } catch(_) {}
        }).catch(() => {});
      }
    } catch (e) {
      console.warn('drawImage error', e);
    }

    if (video.currentTime >= end_s) {
      done = true;
    } else {
      try { video.requestVideoFrameCallback(frameCallback); } catch(e) { /* rVFC not available */ }
    }
  }

  if (typeof video.requestVideoFrameCallback === 'function') {
    try { video.requestVideoFrameCallback(frameCallback); } catch(e) { /* ignore */ }
  } else {
    // fallback to interval if rVFC not present
    const iv = setInterval(() => {
      ctx.drawImage(video, 0, 0, width, height);
      if (encoder) {
        createImageBitmap(canvas).then(bitmap => { try { encoder.encode(bitmap); } catch(e){} finally { if (bitmap.close) bitmap.close(); } }).catch(()=>{});
      }
      if (video.currentTime >= end_s) { clearInterval(iv); done = true; }
    }, 1000/30);
  }

  // Wait until done
  await new Promise(resolve => {
    const t = setInterval(() => { if (done) { clearInterval(t); resolve(); } }, 30);
  });

  // finalize encoder if used
  if (encoder) {
    try { await encoder.flush(); } catch(e) { console.warn('flush error', e); }
    try { encoder.close(); } catch(_) {}
  }

  // Build blob
  let blob = null;
  const good = chunks.filter(Boolean);
  if (good.length > 0) {
    let total = good.reduce((acc, c) => acc + c.length, 0);
    const out = new Uint8Array(total);
    let off = 0;
    for (const c of good) { out.set(c, off); off += c.length; }
    blob = new Blob([out.buffer], { type: 'video/webm' });
  } else {
    // fallback: record canvas stream via MediaRecorder
    try {
      const stream = canvas.captureStream(30);
      const recChunks = [];
      const mr = new MediaRecorder(stream, { mimeType: 'video/webm; codecs=vp9' });
      mr.ondataavailable = (e) => recChunks.push(e.data);
      mr.start();
      await new Promise(r => setTimeout(r, duration_ms));
      mr.stop();
      await new Promise(r => { mr.onstop = () => r(); });
      blob = new Blob(recChunks, { type: 'video/webm' });
    } catch (e) {
      throw new Error('Both WebCodecs and MediaRecorder fallback failed: ' + e);
    }
  }

  const outUrl = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = outUrl;
  a.download = output_filename || 'trim.webm';
  a.click();
  URL.revokeObjectURL(url);
  URL.revokeObjectURL(outUrl);

  if (progress_cb && typeof progress_cb === 'function') {
    try { progress_cb(1.0); } catch (_) {}
  }
  return true;
}
"#)]
extern "C" {
  #[wasm_bindgen(js_name = rvfc_trim_and_export)]
  fn rvfc_trim_and_export_promise(
    file: &JsValue,
    trim_start_ms: f64,
    trim_end_ms: f64,
    output_filename: &str,
    progress_cb: &JsValue,
  ) -> Promise;
}

#[wasm_bindgen]
pub async fn rvfc_trim_and_export(
  file: JsValue,
  trim_start_ms: f64,
  trim_end_ms: f64,
  output_filename: &str,
  progress_cb: &JsValue,
) -> Result<JsValue, JsValue> {
  let p = rvfc_trim_and_export_promise(
    &file,
    trim_start_ms,
    trim_end_ms,
    output_filename,
    progress_cb,
  );
  let v = JsFuture::from(p).await?;
  Ok(v)
}
