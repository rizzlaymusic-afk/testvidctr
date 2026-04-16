use leptos::*;

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
        }
    }
}

pub fn provide_app_state() {
    provide_context(AppState::new());
}
pub fn use_app_state() -> AppState {
    use_context::<AppState>().expect("AppState not found")
}
