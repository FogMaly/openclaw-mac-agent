use anyhow::{anyhow, Result};

/// 检查命令是否安全
pub fn check_command_safety(command: &str) -> Result<()> {
    // 基本的安全检查
    let dangerous_patterns = [
        "rm -rf /",
        "dd if=",
        "mkfs",
        ":(){ :|:& };:",  // fork bomb
    ];

    for pattern in &dangerous_patterns {
        if command.contains(pattern) {
            return Err(anyhow!("Dangerous command detected: {}", pattern));
        }
    }

    Ok(())
}

/// 验证命令长度
pub fn validate_command_length(command: &str) -> Result<()> {
    const MAX_COMMAND_LENGTH: usize = 10000;
    
    if command.len() > MAX_COMMAND_LENGTH {
        return Err(anyhow!("Command too long: {} bytes", command.len()));
    }
    
    Ok(())
}
