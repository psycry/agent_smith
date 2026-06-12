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

fn validate_no_traversal(path_str: &str) -> Result<(), CallToolResult> {
    let path = std::path::Path::new(path_str);
    if path.components().any(|c| c == std::path::Component::ParentDir) || path_str.contains("..") {
        return Err(CallToolResult::error(vec![Content::text("Path traversal detected")]));
    }
    Ok(())
}

pub async fn read_file(config: &SandboxConfig, input: ReadFileInput) -> Result<CallToolResult> {
    if let Err(e) = validate_no_traversal(&input.path) {
        return Ok(e);
    }
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
    if let Err(e) = validate_no_traversal(&input.path) {
        return Ok(e);
    }
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
    if let Err(e) = validate_no_traversal(&input.path) {
        return Ok(e);
    }
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
    if let Err(e) = validate_no_traversal(&input.path) {
        return Ok(e);
    }
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
    if let Err(e) = validate_no_traversal(&input.path) {
        return Ok(e);
    }
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

fn copy_dir_all(src: impl AsRef<std::path::Path>, dst: impl AsRef<std::path::Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub async fn move_file(config: &SandboxConfig, input: MoveFileInput) -> Result<CallToolResult> {
    if let Err(e) = validate_no_traversal(&input.source) {
        return Ok(e);
    }
    if let Err(e) = validate_no_traversal(&input.destination) {
        return Ok(e);
    }
    let safe_source = match config.resolve_safe_path(&input.source) {
        Some(p) => p,
        None => return Ok(CallToolResult::error(vec![Content::text("Path not allowed")])),
    };
    let safe_destination = match config.resolve_safe_path(&input.destination) {
        Some(p) => p,
        None => return Ok(CallToolResult::error(vec![Content::text("Path not allowed")])),
    };
    
    if let Err(_) = fs::rename(&safe_source, &safe_destination) {
        if safe_source.is_dir() {
            copy_dir_all(&safe_source, &safe_destination)?;
            fs::remove_dir_all(&safe_source)?;
        } else {
            fs::copy(&safe_source, &safe_destination)?;
            fs::remove_file(&safe_source)?;
        }
    }
    Ok(CallToolResult::success(vec![Content::text("File/Directory moved successfully")]))
}
