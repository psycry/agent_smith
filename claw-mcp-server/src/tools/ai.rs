use crate::config::SandboxConfig;
use crate::ai::{AiProvider, cloud::CloudProvider, local::LocalProvider, ChatMessage};
use anyhow::Result;
use serde::Deserialize;
use schemars::JsonSchema;
use rmcp::model::{CallToolResult, Content};

#[derive(Deserialize, JsonSchema)]
pub struct AskAiInput {
    pub prompt: String,
    pub provider: String,
    pub model: Option<String>,
}

pub async fn ask_ai(config: &SandboxConfig, input: AskAiInput) -> Result<CallToolResult> {
    let ai_config = config.get_ai_config(&input.provider)
        .ok_or_else(|| anyhow::anyhow!("AI Provider '{}' not configured", input.provider))?;

    let provider: Box<dyn AiProvider> = match input.provider.as_str() {
        "gemini" => Box::new(CloudProvider::new(ai_config.api_key.clone(), ai_config.default_model.clone())),
        "ollama" => Box::new(LocalProvider::new(ai_config.default_model.clone(), ai_config.base_url.clone())),
        _ => return Err(anyhow::anyhow!("Unknown AI Provider")),
    };

    // For a one-off tool call, history is empty
    let response = provider.prompt_with_history(
        "You are a helpful assistant.",
        &[ChatMessage { role: "user".to_string(), content: input.prompt }], 
        input.model.as_deref()
    ).await?;

    Ok(CallToolResult::success(vec![Content::text(response)]))
}
