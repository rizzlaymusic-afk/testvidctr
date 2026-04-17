use serde::{Deserialize, Serialize};

/// Repräsentiert einen Schnittbereich
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrimRange {
    pub start_ms: f64,
    pub end_ms: f64,
}

impl TrimRange {
    pub fn new(start_ms: f64, end_ms: f64) -> Self {
        Self { start_ms, end_ms }
    }
    pub fn duration_ms(&self) -> f64 {
        (self.end_ms - self.start_ms).max(0.0)
    }
    pub fn is_valid(&self) -> bool {
        self.start_ms >= 0.0 && self.end_ms > self.start_ms
    }
    pub fn clamped(&self, video_duration_ms: f64) -> Self {
        Self {
            start_ms: self.start_ms.clamp(0.0, video_duration_ms),
            end_ms: self.end_ms.clamp(self.start_ms, video_duration_ms),
        }
    }
}

impl Default for TrimRange {
    fn default() -> Self {
        Self { start_ms: 0.0, end_ms: 0.0 }
    }
}

/// Metadaten einer Video-Datei (vom WASM-Decoder zurückgegeben)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMetadata {
    pub duration_ms: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub mime_type: String,
    pub file_name: String,
    pub file_size: u64,
}

impl Default for VideoMetadata {
    fn default() -> Self {
        Self {
            duration_ms: 0.0,
            width: 0,
            height: 0,
            fps: 30.0,
            mime_type: String::new(),
            file_name: String::new(),
            file_size: 0,
        }
    }
}

/// Zustand einer aktiven Kollaborations-Session (Payload für StateSync)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionState {
    pub playhead_ms: f64,
    pub trim_range: TrimRange,
    pub participant_count: usize,
}

/// REST-API: Create Session Request
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateSessionRequest {
    pub initial_trim_range: Option<TrimRange>,
}

/// REST-API: Create Session Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub ws_url: String,
    pub share_url: String,
}

/// REST-API: Session Info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfoResponse {
    pub session_id: String,
    pub state: SessionState,
    pub created_at_unix: u64,
}

/// WebSocket-Protokoll-Nachrichten (geteilt zwischen Client und Server)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsMessage {
    TimestampUpdate {
        participant_id: String,
        playhead_ms: f64,
    },
    TrimUpdate {
        participant_id: String,
        range: TrimRange,
    },
    ParticipantJoined {
        participant_id: String,
        participant_count: usize,
    },
    ParticipantLeft {
        participant_id: String,
        participant_count: usize,
    },
    StateSync(SessionState),
    Ping,
    Pong,
    Error { code: String, message: String },
}
