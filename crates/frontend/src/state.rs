use leptos::*;
use serde_json::json;

#[derive(Clone, Debug)]
pub struct AppState {
    pub file: RwSignal<Option<web_sys::File>>,
    pub duration_ms: RwSignal<f64>,
    pub playhead_ms: RwSignal<f64>,
    pub trim_start_ms: RwSignal<f64>,
    pub trim_end_ms: RwSignal<f64>,
    pub export_progress: RwSignal<Option<f64>>,
    pub session_id: RwSignal<Option<String>>,
    pub participant_count: RwSignal<usize>,
    pub error_message: RwSignal<Option<String>>,
    pub video_width_pct: RwSignal<f64>,
    pub theater_mode: RwSignal<bool>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            file: create_rw_signal(None),
            duration_ms: create_rw_signal(0.0),
            playhead_ms: create_rw_signal(0.0),
            trim_start_ms: create_rw_signal(0.0),
            trim_end_ms: create_rw_signal(0.0),
            export_progress: create_rw_signal(None),
            session_id: create_rw_signal(None),
            participant_count: create_rw_signal(0),
            error_message: create_rw_signal(None),
            video_width_pct: create_rw_signal(60.0),
            theater_mode: create_rw_signal(false),
        }
    }
}

/// Provide application state into Leptos context.
/// This also tries to restore a saved session from `localStorage` and
/// wires simple persistence effects for `trim_start_ms` and `trim_end_ms`.
pub fn provide_app_state() {
    let state = AppState::new();

    // Attempt to restore persisted session (best-effort)
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(saved)) = storage.get_item("flashcut:session") {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&saved) {
                    if let Some(start) = value.get("trim_start_ms").and_then(|v| v.as_f64()) {
                        state.trim_start_ms.set(start);
                    }
                    if let Some(end) = value.get("trim_end_ms").and_then(|v| v.as_f64()) {
                        state.trim_end_ms.set(end);
                    }
                    if let Some(vw) = value.get("video_width_pct").and_then(|v| v.as_f64()) {
                        state.video_width_pct.set(vw);
                    }
                    if let Some(tm) = value.get("theater_mode").and_then(|v| v.as_bool()) {
                        state.theater_mode.set(tm);
                    }
                    if let Some(sid) = value.get("session_id").and_then(|v| v.as_str()) {
                        state.session_id.set(Some(sid.to_string()));
                    }
                }
            }
        }
    }
    // Persist core session whenever key UI state changes
    {
        let s = state.clone();
        create_effect(move |_| {
            // create dependencies
            let _ = (
                s.trim_start_ms.get(),
                s.trim_end_ms.get(),
                s.video_width_pct.get(),
                s.theater_mode.get(),
                s.session_id.get().clone(),
            );
            if let Some(window) = web_sys::window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    let obj = json!({
                        "trim_start_ms": s.trim_start_ms.get(),
                        "trim_end_ms": s.trim_end_ms.get(),
                        "video_width_pct": s.video_width_pct.get(),
                        "theater_mode": s.theater_mode.get(),
                        "session_id": s.session_id.get(),
                    });
                    let _ = storage.set_item("flashcut:session", &obj.to_string());
                }
            }
        });
    }

    provide_context(state);
}

pub fn use_app_state() -> AppState {
    use_context::<AppState>().expect("AppState not found")
}
