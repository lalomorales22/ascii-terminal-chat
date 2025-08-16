use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    Join {
        id: Uuid,
        username: String,
    },
    Leave {
        id: Uuid,
    },
    Chat {
        id: Uuid,
        username: String,
        text: String,
        timestamp: u64,
    },
    VideoFrame {
        id: Uuid,
        username: String,
        frame: Vec<u8>, // Serialized AsciiFrame
    },
    UserList {
        users: Vec<UserInfo>,
    },
    ServerInfo {
        ngrok_url: Option<String>,
        room_name: String,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub username: String,
    pub joined_at: u64,
}

impl Message {
    #[allow(dead_code)]
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}