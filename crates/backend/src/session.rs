use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

pub const MAX_PARTICIPANTS: usize = 10;
const BROADCAST_CAPACITY: usize = 64;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub created_at: std::time::SystemTime,
    pub sender: broadcast::Sender<SessionMessage>,
    pub state: Arc<tokio::sync::RwLock<SessionState>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionState {
    pub playhead_ms: f64,
    pub trim_start_ms: f64,
    pub trim_end_ms: f64,
    pub participant_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum SessionMessage {
    TimestampUpdate {
        participant_id: String,
        playhead_ms: f64,
    },
    TrimUpdate {
        participant_id: String,
        start_ms: f64,
        end_ms: f64,
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
}

pub struct SessionStore {
    sessions: DashMap<String, Session>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
        }
    }
    pub fn create_session(&self) -> String {
        let id = Uuid::new_v4().to_string()[..8].to_string();
        let (sender, _) = broadcast::channel(BROADCAST_CAPACITY);
        let session = Session {
            id: id.clone(),
            created_at: std::time::SystemTime::now(),
            sender,
            state: Arc::new(tokio::sync::RwLock::new(SessionState::default())),
        };
        self.sessions.insert(id.clone(), session);
        tracing::info!("Neue Session erstellt: {}", id);
        id
    }
    pub fn get_session(&self, id: &str) -> Option<Session> {
        self.sessions.get(id).map(|s| s.clone())
    }
    pub fn remove_session(&self, id: &str) {
        self.sessions.remove(id);
        tracing::info!("Session entfernt: {}", id);
    }
}
