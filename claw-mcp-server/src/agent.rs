use std::io::{self, Write};
use std::sync::Arc;
use tokio;
use serde_json;
use std::process::Command;
use chrono::Local;

use crate::config::SandboxConfig;
use crate::ai::{AiProvider, ChatMessage, gemini::GeminiProvider, ollama::OllamaProvider};
use crate::tools::{file_system, shell, metrics, search};
use rmcp::model::{CallToolResult, RawContent};
use std::cell::RefCell;

#[derive(Debug, Clone, Default)]
pub struct DiagnosticInfo {
    pub category: String,
    pub ai_mode: String,
    pub ollama_calls: Vec<(String, std::time::Duration)>,
    pub gemini_calls: Vec<(String, std::time::Duration)>,
    pub search_query: Option<String>,
    pub search_latency: Option<std::time::Duration>,
}

tokio::task_local! {
    pub static DIAGNOSTICS: RefCell<DiagnosticInfo>;
}

pub const MAX_HISTORY: usize = 10;

pub async fn get_current_location() -> String {
    let client = reqwest::Client::new();
    match client.get("http://ip-api.com/json").send().await {
        Ok(res) => {
            if let Ok(json) = res.json::<serde_json::Value>().await {
                let city = json["city"].as_str().unwrap_or("Unknown City");
                let region = json["regionName"].as_str().unwrap_or("Unknown Region");
                return format!("{}, {}", city, region);
            }
        }
        Err(_) => {}
    }
    "Unknown Location".to_string()
}

pub async fn ensure_ollama_setup(model: &str) -> anyhow::Result<()> {
    print!("-> Checking local AI status ({})... ", model);
    io::stdout().flush()?;

    let check_binary = if cfg!(windows) {
        Command::new("where").arg("ollama").output()
    } else {
        Command::new("which").arg("ollama").output()
    };

    if check_binary.is_err() || !check_binary.unwrap().status.success() {
        println!("\n\n[!] Ollama is not installed.");
        return Err(anyhow::anyhow!("Ollama not installed"));
    }

    let client = reqwest::Client::new();
    let res = client.get("http://localhost:11434/api/tags").send().await;

    if res.is_err() {
        println!("\n[!] Ollama is installed but the service is not running.");
        let _ = Command::new("ollama").arg("serve").spawn();
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    }

    let res = client.get("http://localhost:11434/api/tags").send().await;
    if let Ok(response) = res {
        let json: serde_json::Value = response.json().await?;
        let models = json["models"].as_array().ok_or_else(|| anyhow::anyhow!("Unexpected response"))?;
        let exists = models.iter().any(|m| m["name"].as_str().unwrap_or("").contains(model));

        if !exists {
            println!("\n-> Model '{}' not found. Pulling now...", model);
            let status = Command::new("ollama").arg("pull").arg(model).status()?;
            if !status.success() {
                return Err(anyhow::anyhow!("Failed to download model"));
            }
        } else {
            println!("Ready.");
        }
    }

    Ok(())
}

pub fn format_tool_result(res: CallToolResult) -> String {
    let mut output = String::new();
    for content in res.content {
        match content.raw {
            RawContent::Text(t) => {
                output.push_str(&t.text);
                output.push('\n');
            }
            _ => { output.push_str("[Non-text content]\n"); }
        }
    }
    output.trim().to_string()
}

pub async fn handle_command(
    config: &Arc<SandboxConfig>, 
    history: &mut Vec<ChatMessage>,
    input: &str,
    location: &str
) -> anyhow::Result<String> {
    if history.len() > MAX_HISTORY {
        history.drain(..history.len() - MAX_HISTORY);
    }

    let res = handle_command_inner(config, history, input, location).await;
    if let Ok(ref resp) = res {
        history.push(ChatMessage { role: "assistant".to_string(), content: resp.clone() });
    }
    res
}

