use crate::state::use_app_state;
use leptos::ev::{MouseEvent, KeyboardEvent};
use leptos::*;
use wasm_bindgen::JsCast;

#[component]
pub fn Timeline() -> impl IntoView {
    let state = use_app_state();
    let trim_start_pct = move || {
        let duration = state.duration_ms.get();
        if duration <= 0.0 {
            0.0
        } else {
            (state.trim_start_ms.get() / duration) * 100.0
        }
    };
    let trim_end_pct = move || {
        let duration = state.duration_ms.get();
        if duration <= 0.0 {
            100.0
        } else {
            (state.trim_end_ms.get() / duration) * 100.0
        }
    };
    let playhead_pct = move || {
        let duration = state.duration_ms.get();
        if duration <= 0.0 {
            0.0
        } else {
            (state.playhead_ms.get() / duration) * 100.0
        }
    };
    let on_timeline_click = move |ev: MouseEvent| {
        let click_x = ev.offset_x() as f64;
        let target_width = ev.current_target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok())
            .map(|el| el.client_width() as f64)
            .unwrap_or(640.0);
        let pct = (click_x / target_width).clamp(0.0, 1.0);
        let new_playhead = pct * state.duration_ms.get();
        state.playhead_ms.set(new_playhead);
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(video_el) = document.get_element_by_id("player")
                    .and_then(|el| el.dyn_into::<web_sys::HtmlVideoElement>().ok())
                {
                    video_el.set_current_time(new_playhead / 1000.0);
                }
            }
        }
    };

    // Keyboard handlers for trim handles: Arrow keys move by 100ms steps
    let on_start_handle_key = {
        let state = state.clone();
        move |ev: KeyboardEvent| {
            let key = ev.key();
            let step = 100.0; // ms
            let mut new_start = state.trim_start_ms.get();
            match key.as_str() {
                "ArrowLeft" => new_start = (new_start - step).max(0.0),
                "ArrowRight" => new_start = (new_start + step).min(state.trim_end_ms.get()),
                _ => return,
            }
            state.trim_start_ms.set(new_start);
            ev.prevent_default();
        }
    };

    let on_end_handle_key = {
        let state = state.clone();
        move |ev: KeyboardEvent| {
            let key = ev.key();
            let step = 100.0; // ms
            let mut new_end = state.trim_end_ms.get();
            match key.as_str() {
                "ArrowLeft" => new_end = (new_end - step).max(state.trim_start_ms.get()),
                "ArrowRight" => new_end = (new_end + step).min(state.duration_ms.get()),
                _ => return,
            }
            state.trim_end_ms.set(new_end);
            ev.prevent_default();
        }
    };
    view! {
        <div class="timeline-container">
            <div class="timeline-labels">
                <span class="time-label">{move || format_timecode(state.trim_start_ms.get())}</span>
                <span class="time-label center">{move || format_timecode(state.playhead_ms.get())}</span>
                <span class="time-label right">{move || format_timecode(state.trim_end_ms.get())}</span>
            </div>
            <div class="timeline-track" on:click=on_timeline_click>
                <div class="timeline-excluded" style=move || format!("width: {}%", trim_start_pct()) />
                <div class="timeline-active" style=move || format!("left: {}%; width: {}%", trim_start_pct(), trim_end_pct() - trim_start_pct()) />
                <div class="timeline-excluded right" style=move || format!("left: {}%; width: {}%", trim_end_pct(), 100.0 - trim_end_pct()) />
                <div class="trim-handle start" tabindex="0" role="slider" aria-label="Trim start" aria-valuemin=move || "0" aria-valuemax=move || state.duration_ms.get().to_string() aria-valuenow=move || state.trim_start_ms.get().to_string() on:keydown=on_start_handle_key style=move || format!("left: {}%", trim_start_pct()) />
                <div class="trim-handle end" tabindex="0" role="slider" aria-label="Trim end" aria-valuemin=move || state.trim_start_ms.get().to_string() aria-valuemax=move || state.duration_ms.get().to_string() aria-valuenow=move || state.trim_end_ms.get().to_string() on:keydown=on_end_handle_key style=move || format!("left: {}%", trim_end_pct()) />
                <div class="playhead" style=move || format!("left: {}%", playhead_pct()) />
            </div>
            <div class="timeline-duration">"Dauer: " {move || format_timecode(state.duration_ms.get())} " · Schnitt: " {move || format_timecode(state.trim_end_ms.get() - state.trim_start_ms.get())}</div>
        </div>
    }
}

fn format_timecode(ms: f64) -> String {
    let total_secs = (ms / 1000.0) as u64;
    let minutes = total_secs / 60;
    let seconds = total_secs % 60;
    let millis = (ms % 1000.0) as u64;
    format!("{:02}:{:02}.{:03}", minutes, seconds, millis)
}
