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
    
    let output = Command::new(&input.command)
        .args(&input.args)
        .output()?;
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);
    
    let response = format!("Exit Code: {}\nSTDOUT:\n{}\nSTDERR:\n{}", exit_code, stdout, stderr);
    Ok(CallToolResult::success(vec![Content::text(response)]))
}
