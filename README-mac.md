# OpenClaw Agent (Mac Version)

## Overview
This package provides a Mac workstation profile for `openclaw-agent`, focused on code execution and development workflows.

## Files
- `target/release/openclaw-agent`: agent binary
- `config/mac-profile.json`: high-level Mac profile
- `start-mac.sh`: startup script

## Quick Start
1. Build binary (if not built):
   ```bash
   cargo build --release
   ```
2. Start agent:
   ```bash
   ./start-mac.sh
   ```

## Runtime Config
The startup script auto-creates `~/.openclaw-agent/config.json` on first run if missing.

Update these fields before production use:
- `server_addr`
- `server_name`
- `token`
- `agent_id`

## Profile
`config/mac-profile.json` defines:
- Features: code execution, file sync, build tasks
- Limits: CPU/memory/execution time guardrails
- Whitelist: allowed commands and path patterns

## Notes
- Keep command whitelist minimal.
- Keep `file_path_whitelist` aligned with your actual project directories.
