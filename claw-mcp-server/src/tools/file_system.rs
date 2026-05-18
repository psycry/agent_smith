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
    let safe_path = match config.resolve_safe_path(&input.path) {
        Some(p) => p,
        None => return Ok(CallToolResult::error(vec![Content::text("Path not allowed")])),
    };
    
    let content = fs::read_to_string(safe_path)?;
    Ok(CallToolResult::success(vec![Content::text(content)]))
}

#[derive(Deserialize, JsonSchema)]
pub struct WriteFileInput {
    pub path: String,
    pub content: String,
}

pub async fn write_file(config: &SandboxConfig, input: WriteFileInput) -> Result<CallToolResult> {
    let safe_path = match config.resolve_safe_path(&input.path) {
        Some(p) => p,
        None => return Ok(CallToolResult::error(vec![Content::text("Path not allowed")])),
    };
    
    fs::write(safe_path, &input.content)?;
    Ok(CallToolResult::success(vec![Content::text("File written successfully")]))
}

#[derive(Deserialize, JsonSchema)]
pub struct ListDirectoryInput {
    pub path: String,
}

pub async fn list_directory(config: &SandboxConfig, input: ListDirectoryInput) -> Result<CallToolResult> {
    let safe_path = match config.resolve_safe_path(&input.path) {
        Some(p) => p,
        None => return Ok(CallToolResult::error(vec![Content::text("Path not allowed")])),
    };
    
    let entries = fs::read_dir(safe_path)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    
    let content = entries.join("\n");
    Ok(CallToolResult::success(vec![Content::text(content)]))
}

#[derive(Deserialize, JsonSchema)]
pub struct CreateDirectoryInput {
    pub path: String,
}

pub async fn create_directory(config: &SandboxConfig, input: CreateDirectoryInput) -> Result<CallToolResult> {
    let safe_path = match config.resolve_safe_path(&input.path) {
        Some(p) => p,
        None => return Ok(CallToolResult::error(vec![Content::text("Path not allowed")])),
    };
    
    fs::create_dir_all(safe_path)?;
    Ok(CallToolResult::success(vec![Content::text("Directory created successfully")]))
}

#[derive(Deserialize, JsonSchema)]
pub struct DeleteFileInput {
    pub path: String,
}

pub async fn delete_file(config: &SandboxConfig, input: DeleteFileInput) -> Result<CallToolResult> {
    let safe_path = match config.resolve_safe_path(&input.path) {
        Some(p) => p,
        None => return Ok(CallToolResult::error(vec![Content::text("Path not allowed")])),
    };
    
    if !safe_path.exists() {
        return Ok(CallToolResult::error(vec![Content::text("Path does not exist")]));
    }
    
    if safe_path.is_dir() {
        fs::remove_dir_all(&safe_path)?;
        Ok(CallToolResult::success(vec![Content::text("Directory deleted successfully")]))
    } else {
        fs::remove_file(&safe_path)?;
        Ok(CallToolResult::success(vec![Content::text("File deleted successfully")]))
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct MoveFileInput {
    pub source: String,
    pub destination: String,
}

pub async fn move_file(config: &SandboxConfig, input: MoveFileInput) -> Result<CallToolResult> {
    let safe_source = match config.resolve_safe_path(&input.source) {
        Some(p) => p,
        None => return Ok(CallToolResult::error(vec![Content::text("Path not allowed")])),
    };
    let safe_destination = match config.resolve_safe_path(&input.destination) {
        Some(p) => p,
        None => return Ok(CallToolResult::error(vec![Content::text("Path not allowed")])),
    };
    
    fs::rename(safe_source, safe_destination)?;
    Ok(CallToolResult::success(vec![Content::text("File/Directory moved successfully")]))
}
