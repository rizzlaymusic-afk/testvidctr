use leptos::*;
use crate::state::use_app_state;

#[component]
pub fn SessionPanel() -> impl IntoView {
    let state = use_app_state();
    view! {
        <aside class="session-panel">
            <div>{move || format!("Session: {}", state.session_id.get().unwrap_or_default())}</div>
            <div>{move || format!("Participants: {}", state.participant_count.get())}</div>
            <div class="error">{move || state.error_message.get().unwrap_or_default()}</div>
        </aside>
    }
}
