use super::{AiProvider, ChatMessage};
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use serde_json::json;
use reqwest::Client;
use std::time::Duration;

#[derive(Clone)]
pub struct LocalProvider {
    model: String,
    client: Client,
    url: String,
}

impl LocalProvider {
    pub fn new(model: String) -> Self {
        Self {
            model,
            // 120 second timeout - allows local model load times on various hardware under load
            client: Client::builder().timeout(Duration::from_secs(120)).build().unwrap(),
            url: "http://localhost:11434/api/chat".to_string(),
        }
    }
}

#[async_trait]
impl AiProvider for LocalProvider {
    async fn prompt_with_history(
        &self, 
        system_instructions: &str,
        history: &[ChatMessage],
        model: Option<&str>
    ) -> Result<String> {
        let semaphore = crate::agent::get_local_semaphore();
        let _permit = semaphore.acquire().await.map_err(|e| anyhow!("Failed to acquire local inference permit: {}", e))?;

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

        println!("         [LOCAL AI API] Dispatching prompt to local model '{}'...", model_name);
        let start_time = std::time::Instant::now();

        let request_future = async {
            let response = self.client.post(&self.url)
                .json(&payload)
                .send()
                .await
                .map_err(|e| anyhow!("Local AI connection error: {}. Is the service running?", e))?;

            if !response.status().is_success() {
                let error_text = response.text().await?;
                println!("         [LOCAL AI API] Local model error: {}", error_text);
                return Err(anyhow!("Local AI API error: {}", error_text));
            }

            let json_resp: serde_json::Value = response.json().await?;
            let text = json_resp["message"]["content"]
                .as_str()
                .ok_or_else(|| anyhow!("Failed to parse response from local AI chat"))?;
            
            Ok(text.to_string())
        };

        tokio::select! {
            res = request_future => {
                let duration = start_time.elapsed();
                println!("         [LOCAL AI API] Local model response resolved successfully (took {:.2?})", duration);
                if res.is_ok() {
                    let _ = crate::agent::DIAGNOSTICS.try_with(|d| {
                        d.borrow_mut().local_calls.push((model_name.to_string(), duration));
                    });
                }
                res
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\n🕶️: Operation aborted by operator. Purging execution pipeline.");
                Err(anyhow!("Local AI request canceled by operator"))
            }
        }
    }
}
