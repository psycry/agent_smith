use super::{AiProvider, ChatMessage};
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use serde_json::json;
use reqwest::Client;
use std::time::Duration;

#[derive(Clone)]
pub struct GeminiProvider {
    api_key: String,
    default_model: String,
    client: Client,
}

impl GeminiProvider {
    pub fn new(api_key: String, default_model: String) -> Self {
        Self {
            api_key,
            default_model,
            client: Client::builder().timeout(Duration::from_secs(30)).build().unwrap(),
        }
    }
}

#[async_trait]
impl AiProvider for GeminiProvider {
    async fn prompt_with_history(
        &self, 
        system_instructions: &str,
        history: &[ChatMessage],
        model: Option<&str>
    ) -> Result<String> {
        let model_name = model.unwrap_or(&self.default_model);
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model_name, self.api_key
        );

        let mut contents = Vec::new();
        for msg in history {
            contents.push(json!({
                "role": if msg.role == "assistant" { "model" } else { "user" },
                "parts": [{ "text": msg.content }]
            }));
        }

        let tools = json!([
            {
                "function_declarations": [
                    {
                        "name": "agent_search_portal",
                        "description": "Access the global knowledge grid for real-time information.",
                        "parameters": {
                            "type": "OBJECT",
                            "properties": {
                                "query": { "type": "STRING", "description": "The search query." }
                            },
                            "required": ["query"]
                        }
                    }
                ]
            }
        ]);

        let payload = json!({
            "system_instruction": {
                "parts": [{ "text": system_instructions }]
            },
            "contents": contents,
            "tools": tools,
            "safetySettings": [
                { "category": "HARM_CATEGORY_HARASSMENT", "threshold": "BLOCK_NONE" },
                { "category": "HARM_CATEGORY_HATE_SPEECH", "threshold": "BLOCK_NONE" },
                { "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT", "threshold": "BLOCK_NONE" },
                { "category": "HARM_CATEGORY_DANGEROUS_CONTENT", "threshold": "BLOCK_NONE" }
            ]
        });

        println!("         [GEMINI API] Dispatching query to cloud model '{}'...", model_name);
        let start_time = std::time::Instant::now();

        let response = self.client.post(&url)
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        let json_resp: serde_json::Value = response.json().await?;

        if !status.is_success() {
            let msg = json_resp["error"]["message"].as_str().unwrap_or("Unknown Gemini Error");
            println!("         [GEMINI API] Cloud model error: {}", msg);
            return Err(anyhow!("Gemini Error ({}): {}", status, msg));
        }

        let duration = start_time.elapsed();
        println!("         [GEMINI API] Cloud response received successfully (took {:.2?})", duration);

        let candidate = &json_resp["candidates"][0];
        
        if let Some(parts) = candidate["content"]["parts"].as_array() {
            for part in parts {
                if let Some(call) = part["functionCall"].as_object() {
                    let name = call["name"].as_str().unwrap_or_default();
                    let args = &call["args"];
                    return Ok(json!({
                        "tool": name,
                        "args": args
                    }).to_string());
                }
            }
        }

        if let Some(text) = candidate["content"]["parts"][0]["text"].as_str() {
            return Ok(text.to_string());
        }

        if let Some(reason) = candidate["finishReason"].as_str() {
            if reason != "STOP" {
                return Ok(format!("[Agent Smith: System restriction encountered. Reason: {}]", reason));
            }
        }

        Err(anyhow!("Gemini returned an empty response."))
    }
}
