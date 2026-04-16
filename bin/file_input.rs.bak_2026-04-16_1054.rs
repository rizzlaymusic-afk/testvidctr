use leptos::*;
use crate::state::use_app_state;
use wasm_bindgen::JsCast;

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
    view! {
        <div class="file-input">
            <input type="file" accept="video/*" on:change=onchange/>
            <p>"Drop a video or choose a file"</p>
        </div>
    }
}
