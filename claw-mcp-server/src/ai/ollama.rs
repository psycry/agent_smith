use super::{AiProvider, ChatMessage};
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use serde_json::json;
use reqwest::Client;
use std::time::Duration;

#[derive(Clone)]
pub struct OllamaProvider {
    model: String,
    client: Client,
    url: String,
}

impl OllamaProvider {
    pub fn new(model: String) -> Self {
        Self {
            model,
            // 30 second timeout - Ollama should be fast locally
            client: Client::builder().timeout(Duration::from_secs(30)).build().unwrap(),
            url: "http://localhost:11434/api/chat".to_string(),
        }
    }
}

#[async_trait]
impl AiProvider for OllamaProvider {
    async fn prompt_with_history(
        &self, 
        system_instructions: &str,
        history: &[ChatMessage],
        model: Option<&str>
    ) -> Result<String> {
        let model_name = model.unwrap_or(&self.model);
        
        let mut messages = Vec::new();
        messages.push(json!({ "role": "system", "content": system_instructions }));

        for msg in history {
            messages.push(json!({ "role": msg.role, "content": msg.content }));
        }

        let payload = json!({
            "model": model_name,
            "messages": messages,
            "stream": false
        });

        println!("         [OLLAMA API] Dispatching prompt to local model '{}'...", model_name);
        let start_time = std::time::Instant::now();

        let response = self.client.post(&self.url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow!("Ollama connection error: {}. Is the service running?", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            println!("         [OLLAMA API] Local model error: {}", error_text);
            return Err(anyhow!("Ollama API error: {}", error_text));
        }

        let json_resp: serde_json::Value = response.json().await?;
        let text = json_resp["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow!("Failed to parse response from Ollama chat"))?;

        let duration = start_time.elapsed();
        println!("         [OLLAMA API] Local model response resolved successfully (took {:.2?})", duration);

        Ok(text.to_string())
    }
}
