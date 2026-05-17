use crate::config::SandboxConfig;
use std::fs;
use anyhow::Result;
use serde::Deserialize;
use schemars::JsonSchema;
use rmcp::model::{CallToolResult, Content};

#[derive(Deserialize, JsonSchema)]
pub struct ReadFileInput {
    pub path: String,
}

pub async fn read_file(config: &SandboxConfig, input: ReadFileInput) -> Result<CallToolResult> {
    if !config.is_path_allowed(&input.path) {
        return Ok(CallToolResult::error(vec![Content::text("Path not allowed")]));
    }
    
    let content = fs::read_to_string(&input.path)?;
    Ok(CallToolResult::success(vec![Content::text(content)]))
}

#[derive(Deserialize, JsonSchema)]
pub struct WriteFileInput {
    pub path: String,
    pub content: String,
}

pub async fn write_file(config: &SandboxConfig, input: WriteFileInput) -> Result<CallToolResult> {
    if !config.is_path_allowed(&input.path) {
        return Ok(CallToolResult::error(vec![Content::text("Path not allowed")]));
    }
    
    fs::write(&input.path, &input.content)?;
    Ok(CallToolResult::success(vec![Content::text("File written successfully")]))
}

#[derive(Deserialize, JsonSchema)]
pub struct ListDirectoryInput {
    pub path: String,
}

pub async fn list_directory(config: &SandboxConfig, input: ListDirectoryInput) -> Result<CallToolResult> {
    if !config.is_path_allowed(&input.path) {
        return Ok(CallToolResult::error(vec![Content::text("Path not allowed")]));
    }
    
    let entries = fs::read_dir(&input.path)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    
    let content = entries.join("\n");
    Ok(CallToolResult::success(vec![Content::text(content)]))
}
