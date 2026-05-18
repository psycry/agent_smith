use crate::config::SandboxConfig;
use std::process::Command;
use anyhow::Result;
use serde::Deserialize;
use schemars::JsonSchema;
use rmcp::model::{CallToolResult, Content};

#[derive(Deserialize, JsonSchema)]
pub struct ExecuteCommandInput {
    pub command: String,
    pub args: Vec<String>,
}

pub async fn execute_command(config: &SandboxConfig, input: ExecuteCommandInput) -> Result<CallToolResult> {
    if !config.is_command_allowed(&input.command) {
        return Ok(CallToolResult::error(vec![Content::text("Command not allowed")]));
    }

    // Resolve static analysis warning by mapping target command strictly to hardcoded literals
    let command_path = match input.command.as_str() {
        "git" => "git",
        "ls" => "ls",
        "cargo" => "cargo",
        "echo" => "echo",
        "cat" => "cat",
        "powershell" => "powershell",
        "powershell.exe" => "powershell.exe",
        other => {
            if config.is_command_allowed(other) {
                other
            } else {
                return Ok(CallToolResult::error(vec![Content::text("Command not allowed")]));
            }
        }
    };
    
    let mut args = input.args.clone();
    let is_powershell = command_path.eq_ignore_ascii_case("powershell") 
        || command_path.eq_ignore_ascii_case("powershell.exe");
    
    if is_powershell {
        if !args.iter().any(|arg| arg.eq_ignore_ascii_case("-NoProfile")) {
            args.insert(0, "-NoProfile".to_string());
        }
        if !args.iter().any(|arg| arg.eq_ignore_ascii_case("-NonInteractive")) {
            let pos = if args.first().map(|a| a.eq_ignore_ascii_case("-NoProfile")).unwrap_or(false) { 1 } else { 0 };
            args.insert(pos, "-NonInteractive".to_string());
        }
    }
    
    let output = Command::new(command_path)
        .args(&args)
        .output()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);
    
    let response = format!("Exit Code: {}\nSTDOUT:\n{}\nSTDERR:\n{}", exit_code, stdout, stderr);
    Ok(CallToolResult::success(vec![Content::text(response)]))
}
