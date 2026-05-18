use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AiConfig {
    pub api_key: String,
    pub default_model: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SandboxConfig {
    pub allowed_paths: Vec<String>,
    pub allowed_commands: Vec<String>,
    pub whitelisted_tools: Vec<String>,
    pub ai_mode: String, // "hybrid" or "cloud"
    pub ai_providers: HashMap<String, AiConfig>,
    pub discord_token: Option<String>,
    pub discord_whitelist: Option<Vec<u64>>, // Discord User IDs
}

impl SandboxConfig {
    pub fn load() -> Result<Self> {
        let config_path = Path::new("sandbox_config.json");
        let config_str = if config_path.exists() {
            fs::read_to_string(config_path)?
        } else {
            let fallback_path = Path::new("claw-mcp-server/sandbox_config.json");
            if fallback_path.exists() {
                fs::read_to_string(fallback_path)?
            } else {
                return Err(anyhow::anyhow!("sandbox_config.json not found in current directory or claw-mcp-server/"));
            }
        };
        let config: SandboxConfig = serde_json::from_str(&config_str)?;
        Ok(config)
    }

    pub fn is_path_allowed(&self, path: &str) -> bool {
        let path = path.replace("\\", "/").to_lowercase();
        self.allowed_paths.iter().any(|p| {
            let p = p.replace("\\", "/").to_lowercase();
            path.starts_with(&p)
        })
    }

    pub fn is_command_allowed(&self, command: &str) -> bool {
        self.allowed_commands.contains(&command.to_string())
    }

    pub fn is_tool_whitelisted(&self, tool: &str) -> bool {
        self.whitelisted_tools.contains(&tool.to_string())
    }

    pub fn get_ai_config(&self, provider: &str) -> Option<&AiConfig> {
        self.ai_providers.get(provider)
    }
}
