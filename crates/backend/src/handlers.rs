use flashcut_shared::{TrimRange, WsMessage, CreateSessionRequest, CreateSessionResponse, SessionInfoResponse, SessionState};
use crate::SharedState;
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
use serde::Serialize;
use uuid::Uuid;

pub async fn create_session(
    State(state): State<SharedState>,
    Json(body): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    let session_id = state.sessions.create_session();
    // Apply initial trim range if provided
    if let Some(range) = body.initial_trim_range {
        if let Some(session) = state.sessions.get_session(&session_id) {
            let mut s = session.state.write().await;
            s.trim_start_ms = range.start_ms;
            s.trim_end_ms = range.end_ms;
        }
    }

    let ws_url = format!("/ws/{}", session_id);
    let share_url = format!("{}/?session={}", state.frontend_url, session_id);
    Json(CreateSessionResponse { session_id, ws_url, share_url })
}

// Use types from `flashcut_shared` for API DTOs

pub async fn get_session(
    Path(session_id): Path<String>,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    match state.sessions.get_session(&session_id) {
        Some(session) => {
            let s = (*session.state.read().await).clone();
            let created_at_unix = session.created_at.duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs()).unwrap_or(0);
            let resp = SessionInfoResponse {
                session_id,
                state: SessionState {
                    playhead_ms: s.playhead_ms,
                    trim_range: TrimRange { start_ms: s.trim_start_ms, end_ms: s.trim_end_ms },
                    participant_count: s.participant_count,
                },
                created_at_unix,
            };
            (StatusCode::OK, Json(resp)).into_response()
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

    // Broadcast join using shared WsMessage
    let participant_count = session.state.read().await.participant_count;
    let join_msg = WsMessage::ParticipantJoined {
        participant_id: participant_id.clone(),
        participant_count,
    };
    let _ = session.sender.send(join_msg);

    // Send current state as StateSync (map internal start/end to TrimRange)
    let current_state = (*session.state.read().await).clone();
    let sync_state = SessionState {
        playhead_ms: current_state.playhead_ms,
        trim_range: TrimRange { start_ms: current_state.trim_start_ms, end_ms: current_state.trim_end_ms },
        participant_count: current_state.participant_count,
    };
    let sync_msg = WsMessage::StateSync(sync_state);
    let (mut sender, mut receiver) = socket.split();
    if let Ok(json) = serde_json::to_string(&sync_msg) {
        let _ = sender.send(Message::Text(json)).await;
    }

    let mut broadcast_rx = session.sender.subscribe();
    let pid_bcast = participant_id.clone();

    // Task: Broadcast -> WebSocket
    let bcast_task = tokio::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            // avoid reflecting messages originating from this participant
            let skip = match &msg {
                WsMessage::TimestampUpdate { participant_id, .. } |
                WsMessage::TrimUpdate { participant_id, .. } => participant_id == &pid_bcast,
                _ => false,
            };
            if skip { continue; }
            if let Ok(json) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Task: WebSocket -> State + Broadcast
    let session_clone = session.clone();
    let participant_id_clone = participant_id.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => match serde_json::from_str::<WsMessage>(&text) {
                    Ok(ws_msg) => {
                        match &ws_msg {
                                        WsMessage::TimestampUpdate { playhead_ms, .. } => {
                                            let mut s = session_clone.state.write().await;
                                            s.playhead_ms = *playhead_ms;
                                        }
                                        WsMessage::TrimUpdate { range, .. } => {
                                            let mut s = session_clone.state.write().await;
                                            s.trim_start_ms = range.start_ms;
                                            s.trim_end_ms = range.end_ms;
                                        }
                                        WsMessage::Ping => {
                                            let _ = session_clone.sender.send(WsMessage::Pong);
                                            continue;
                                        }
                            _ => {}
                        }
                        let _ = session_clone.sender.send(ws_msg);
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

    tokio::select! { _ = bcast_task => {}, _ = recv_task => {}, }

    {
        let mut s = session.state.write().await;
        s.participant_count = s.participant_count.saturating_sub(1);
        let count = s.participant_count;
        let leave_msg = WsMessage::ParticipantLeft {
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
