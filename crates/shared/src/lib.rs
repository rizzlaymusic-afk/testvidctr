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
        self.end_ms - self.start_ms
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
        count: usize,
    },
    ParticipantLeft {
        participant_id: String,
        count: usize,
    },
    StateSync {
        playhead_ms: f64,
        trim_range: TrimRange,
        participant_count: usize,
    },
    Ping,
    Pong,
    Error {
        message: String,
    },
}
