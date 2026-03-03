use std::fs;
use std::io::Cursor;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use quinn::crypto::rustls::QuicClientConfig;
use quinn::{ClientConfig, Connection, Endpoint, SendStream};
use rustls::RootCertStore;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Mutex;

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
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse().map_err(err_to_string)?)
        .map_err(err_to_string)?;

    let client_cfg = build_client_config(&cfg)?;
    endpoint.set_default_client_config(client_cfg);

    let connecting = endpoint
        .connect(cfg.server_addr, &cfg.server_name)
        .map_err(err_to_string)?;
    let connection = connecting.await.map_err(err_to_string)?;

    handle_connection(connection, cfg).await
}

async fn handle_connection(connection: Connection, cfg: Config) -> Result<(), String> {
    let (send, recv) = connection.open_bi().await.map_err(err_to_string)?;
    let writer: SharedWriter = Arc::new(Mutex::new(send));

    send_message(
        &writer,
        ClientMessage::Hello {
            agent_id: cfg.agent_id.clone(),
            token: cfg.token.clone(),
        },
    )
    .await?;

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

    let whitelist = Arc::new(cfg.whitelist.clone());
    let path_whitelist = Arc::new(cfg.path_whitelist.clone());
    let mut reader = BufReader::new(recv);
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).await.map_err(err_to_string)?;
        if n == 0 {
            heartbeat_task.abort();
            return Err("server closed stream".to_string());
        }

        let msg = match serde_json::from_str::<ServerMessage>(line.trim()) {
            Ok(v) => v,
            Err(_) => continue,
        };

        match msg {
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
}

fn build_client_config(cfg: &Config) -> Result<ClientConfig, String> {
    let pem = fs::read(&cfg.ca_cert_path).map_err(err_to_string)?;
    let mut cursor = Cursor::new(pem);

    let mut roots = RootCertStore::empty();
    for cert in rustls_pemfile::certs(&mut cursor) {
        let cert = cert.map_err(err_to_string)?;
        roots.add(cert).map_err(err_to_string)?;
    }

    let tls = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    let quic = QuicClientConfig::try_from(tls).map_err(err_to_string)?;
    Ok(ClientConfig::new(Arc::new(quic)))
}

async fn send_message(writer: &Arc<Mutex<SendStream>>, msg: ClientMessage) -> Result<(), String> {
    let encoded = serde_json::to_vec(&msg).map_err(err_to_string)?;
    let mut guard = writer.lock().await;
    guard.write_all(&encoded).await.map_err(err_to_string)?;
    guard.write_all(b"\n").await.map_err(err_to_string)?;
    Ok(())
}

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
}

fn err_to_string<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}
