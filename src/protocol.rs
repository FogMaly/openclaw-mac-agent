use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Exec {
        id: String,
        command: String,
        #[serde(default)]
        args: Vec<String>,
    },
    Ping,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Hello { agent_id: String, token: Option<String> },
    Heartbeat { ts: u64 },
    Output { id: String, stream: StreamType, chunk: String },
    Exit { id: String, code: i32 },
    Error { id: String, message: String },
}

#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum StreamType {
    Stdout,
    Stderr,
}
