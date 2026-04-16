use leptos::*;
use crate::state::use_app_state;

#[component]
pub fn Toolbar() -> impl IntoView {
    let state = use_app_state();
    let on_export = move |_| {
        state.export_progress.set(Some(0.0));
    };
    view! {
        <div class="toolbar">
            <button on:click=on_export>"Export"</button>
        </div>
    }
}
