use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures_util::SinkExt;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;

use crate::protocol::{ClientMessage, StreamType};

pub type SharedWriter<W> = Arc<Mutex<W>>;

pub async fn run_command<W>(
    id: String,
    command: String,
    args: Vec<String>,
    whitelist: Arc<HashSet<String>>,
    path_whitelist: Arc<Vec<PathBuf>>,
    writer: SharedWriter<W>,
) where
    W: SinkExt<Message> + Unpin + Send + 'static,
    W::Error: std::fmt::Display,
{
    let key = Path::new(&command)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(command.as_str())
        .to_string();

    if !whitelist.contains(&key) {
        let _ = send(
            &writer,
            ClientMessage::Error {
                id: id.clone(),
                message: format!("command '{}' not in whitelist", command),
            },
        )
        .await;
        return;
    }

    let cwd = std::env::current_dir().ok();
    if let Some(ref cwd) = cwd {
        if !is_path_allowed(cwd, &path_whitelist) {
            let _ = send(
                &writer,
                ClientMessage::Error {
                    id: id.clone(),
                    message: format!("cwd '{}' not in path whitelist", cwd.display()),
                },
            )
            .await;
            return;
        }
    }

    let mut child = match Command::new(&command).args(&args).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn() {
        Ok(c) => c,
        Err(e) => {
            let _ = send(
                &writer,
                ClientMessage::Error {
                    id: id.clone(),
                    message: format!("spawn failed: {}", e),
                },
            )
            .await;
            return;
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let writer_clone = writer.clone();
    let id_clone = id.clone();
    if let Some(stdout) = stdout {
        tokio::spawn(async move {
            stream_output(stdout, id_clone, StreamType::Stdout, writer_clone).await;
        });
    }

    let writer_clone = writer.clone();
    let id_clone = id.clone();
    if let Some(stderr) = stderr {
        tokio::spawn(async move {
            stream_output(stderr, id_clone, StreamType::Stderr, writer_clone).await;
        });
    }

    let status = child.wait().await;
    let code = status.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);

    let _ = send(&writer, ClientMessage::Exit { id, code }).await;
}

async fn stream_output<R, W>(mut reader: R, id: String, stream: StreamType, writer: SharedWriter<W>)
where
    R: AsyncRead + Unpin,
    W: SinkExt<Message> + Unpin + Send + 'static,
    W::Error: std::fmt::Display,
{
    let mut buf = vec![0u8; 4096];
    loop {
        match reader.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => {
                let chunk = String::from_utf8_lossy(&buf[..n]).to_string();
                let _ = send(
                    &writer,
                    ClientMessage::Output {
                        id: id.clone(),
                        stream,
                        chunk,
                    },
                )
                .await;
            }
            Err(_) => break,
        }
    }
}

async fn send<W>(writer: &SharedWriter<W>, msg: ClientMessage) -> Result<(), String>
where
    W: SinkExt<Message> + Unpin + Send + 'static,
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

fn is_path_allowed(path: &Path, whitelist: &[PathBuf]) -> bool {
    whitelist.iter().any(|allowed| path.starts_with(allowed))
}
