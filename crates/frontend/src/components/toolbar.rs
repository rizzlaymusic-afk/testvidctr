use leptos::*;
use crate::state::use_app_state;
use js_sys::{Array, Function, Object, Reflect};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlAnchorElement, HtmlVideoElement, Url};

#[component]
pub fn Toolbar() -> impl IntoView {
    let state = use_app_state();
    let state_for_export = state.clone();
    let on_export = move |_| {
        let state_clone = state_for_export.clone();
        let file_opt = state_clone.file.get();
        let start_ms = state_clone.trim_start_ms.get();
        let end_ms = state_clone.trim_end_ms.get();
        if let Some(file) = file_opt {
            if end_ms <= start_ms {
                state.error_message.set(Some("Trim range must be positive".to_string()));
                return;
            }
            state.error_message.set(None);
            state.export_progress.set(Some(0.05));
            spawn_local(async move {
                if let Err(err) = export_trimmed(file, start_ms, end_ms, state_clone.clone()).await {
                    state_clone.error_message.set(Some(format!("Export failed: {:?}", err)));
                    state_clone.export_progress.set(None);
                } else {
                    state_clone.export_progress.set(None);
                }
            });
        } else {
            state.error_message.set(Some("No file selected".to_string()));
        }
    };
    view! {
        <div class="toolbar">
            <button class="export-btn micro-anim" on:click=on_export>"Export"</button>
            <Show when=move || state.export_progress.get().is_some() fallback=|| view! { <></> }>
                <div class="progress-bar">
                    <div class="progress-bar-fill" style=move || format!("width: {}%;", state.export_progress.get().unwrap_or(0.0) * 100.0)></div>
                </div>
            </Show>
        </div>
    }
}

async fn export_trimmed(
    file: web_sys::File,
    trim_start_ms: f64,
    trim_end_ms: f64,
    state: crate::state::AppState,
) -> Result<(), wasm_bindgen::JsValue> {
    // Delegate to core-wasm pipeline wrapper and update UI via a JS callback.
    let cb = Closure::wrap(Box::new(move |v: JsValue| {
        if let Some(n) = v.as_f64() {
            state.export_progress.set(Some(n));
        }
    }) as Box<dyn FnMut(JsValue)>);

    // Call pipeline wrapper exported by flashcut-core-wasm.
    let res = flashcut_core_wasm::pipeline::pipeline_trim_and_export(
        file,
        trim_start_ms,
        trim_end_ms,
        "flashcut-trim.webm",
        cb.as_ref(),
    )
    .await;

    // Drop the callback so it can be freed now that export finished.
    drop(cb);

    match res {
        Ok(_) => {
            state.export_progress.set(None);
            Ok(())
        }
        Err(e) => {
            state.error_message.set(Some(format!("Export failed: {:?}", e)));
            state.export_progress.set(None);
            Err(e)
        }
    }
}
