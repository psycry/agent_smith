use rmcp::{handler::server::wrapper::Parameters, transport::stdio, ServiceExt, tool, tool_router, model::{CallToolResult, Content}};
use std::sync::Arc;
use tokio;

use claw_mcp_server::config::SandboxConfig;
use claw_mcp_server::tools::{file_system, shell, metrics, ai as ai_tool, search};

#[derive(Debug, Clone)]
pub struct ClawServer {
    config: Arc<SandboxConfig>,
}

#[tool_router(server_handler)]
impl ClawServer {
    #[tool(description = "Read the contents of a file from the allowed paths.")]
    async fn read_file(&self, Parameters(input): Parameters<file_system::ReadFileInput>) -> CallToolResult {
        if !self.config.is_tool_whitelisted("read_file") {
            return CallToolResult::error(vec![Content::text("Tool 'read_file' is not whitelisted.")]);
        }
        match file_system::read_file(&self.config, input).await {
            Ok(res) => res,
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {}", e))]),
        }
    }

    #[tool(description = "Write content to a file in the allowed paths.")]
    async fn write_file(&self, Parameters(input): Parameters<file_system::WriteFileInput>) -> CallToolResult {
        if !self.config.is_tool_whitelisted("write_file") {
            return CallToolResult::error(vec![Content::text("Tool 'write_file' is not whitelisted.")]);
        }
        match file_system::write_file(&self.config, input).await {
            Ok(res) => res,
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {}", e))]),
        }
    }

    #[tool(description = "List files in a directory within the allowed paths.")]
    async fn list_directory(&self, Parameters(input): Parameters<file_system::ListDirectoryInput>) -> CallToolResult {
        if !self.config.is_tool_whitelisted("list_directory") {
            return CallToolResult::error(vec![Content::text("Tool 'list_directory' is not whitelisted.")]);
        }
        match file_system::list_directory(&self.config, input).await {
            Ok(res) => res,
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {}", e))]),
        }
    }

    #[tool(description = "Execute a shell command from the allowed commands list.")]
    async fn execute_command(&self, Parameters(input): Parameters<shell::ExecuteCommandInput>) -> CallToolResult {
        if !self.config.is_tool_whitelisted("execute_command") {
            return CallToolResult::error(vec![Content::text("Tool 'execute_command' is not whitelisted.")]);
        }
        match shell::execute_command(&self.config, input).await {
            Ok(res) => res,
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {}", e))]),
        }
    }

    #[tool(description = "Get system metrics like CPU and Memory usage.")]
    async fn get_system_stats(&self, _params: Parameters<serde_json::Value>) -> CallToolResult {
        if !self.config.is_tool_whitelisted("get_system_stats") {
            return CallToolResult::error(vec![Content::text("Tool 'get_system_stats' is not whitelisted.")]);
        }
        match metrics::get_system_stats().await {
            Ok(res) => res,
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {}", e))]),
        }
    }

    #[tool(description = "Ask an AI model a question.")]
    async fn ask_ai(&self, Parameters(input): Parameters<ai_tool::AskAiInput>) -> CallToolResult {
        if !self.config.is_tool_whitelisted("ask_ai") {
            return CallToolResult::error(vec![Content::text("Tool 'ask_ai' is not whitelisted.")]);
        }
        match ai_tool::ask_ai(&self.config, input).await {
            Ok(res) => res,
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {}", e))]),
        }
    }

    #[tool(description = "Search the web for information using DuckDuckGo (Free).")]
    async fn search_web(&self, Parameters(input): Parameters<search::SearchWebInput>) -> CallToolResult {
        if !self.config.is_tool_whitelisted("search_web") {
            return CallToolResult::error(vec![Content::text("Tool 'search_web' is not whitelisted.")]);
        }
        match search::search_web(&self.config, input).await {
            Ok(res) => res,
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {}", e))]),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = SandboxConfig::load()?;
    let server = ClawServer {
        config: Arc::new(config),
    };

    eprintln!("Claw MCP Server starting on stdio...");
    let service = server.serve(stdio()).await?;
    
    service.waiting().await?;
    Ok(())
}
