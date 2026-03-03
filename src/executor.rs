use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use quinn::SendStream;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::sync::Mutex;

use crate::protocol::{ClientMessage, StreamType};

pub type SharedWriter = Arc<Mutex<SendStream>>;

pub async fn run_command(
    id: String,
    command: String,
    args: Vec<String>,
    whitelist: Arc<HashSet<String>>,
    path_whitelist: Arc<Vec<PathBuf>>,
    writer: SharedWriter,
) {
    let key = Path::new(&command)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(command.as_str())
        .to_string();

    if !whitelist.contains(&key) {
        let _ = send(
            &writer,
            ClientMessage::Error {
                id,
                message: format!("command not allowed: {key}"),
            },
        )
        .await;
        return;
    }

    if let Some(bad) = find_disallowed_path(&args, &path_whitelist) {
        let _ = send(
            &writer,
            ClientMessage::Error {
                id,
                message: format!("path not allowed: {bad}"),
            },
        )
        .await;
        return;
    }

    let mut child = match Command::new(&command)
        .args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
    {
        Ok(v) => v,
        Err(e) => {
            let _ = send(
                &writer,
                ClientMessage::Error {
                    id,
                    message: format!("spawn failed: {e}"),
                },
            )
            .await;
            return;
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let out_task = tokio::spawn(pump(stdout, id.clone(), StreamType::Stdout, writer.clone()));
    let err_task = tokio::spawn(pump(stderr, id.clone(), StreamType::Stderr, writer.clone()));

    let status = child.wait().await;
    let _ = out_task.await;
    let _ = err_task.await;

    match status {
        Ok(s) => {
            let code = s.code().unwrap_or(-1);
            let _ = send(&writer, ClientMessage::Exit { id, code }).await;
        }
        Err(e) => {
            let _ = send(
                &writer,
                ClientMessage::Error {
                    id,
                    message: format!("wait failed: {e}"),
                },
            )
            .await;
        }
    }
}

fn find_disallowed_path(args: &[String], path_whitelist: &[PathBuf]) -> Option<String> {
    for arg in args {
        if !looks_like_path(arg) {
            continue;
        }

        let path = PathBuf::from(arg);
        let candidate = std::fs::canonicalize(&path).unwrap_or(path);
        let allowed = path_whitelist.iter().any(|root| candidate.starts_with(root));

        if !allowed {
            return Some(arg.clone());
        }
    }

    None
}

fn looks_like_path(value: &str) -> bool {
    value.starts_with('/') || value.starts_with("./") || value.starts_with("../") || value.contains('/')
}

async fn pump<R>(stream: Option<R>, id: String, stream_type: StreamType, writer: SharedWriter)
where
    R: AsyncRead + Unpin,
{
    if let Some(mut s) = stream {
        let mut buf = [0_u8; 1024];
        loop {
            match s.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = String::from_utf8_lossy(&buf[..n]).to_string();
                    if send(
                        &writer,
                        ClientMessage::Output {
                            id: id.clone(),
                            stream: stream_type,
                            chunk,
                        },
                    )
                    .await
                    .is_err()
                    {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    }
}

async fn send(writer: &SharedWriter, msg: ClientMessage) -> Result<(), ()> {
    let encoded = serde_json::to_vec(&msg).map_err(|_| ())?;
    let mut guard = writer.lock().await;
    guard.write_all(&encoded).await.map_err(|_| ())?;
    guard.write_all(b"\n").await.map_err(|_| ())?;
    Ok(())
}
