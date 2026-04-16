use crate::{session::SessionMessage, SharedState};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
pub struct CreateSessionResponse {
    session_id: String,
    ws_url: String,
}

pub async fn create_session(State(state): State<SharedState>) -> impl IntoResponse {
    let session_id = state.sessions.create_session();
    let ws_url = format!("/ws/{}", session_id);
    Json(CreateSessionResponse { session_id, ws_url })
}

#[derive(Serialize)]
pub struct SessionInfoResponse {
    session_id: String,
    participant_count: usize,
    playhead_ms: f64,
}

pub async fn get_session(
    Path(session_id): Path<String>,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    match state.sessions.get_session(&session_id) {
        Some(session) => {
            let s = session.state.blocking_read();
            (
                StatusCode::OK,
                Json(SessionInfoResponse {
                    session_id,
                    participant_count: s.participant_count,
                    playhead_ms: s.playhead_ms,
                }),
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "Session nicht gefunden").into_response(),
    }
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(session_id): Path<String>,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, session_id, state))
}

async fn handle_socket(socket: WebSocket, session_id: String, state: SharedState) {
    let participant_id = Uuid::new_v4().to_string()[..8].to_string();
    if state.sessions.get_session(&session_id).is_none() {
        tracing::warn!("Session {} nicht gefunden, erstelle neue", session_id);
    }
    let session = match state.sessions.get_session(&session_id) {
        Some(s) => s,
        None => {
            tracing::error!(
                "Session {} konnte nicht gefunden/erstellt werden",
                session_id
            );
            return;
        }
    };
    {
        let mut s = session.state.write().await;
        s.participant_count += 1;
    }
    let join_msg = SessionMessage::ParticipantJoined {
        participant_id: participant_id.clone(),
        participant_count: session.state.read().await.participant_count,
    };
    let _ = session.sender.send(join_msg);
    let current_state = session.state.read().await.clone();
    let sync_msg = SessionMessage::StateSync(current_state);
    let (mut sender, mut receiver) = socket.split();
    if let Ok(json) = serde_json::to_string(&sync_msg) {
        let _ = sender.send(Message::Text(json)).await;
    }
    let mut broadcast_rx = session.sender.subscribe();
    let broadcast_task = tokio::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });
    let session_clone = session.clone();
    let participant_id_clone = participant_id.clone();
    let receive_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => match serde_json::from_str::<SessionMessage>(&text) {
                    Ok(session_msg) => {
                        match &session_msg {
                            SessionMessage::TimestampUpdate { playhead_ms, .. } => {
                                let mut s = session_clone.state.write().await;
                                s.playhead_ms = *playhead_ms;
                            }
                            SessionMessage::TrimUpdate {
                                start_ms, end_ms, ..
                            } => {
                                let mut s = session_clone.state.write().await;
                                s.trim_start_ms = *start_ms;
                                s.trim_end_ms = *end_ms;
                            }
                            SessionMessage::Ping => {
                                let _ = session_clone.sender.send(SessionMessage::Pong);
                                continue;
                            }
                            _ => {}
                        }
                        let _ = session_clone.sender.send(session_msg);
                    }
                    Err(e) => {
                        tracing::warn!("Ungültige Nachricht von {}: {}", participant_id_clone, e);
                    }
                },
                Message::Ping(_) => {}
                Message::Close(_) => break,
                _ => {}
            }
        }
    });
    tokio::select! { _ = broadcast_task => {}, _ = receive_task => {}, }
    {
        let mut s = session.state.write().await;
        s.participant_count = s.participant_count.saturating_sub(1);
        let count = s.participant_count;
        let leave_msg = SessionMessage::ParticipantLeft {
            participant_id: participant_id.clone(),
            participant_count: count,
        };
        let _ = session.sender.send(leave_msg);
        if count == 0 {
            state.sessions.remove_session(&session_id);
        }
    }
    tracing::info!(
        "Teilnehmer {} hat Session {} verlassen",
        participant_id,
        session_id
    );
}
