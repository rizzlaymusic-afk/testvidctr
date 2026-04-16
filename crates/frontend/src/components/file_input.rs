use crate::state::use_app_state;
use js_sys::Array;
use leptos::*;
use leptos::ev::KeyboardEvent;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn FileInput() -> impl IntoView {
    let state = use_app_state();
    let onchange = move |ev: web_sys::Event| {
        if let Some(input) = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        {
            if let Some(files) = input.files() {
                if let Some(file) = files.get(0) {
                    state.file.set(Some(file));
                }
            }
        }
    };

    // Dev helper: fetch a bundled or remote sample video and set it as the current file
    let load_sample = {
        let state = state.clone();
        move |_| {
            spawn_local(async move {
                let window = match web_sys::window() {
                    Some(w) => w,
                    None => return,
                };
                let local_url = "/assets/sample/sample.webm";
                // Try local asset first, fall back to an MDN sample if missing
                let resp_val =
                    wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(local_url)).await;
                let response = match resp_val {
                    Ok(v) => v.dyn_into::<web_sys::Response>().ok(),
                    Err(_) => {
                        let remote = "https://interactive-examples.mdn.mozilla.net/media/cc0-videos/flower.mp4";
                        match wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(remote))
                            .await
                        {
                            Ok(rv) => rv.dyn_into::<web_sys::Response>().ok(),
                            Err(_) => None,
                        }
                    }
                };

                if let Some(resp) = response {
                    if resp.ok() {
                        match wasm_bindgen_futures::JsFuture::from(resp.blob().unwrap()).await {
                            Ok(blob_val) => {
                                let blob = blob_val.dyn_into::<web_sys::Blob>().unwrap();
                                let parts = Array::new();
                                parts.push(&blob);
                                // Construct a File from the Blob parts. If this fails, set an error.
                                match web_sys::File::new_with_blob_sequence(&parts, "sample.webm") {
                                    Ok(file) => {
                                        state.file.set(Some(file));
                                    }
                                    Err(_) => {
                                        state.error_message.set(Some(
                                            "Failed to create File from blob".to_string(),
                                        ));
                                    }
                                }
                            }
                            Err(_) => state
                                .error_message
                                .set(Some("Failed to read sample blob".to_string())),
                        }
                    } else {
                        state
                            .error_message
                            .set(Some("Failed to fetch sample".to_string()));
                    }
                } else {
                    state.error_message.set(Some("Fetch error".to_string()));
                }
            });
        }
    };

    // Keyboard helper: pressing Enter/Space when the drop zone is focused opens file dialog
    let on_dropzone_key = move |ev: KeyboardEvent| {
        let k = ev.key();
        if k == "Enter" || k == " " {
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Some(input_el) = document.get_element_by_id("file-input")
                        .and_then(|e| e.dyn_into::<web_sys::HtmlInputElement>().ok())
                    {
                        input_el.click();
                    }
                }
            }
        }
    };

    view! {
        <div class="drop-zone file-input" role="region" aria-label="File input drop zone" tabindex="0" on:keydown=on_dropzone_key>
            <input id="file-input" class="micro-anim" type="file" accept="video/*" on:change=onchange aria-label="Choose a video file" />
            <p>"Drop a video or " <label for="file-input">"choose a file"</label></p>
            <button type="button" class="micro-anim" on:click=load_sample aria-label="Load sample video (development)">{|| "Load Sample Video (dev)" }</button>
        </div>
    }
}
