use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct Config {
    pub server_addr: String,
    pub agent_id: String,
    pub token: Option<String>,
    pub heartbeat_secs: u64,
    pub reconnect_max_secs: u64,
    pub whitelist: HashSet<String>,
    pub path_whitelist: Vec<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FileConfig {
    server_addr: Option<String>,
    agent_id: Option<String>,
    token: Option<String>,
    heartbeat_secs: Option<u64>,
    reconnect_max_secs: Option<u64>,
    command_whitelist: Option<Vec<String>>,
    file_path_whitelist: Option<Vec<String>>,
}

impl FileConfig {
    fn defaults() -> Self {
        Self {
            server_addr: Some("127.0.0.1:34060".to_string()),
            agent_id: Some(default_agent_id()),
            token: Some("change-me".to_string()),
            heartbeat_secs: Some(20),
            reconnect_max_secs: Some(30),
            command_whitelist: Some(vec![
                "echo".to_string(),
                "ls".to_string(),
                "pwd".to_string(),
            ]),
            file_path_whitelist: Some(vec![
                "$HOME".to_string(),
                "/tmp".to_string(),
            ]),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let config_path = config_file_path()?;
        let mut merged = FileConfig::defaults();

        ensure_config_file(&config_path, &merged)?;

        if let Ok(local) = load_file_config(&config_path) {
            merged = merge(merged, local);
        }

        let server_addr = merged
            .server_addr
            .unwrap_or_else(|| "127.0.0.1:34060".to_string());

        let agent_id = merged.agent_id.unwrap_or_else(default_agent_id);
        let token = merged.token.filter(|v| !v.is_empty());
        let heartbeat_secs = merged.heartbeat_secs.unwrap_or(20).max(3);
        let reconnect_max_secs = merged.reconnect_max_secs.unwrap_or(30).max(1);

        let whitelist = merged
            .command_whitelist
            .unwrap_or_default()
            .into_iter()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .collect::<HashSet<_>>();

        if whitelist.is_empty() {
            return Err("command whitelist cannot be empty".to_string());
        }

        let path_whitelist = normalize_path_whitelist(merged.file_path_whitelist.unwrap_or_default());

        Ok(Self {
            server_addr,
            agent_id,
            token,
            heartbeat_secs,
            reconnect_max_secs,
            whitelist,
            path_whitelist,
        })
    }
}

fn config_file_path() -> Result<PathBuf, String> {
    if let Ok(raw) = env::var("OC_CONFIG_FILE") {
        let p = expand_home(&raw);
        return Ok(p);
    }

    let base = if let Ok(raw) = env::var("OC_CONFIG_DIR") {
        expand_home(&raw)
    } else {
        home_dir()?.join(".openclaw-agent")
    };

    Ok(base.join("config.json"))
}

fn ensure_config_file(path: &Path, defaults: &FileConfig) -> Result<(), String> {
    if path.exists() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create config dir failed: {e}"))?;
    }

    let content = serde_json::to_string_pretty(defaults)
        .map_err(|e| format!("serialize default config failed: {e}"))?;
    fs::write(path, content).map_err(|e| format!("write config file failed: {e}"))
}

fn load_file_config(path: &Path) -> Result<FileConfig, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("read config failed: {e}"))?;
    serde_json::from_str::<FileConfig>(&content).map_err(|e| format!("parse config failed: {e}"))
}

fn merge(mut base: FileConfig, incoming: FileConfig) -> FileConfig {
    if incoming.server_addr.is_some() {
        base.server_addr = incoming.server_addr;
    }
    if incoming.agent_id.is_some() {
        base.agent_id = incoming.agent_id;
    }
    if incoming.token.is_some() {
        base.token = incoming.token;
    }
    if incoming.heartbeat_secs.is_some() {
        base.heartbeat_secs = incoming.heartbeat_secs;
    }
    if incoming.reconnect_max_secs.is_some() {
        base.reconnect_max_secs = incoming.reconnect_max_secs;
    }
    if incoming.command_whitelist.is_some() {
        base.command_whitelist = incoming.command_whitelist;
    }
    if incoming.file_path_whitelist.is_some() {
        base.file_path_whitelist = incoming.file_path_whitelist;
    }
    base
}

fn normalize_path_whitelist(raw: Vec<String>) -> Vec<PathBuf> {
    raw.into_iter()
        .map(|p| expand_home(&p))
        .filter_map(|p| fs::canonicalize(&p).ok().or(Some(p)))
        .collect()
}

fn expand_home(input: &str) -> PathBuf {
    if let Some(stripped) = input.strip_prefix("$HOME/") {
        if let Ok(home) = home_dir() {
            return home.join(stripped);
        }
    }

    if input == "$HOME" {
        if let Ok(home) = home_dir() {
            return home;
        }
    }

    if let Some(stripped) = input.strip_prefix("~/") {
        if let Ok(home) = home_dir() {
            return home.join(stripped);
        }
    }

    if input == "~" {
        if let Ok(home) = home_dir() {
            return home;
        }
    }

    PathBuf::from(input)
}

fn default_agent_id() -> String {
    env::var("HOSTNAME").unwrap_or_else(|_| "openclaw-agent".to_string())
}

fn home_dir() -> Result<PathBuf, String> {
    env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "HOME not set".to_string())
}
