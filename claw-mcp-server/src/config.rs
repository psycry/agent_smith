use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AiConfig {
    pub api_key: String,
    pub default_model: String,
    pub base_url: Option<String>,
    pub fallback_models: Option<Vec<String>>,
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

    pub fn resolve_safe_path(&self, raw_path: &str) -> Option<std::path::PathBuf> {
        let path = Path::new(raw_path);
        
        // Find the closest existing ancestor
        let mut ancestor = path.to_path_buf();
        let mut sub_path = std::path::PathBuf::new();
        
        while !ancestor.exists() {
            if let Some(parent) = ancestor.parent() {
                if let Some(file_name) = ancestor.file_name() {
                    let mut new_sub = std::path::PathBuf::from(file_name);
                    new_sub.push(sub_path);
                    sub_path = new_sub;
                }
                ancestor = parent.to_path_buf();
            } else {
                break;
            }
        }
        
        // Canonicalize the existing ancestor
        let canonical_input = if let Ok(canonical_ancestor) = fs::canonicalize(&ancestor) {
            let mut resolved = canonical_ancestor;
            resolved.push(sub_path);
            resolved
        } else {
            return None;
        };

        // Check if starts_with any allowed path
        let is_allowed = self.allowed_paths.iter().any(|allowed| {
            if let Ok(canonical_allowed) = fs::canonicalize(Path::new(allowed)) {
                canonical_input.starts_with(&canonical_allowed)
            } else {
                false
            }
        });

        if is_allowed {
            Some(canonical_input)
        } else {
            None
        }
    }

    pub fn is_path_allowed(&self, path: &str) -> bool {
        self.resolve_safe_path(path).is_some()
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