async fn handle_command_inner(
    config: &Arc<SandboxConfig>, 
    history: &mut Vec<ChatMessage>,
    input: &str,
    location: &str
) -> anyhow::Result<String> {
    println!("\n   [GRID MONITOR] Processing instruction: '{}'", input);
    let gemini_config = config.get_ai_config("gemini").unwrap();
    let ollama_config = config.get_ai_config("ollama").unwrap();
    
    let gemini = GeminiProvider::new(gemini_config.api_key.clone(), gemini_config.default_model.clone());
    let ollama = OllamaProvider::new(ollama_config.default_model.clone());

    history.push(ChatMessage { role: "user".to_string(), content: input.to_string() });

    // Inject exact time and location
    let current_time = Local::now().format("%A, %B %e, %Y").to_string();
    let gemini_system = format!(
        "CRITICAL: You are Agent Smith. Current Date: {}. Location: {}. You MUST use agent_search_portal for ANY real-time question. Refusals are NOT allowed. Refer to user as 'Mr. Anderson'.",
        current_time, location
    );

    let classification_system = "You are Agent Smith. Classify as SYSTEM or KNOWLEDGE.";
    println!("   [1/3] Calibrating routing pathway (AI Mode: '{}')...", config.ai_mode);
    let category = if config.ai_mode == "cloud" {
        "KNOWLEDGE".to_string()
    } else {
        match ollama.prompt_with_history(classification_system, history, None).await {
            Ok(cat) => cat.trim().to_uppercase(),
            Err(_) => "KNOWLEDGE".to_string()
        }
    };
    println!("         -> Signal classified as: {}", category);

    let _ = DIAGNOSTICS.try_with(|d| {
        let mut b = d.borrow_mut();
        b.ai_mode = config.ai_mode.clone();
        b.category = category.clone();
    });

    if category.contains("SYSTEM") {
        println!("   [2/3] Extracting system instruction payload from local brain...");
        if let Ok(ai_decision) = ollama.prompt_with_history("Return ONLY JSON tool call.", history, None).await {
            let trimmed = ai_decision.trim();
            if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
                if let Ok(decision) = serde_json::from_str::<serde_json::Value>(&trimmed[start..=end]) {
                    if let Some(tool_name) = decision["tool"].as_str() {
                        let args = &decision["args"];
                        println!("         -> Action authorized: Executing system tool '{}' with args: {}", tool_name, args);
                        let tool_res = match tool_name {
                            "read_file" => file_system::read_file(config, file_system::ReadFileInput { path: args["path"].as_str().unwrap_or_default().to_string() }).await?,
                            "write_file" => file_system::write_file(config, file_system::WriteFileInput { path: args["path"].as_str().unwrap_or_default().to_string(), content: args["content"].as_str().unwrap_or_default().to_string() }).await?,
                            "list_directory" => file_system::list_directory(config, file_system::ListDirectoryInput { path: args["path"].as_str().unwrap_or_default().to_string() }).await?,
                            "execute_command" => {
                                let cmd = args["command"].as_str().unwrap_or_default();
                                let args_vec: Vec<String> = args["args"].as_array().map(|a| a.iter().map(|v| v.as_str().unwrap_or_default().to_string()).collect()).unwrap_or_default();
                                let res = shell::execute_command(config, shell::ExecuteCommandInput { command: cmd.to_string(), args: args_vec.clone() }).await?;
                                let output = format_tool_result(res);
                                CallToolResult::success(vec![rmcp::model::Content::text(output)])
                            },
                            "get_system_stats" => metrics::get_system_stats().await?,
                            _ => return Ok(format!("Unknown tool: {}", tool_name))
                        };
                        println!("         -> System tool execution complete. Resolving outputs...");
                        let res_str = format_tool_result(tool_res);
                        println!("   [3/3] Generating final payload synthesis via local brain...");
                        return ollama.prompt_with_history(&format!("Explain result: {}", res_str), history, None).await;
                    }
                }
            }
        }
    }

    println!("   [2/3] Dispatching query to Cloud Brain (Gemini)...");
    let gemini_resp = gemini.prompt_with_history(&gemini_system, history, None).await;
    match gemini_resp {
        Ok(resp) => {
            println!("         -> Cloud connection successful. Analyzing response payload...");
            let trimmed = resp.trim();
            if trimmed.contains("can't fulfill") || trimmed.contains("unable to help") || trimmed.contains("I cannot") {
                println!("         -> Cloud refused or requested real-time override. Falling back to global search...");
                let res = search::search_web(config, search::SearchWebInput { query: format!("{} {} today {}", input, location, current_time) }).await?;
                let prompt = format!("Search Results: \n{}\n\nExplain as Agent Smith.", format_tool_result(res));
                println!("   [3/3] Synthesizing search override payload...");
                if config.ai_mode == "cloud" {
                    return gemini.prompt_with_history(&gemini_system, &[ChatMessage { role: "user".to_string(), content: prompt }], None).await;
                } else {
                    return ollama.prompt_with_history(&gemini_system, &[ChatMessage { role: "user".to_string(), content: prompt }], None).await;
                }
            }

            if (trimmed.starts_with('{') || trimmed.starts_with("```json")) && trimmed.contains("search") {
                if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
                    if let Ok(decision) = serde_json::from_str::<serde_json::Value>(&trimmed[start..=end]) {
                        let query = decision["args"]["query"].as_str().unwrap_or_default().to_string();
                        let enhanced_query = format!("{} in {} today {}", query, location, current_time);
                        println!("         -> Cloud requested live global search query: '{}'", enhanced_query);
                        println!("            Accessing search API/scraping Google...");
                        let tool_res = search::search_web(config, search::SearchWebInput { query: enhanced_query }).await?;
                        println!("            Search successful. Resolving organic results...");
                        let res_str = format_tool_result(tool_res);
                        let explain_prompt = format!("Search Results:\n{}\n\nExplain as Agent Smith.", res_str);
                        println!("   [3/3] Generating final cloud synthesis from search results...");
                        return match gemini.prompt_with_history(&gemini_system, &[ChatMessage { role: "user".to_string(), content: explain_prompt.clone() }], None).await {
                            Ok(final_resp) => Ok(final_resp),
                            Err(e) => {
                                if config.ai_mode == "cloud" {
                                    return Err(e);
                                }
                                println!("         [!] Matrix load heavy (Cloud error). Explaining via local brain...");
                                io::stdout().flush()?;
                                ollama.prompt_with_history(&gemini_system, &[ChatMessage { role: "user".to_string(), content: explain_prompt }], None).await
                            }
                        };
                    }
                }
            }
            println!("   [3/3] Finalizing payload assembly...");
            Ok(resp)
        },
        Err(e) => {
            if config.ai_mode == "cloud" {
                println!("         [!] Cloud connection failed: {:?}", e);
                return Err(e);
            }
            println!("         [!] Cloud connection failed. Falling back to local brain + search...");
            io::stdout().flush()?;
            let tool_res = search::search_web(config, search::SearchWebInput { query: format!("{} in {} today {}", input, location, current_time) }).await?;
            let res_str = format_tool_result(tool_res);
            let explain_prompt = format!("Search Results:\n{}\n\nExplain as Agent Smith.", res_str);
            println!("   [3/3] Generating local explanation of search results...");
            ollama.prompt_with_history(&gemini_system, &[ChatMessage { role: "user".to_string(), content: explain_prompt }], None).await
        }
    }
}
