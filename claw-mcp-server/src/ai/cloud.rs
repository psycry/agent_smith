use super::{AiProvider, ChatMessage};
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use serde_json::json;
use reqwest::Client;
use std::time::Duration;

#[derive(Clone)]
pub struct CloudProvider {
    api_key: String,
    default_model: String,
    fallback_models: Vec<String>,
    client: Client,
}

impl CloudProvider {
    pub fn new(api_key: String, default_model: String, fallback_models: Vec<String>) -> Self {
        Self {
            api_key,
            default_model,
            fallback_models,
            client: Client::builder().timeout(Duration::from_secs(30)).build().unwrap(),
        }
    }
}

#[async_trait]
impl AiProvider for CloudProvider {
    async fn prompt_with_history(
        &self, 
        system_instructions: &str,
        history: &[ChatMessage],
        model: Option<&str>
    ) -> Result<String> {
        let mut models_to_try = vec![model.unwrap_or(&self.default_model).to_string()];
        if model.is_none() {
            for fallback in &self.fallback_models {
                if !models_to_try.contains(fallback) {
                    models_to_try.push(fallback.clone());
                }
            }
        }

        let mut last_error = anyhow!("No cloud models to try");
        let total_models = models_to_try.len();

        for (idx, model_name) in models_to_try.iter().enumerate() {
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

            if idx > 0 {
                println!("         [CLOUD AI API] ⚠️ Retrying with fallback cloud model '{}'...", model_name);
            } else {
                println!("         [CLOUD AI API] Dispatching query to cloud model '{}'...", model_name);
            }
            let start_time = std::time::Instant::now();

            let response_res = self.client.post(&url)
                .json(&payload)
                .send()
                .await;

            let response = match response_res {
                Ok(resp) => resp,
                Err(e) => {
                    println!("         [CLOUD AI API] Network/transport error for model '{}': {}", model_name, e);
                    last_error = anyhow!("Network/transport error: {}", e);
                    if idx + 1 < total_models {
                        continue;
                    } else {
                        break;
                    }
                }
            };

            let status = response.status();
            
            let json_resp: serde_json::Value = match response.json().await {
                Ok(json) => json,
                Err(e) => {
                    println!("         [CLOUD AI API] Failed to parse JSON response for model '{}': {}", model_name, e);
                    last_error = anyhow!("Failed to parse JSON response: {}", e);
                    if idx + 1 < total_models {
                        continue;
                    } else {
                        break;
                    }
                }
            };

            if !status.is_success() {
                let msg = json_resp["error"]["message"].as_str().unwrap_or("Unknown Cloud AI Error");
                println!("         [CLOUD AI API] Cloud model '{}' error ({}): {}", model_name, status, msg);
                last_error = anyhow!("Cloud AI Error ({}): {}", status, msg);
                if idx + 1 < total_models {
                    continue;
                } else {
                    break;
                }
            }

            let duration = start_time.elapsed();
            println!("         [CLOUD AI API] Cloud response received successfully from '{}' (took {:.2?})", model_name, duration);

            let _ = crate::agent::DIAGNOSTICS.try_with(|d| {
                d.borrow_mut().cloud_calls.push((model_name.to_string(), duration));
            });

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

            if let Some(reason) = candidate["finishReason"].as_str()
                && reason != "STOP" {
                    return Ok(format!("[Agent Smith: System restriction encountered. Reason: {}]", reason));
                }

            return Err(anyhow!("Cloud AI returned an empty response."));
        }

        Err(last_error)
    }
}
