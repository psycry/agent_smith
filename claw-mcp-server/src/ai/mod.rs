use async_trait::async_trait;
use anyhow::Result;
use serde::{Serialize, Deserialize};

pub mod cloud;
pub mod local;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "user" or "model"/"assistant"
    pub content: String,
}

#[async_trait]
pub trait AiProvider: Send + Sync {
    async fn prompt_with_history(
        &self, 
        system_instructions: &str,
        history: &[ChatMessage],
        model: Option<&str>
    ) -> Result<String>;
}
