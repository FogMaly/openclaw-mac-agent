use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::config::Config;
use crate::executor::{self, SharedWriter};
use crate::protocol::{ClientMessage, ServerMessage};

pub async fn run_forever(cfg: Config) -> Result<(), String> {
    let mut backoff = 1_u64;
    loop {
        match run_once(cfg.clone()).await {
            Ok(()) => {
                backoff = 1;
            }
            Err(e) => {
                eprintln!("connection error: {e}");
                tokio::time::sleep(Duration::from_secs(backoff)).await;
                backoff = (backoff.saturating_mul(2)).min(cfg.reconnect_max_secs.max(1));
            }
        }
    }
}

async fn run_once(cfg: Config) -> Result<(), String> {
    let url = format!("ws://{}/agent", cfg.server_addr);
    
    let (ws_stream, _) = connect_async(&url)
        .await
        .map_err(|e| format!("WebSocket connection failed: {}", e))?;

    let (write, read) = ws_stream.split();
    let writer: Arc<Mutex<_>> = Arc::new(Mutex::new(write));

    // Send Hello
    send_message(
        &writer,
        ClientMessage::Hello {
            agent_id: cfg.agent_id.clone(),
            token: cfg.token.clone(),
        },
    )
    .await?;

    // Heartbeat task
    let heartbeat_writer = writer.clone();
    let heartbeat_secs = cfg.heartbeat_secs.max(3);
    let heartbeat_task = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(heartbeat_secs));
        loop {
            ticker.tick().await;
            let ts = now_ts();
            if send_message(&heartbeat_writer, ClientMessage::Heartbeat { ts })
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Handle incoming messages
    let whitelist = Arc::new(cfg.whitelist.clone());
    let path_whitelist = Arc::new(cfg.path_whitelist.clone());
    
    let mut read = read;
    while let Some(msg) = read.next().await {
        let msg = match msg {
            Ok(Message::Text(text)) => text,
            Ok(Message::Close(_)) => {
                heartbeat_task.abort();
                return Err("server closed connection".to_string());
            }
            Err(e) => {
                heartbeat_task.abort();
                return Err(format!("read error: {}", e));
            }
            _ => continue,
        };

        let server_msg = match serde_json::from_str::<ServerMessage>(&msg) {
            Ok(v) => v,
            Err(_) => continue,
        };

        match server_msg {
            ServerMessage::Exec { id, command, args } => {
                let writer_cloned = writer.clone();
                let whitelist_cloned = whitelist.clone();
                let path_whitelist_cloned = path_whitelist.clone();
                tokio::spawn(async move {
                    executor::run_command(
                        id,
                        command,
                        args,
                        whitelist_cloned,
                        path_whitelist_cloned,
                        writer_cloned,
                    )
                    .await;
                });
            }
            ServerMessage::Ping => {
                let ts = now_ts();
                let _ = send_message(&writer, ClientMessage::Heartbeat { ts }).await;
            }
        }
    }

    heartbeat_task.abort();
    Ok(())
}

async fn send_message<W>(writer: &Arc<Mutex<W>>, msg: ClientMessage) -> Result<(), String>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::fmt::Display,
{
    let encoded = serde_json::to_string(&msg).map_err(|e| e.to_string())?;
    let mut guard = writer.lock().await;
    guard
        .send(Message::Text(encoded))
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
}
