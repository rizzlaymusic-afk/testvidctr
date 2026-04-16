use super::{
    file_input::FileInput, session_panel::SessionPanel, timeline::Timeline, toolbar::Toolbar,
    video_player::VideoPlayer,
};
use crate::state::{provide_app_state, use_app_state};
use leptos::*;

#[component]
pub fn App() -> impl IntoView {
    provide_app_state();
    let state = use_app_state();
    view! {
        <div class="app-container">
            <header class="app-header">
                <h1 class="logo">"⚡ FlashCut"</h1>
                <span class="tagline">"Privacy-First · Zero-Upload · Frame-Accurate"</span>
                <div style="margin-left:auto; display:flex; gap:8px; align-items:center;">
                    <button class="micro-anim" on:click=move |_| { state.theater_mode.set(!state.theater_mode.get()); }>{move || if state.theater_mode.get() { "Exit Theater" } else { "Theater" }}</button>
                </div>
            </header>
            <main class="app-main">
                <Show when=move || state.file.get().is_none() fallback=|| view! { <></> }>
                    <FileInput />
                </Show>
                <Show when=move || state.file.get().is_some() fallback=|| view! { <></> }>
                    <div class=move || format!("editor-layout{}", if state.theater_mode.get() { " theater-mode" } else { "" })>
                        <div class="left-pane" style=move || format!("width: {}%;", state.video_width_pct.get())>
                            <VideoPlayer />
                        </div>
                        <div class="right-pane" style=move || format!("width: {}%;", 100.0 - state.video_width_pct.get())>
                            <Timeline />
                            <Toolbar />
                        </div>
                    </div>
                </Show>
            </main>
            <Show when=move || state.file.get().is_some() fallback=|| view! { <></> }>
                <SessionPanel />
            </Show>

            /* Debug HUD for dev testing */
            <div class="debug-hud">
                <p>{move || format!("file: {}", if state.file.get().is_some() { "loaded" } else { "none" })}</p>
                <p>{move || format!("duration_ms: {:.0}", state.duration_ms.get())}</p>
                <p>{move || format!("playhead_ms: {:.0}", state.playhead_ms.get())}</p>
                <p>{move || match state.export_progress.get() { Some(p) => format!("export: {:.0}%", p * 100.0), None => "export: idle".to_string() } }</p>
            </div>
        </div>
    }
}
