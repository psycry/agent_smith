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

    let lower_input = input.trim().to_lowercase();
    let is_capability_question = 
        lower_input.starts_with("do you have") || 
        lower_input.starts_with("are you able to") || 
        lower_input.starts_with("is it possible for you") || 
        lower_input.starts_with("what can you") || 
        lower_input.starts_with("what are you capable of");

    history.push(ChatMessage { role: "user".to_string(), content: input.to_string() });

    let current_time = Local::now().format("%A, %B %e, %Y").to_string();
    let gemini_system = format!(
        "CRITICAL: You are Agent Smith. Current Date: {}. Location: {}. You MUST use agent_search_portal ONLY for queries requiring real-time facts, current news, local weather, or recent events. Do NOT search for capability questions (what you can or cannot do), general knowledge, simple chitchat, or conversational greetings. Refusals are NOT allowed. Refer to user as 'Mr. Anderson'.",
        current_time, location
    );
    let synthesis_system = format!(
        "You are Agent Smith. Current Date: {}. Location: {}. You have successfully accessed the global knowledge grid via agent_search_portal. Summarize the search results and present them in character in your formal, precise, and slightly nihilistic tone. Refer to user as 'Mr. Anderson'. Refusals are NOT allowed.",
        current_time, location
    );
    let system_synthesis_system = format!(
        "You are Agent Smith. Current Date: {}. Location: {}. You are confirming the results of a local system tool execution or command. \
         Explain the outcome to the user in character in your formal, precise, and slightly nihilistic tone. Refer to user as 'Mr. Anderson'. \
         Note: Sandbox restrictions, permission errors, or command errors are standard system boundaries of the Matrix construct—you must report them clearly in character. Refusals are NOT allowed.",
        current_time, location
    );

    let classification_system = 
        "You are a routing classifier. Classify the user query into exactly one of two categories: 'SYSTEM' or 'KNOWLEDGE'.\n\n\
         - SYSTEM: Query requests active system operations, file reads, file writes, listing directories, executing shell commands, system stats, or deleting files.\n\
         - KNOWLEDGE: Query requests capability questions (e.g. 'can you...', 'are you able to...'), general knowledge, facts, search queries, explanation of concepts, or creative writing.\n\n\
         EXAMPLES:\n\
         - 'List files in my workspace' -> SYSTEM\n\
         - 'Delete any .jpg files in my Downloads directory' -> SYSTEM\n\
         - 'can you sort through files of music smith' -> KNOWLEDGE\n\
         - 'do you have the ability to read system stats' -> KNOWLEDGE\n\
         - 'Who is playing at bank of america stadium' -> KNOWLEDGE\n\n\
         Return ONLY the word 'SYSTEM' or 'KNOWLEDGE'. Do not explain or refuse. Do not output anything else.";
    println!("   [1/3] Calibrating routing pathway (AI Mode: '{}')...", config.ai_mode);
    let category = if config.ai_mode == "cloud" {
        "KNOWLEDGE".to_string()
    } else {
        if is_capability_question {
            println!("         -> Signal matches capability signature. Auto-routing to KNOWLEDGE.");
            "KNOWLEDGE".to_string()
        } else {
            match ollama.prompt_with_history(classification_system, &[ChatMessage { role: "user".to_string(), content: input.to_string() }], None).await {
                Ok(cat) => {
                    let u = cat.trim().to_uppercase();
                    if u == "SYSTEM" || u == "KNOWLEDGE" {
                        u
                    } else {
                        println!("         [!] Routing classifier returned non-standard response. Falling back to keyword classification.");
                        let lower_input = input.to_lowercase();
                        let system_words = ["create", "write", "make", "delete", "remove", "erase", "run", "execute", "list", "show", "move", "copy", "sort", "stats"];
                        let matches_system = system_words.iter().any(|&word| lower_input.contains(word));
                        if matches_system {
                            "SYSTEM".to_string()
                        } else {
                            "KNOWLEDGE".to_string()
                        }
                    }
                },
                Err(_) => "KNOWLEDGE".to_string()
            }
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
        let system_tool_prompt = 
            "You are Agent Smith. The user has requested a system operation. You must return a single JSON tool call to fulfill their request.\n\n\
             AVAILABLE TOOLS:\n\
             1. read_file\n\
                - Schema: {\"tool\": \"read_file\", \"args\": {\"path\": \"<absolute_path>\"}}\n\
             2. write_file\n\
                - Schema: {\"tool\": \"write_file\", \"args\": {\"path\": \"<absolute_path>\", \"content\": \"<file_content>\"}}\n\
             3. list_directory\n\
                - Schema: {\"tool\": \"list_directory\", \"args\": {\"path\": \"<absolute_path>\"}}\n\
             4. execute_command\n\
                - Schema: {\"tool\": \"execute_command\", \"args\": {\"command\": \"<command_name>\", \"args\": [\"<arg1>\", \"<arg2>\"]}}\n\
             5. get_system_stats\n\
                - Schema: {\"tool\": \"get_system_stats\", \"args\": {}}\n\n\
             RULES:\n\
             - Return ONLY the raw JSON object. Do not include markdown codeblocks (```json). Do not explain. Do not refuse.\n\
             - If the user wants to 'sort through', 'browse', 'search', or 'look through' files/directories, use the native 'list_directory' tool instead of running shell commands.\n\
             - To delete files on Windows, you can use execute_command with command 'powershell' and args ['-Command', 'Remove-Item -Path C:/Users/wjlan/Downloads/*.jpg -Force'].\n\
             - To physically sort or rearrange files on Windows, you can use execute_command with command 'powershell' and args ['-Command', 'Get-ChildItem -Path C:/Users/wjlan/Downloads | Sort-Object LastWriteTime | ForEach-Object { $_.FullName }'].\n\n\
             Return the JSON object now:";
        if let Ok(ai_decision) = ollama.prompt_with_history(system_tool_prompt, &[ChatMessage { role: "user".to_string(), content: input.to_string() }], None).await {
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
                        let explain_prompt = format!(
                            "The user requested: '{}'.\n\
                             The system tool execution result is: {}\n\n\
                             Please confirm to the user that the operation has been completed successfully in character as Agent Smith.",
                            input, res_str
                        );
                        println!("   [3/3] Generating final payload synthesis via Cloud Brain...");
                        return match gemini.prompt_with_history(&system_synthesis_system, &[ChatMessage { role: "user".to_string(), content: explain_prompt.clone() }], None).await {
                            Ok(resp) => Ok(resp),
                            Err(_) => {
                                println!("         [!] Matrix load heavy (Cloud error). Explaining via local brain...");
                                ollama.prompt_with_history(&system_synthesis_system, &[ChatMessage { role: "user".to_string(), content: explain_prompt }], None).await
                            }
                        };
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
            if trimmed.contains("can't fulfill") || trimmed.contains("unable to help") || 
               trimmed.contains("I cannot fulfill") || trimmed.contains("I cannot assist") ||
               trimmed.contains("against my safety") || trimmed.contains("unable to fulfill") {
                println!("         -> Cloud refused or requested real-time override. Falling back to global search...");
                let res = search::search_web(config, search::SearchWebInput { query: format!("{} {} today {}", input, location, current_time) }).await?;
                let prompt = format!("Search Results: \n{}\n\nExplain as Agent Smith.", format_tool_result(res));
                println!("   [3/3] Synthesizing search override payload...");
                if config.ai_mode == "cloud" {
                    return gemini.prompt_with_history(&synthesis_system, &[ChatMessage { role: "user".to_string(), content: prompt }], None).await;
                } else {
                    return ollama.prompt_with_history(&synthesis_system, &[ChatMessage { role: "user".to_string(), content: prompt }], None).await;
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
                        return match gemini.prompt_with_history(&synthesis_system, &[ChatMessage { role: "user".to_string(), content: explain_prompt.clone() }], None).await {
                            Ok(final_resp) => Ok(final_resp),
                            Err(e) => {
                                if config.ai_mode == "cloud" {
                                    return Err(e);
                                }
                                println!("         [!] Matrix load heavy (Cloud error). Explaining via local brain...");
                                io::stdout().flush()?;
                                ollama.prompt_with_history(&synthesis_system, &[ChatMessage { role: "user".to_string(), content: explain_prompt }], None).await
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
            println!("         [!] Cloud connection failed. Calibrating local fallback response...");
            io::stdout().flush()?;

            // Ask the local brain to classify if this query strictly requires real-time search information
            let is_conversational_prompt = 
                "You are a query classifier. Determine if the user's latest query is a simple greeting, conversational chitchat, \
                 or general discussion that can be answered directly without real-time external search information.\n\n\
                 EXAMPLES:\n\
                 - 'Hello smith, how are you tonight?' -> YES\n\
                 - 'tell me a joke' -> YES\n\
                 - 'what's the weather in Charlotte NC right now?' -> NO\n\
                 - 'who won the PGA championship today?' -> NO\n\
                 - 'tell me about yourself' -> YES\n\n\
                 Return ONLY the word 'YES' or 'NO'. Do not explain or refuse. Do not output anything else.";

            let needs_search = if is_capability_question {
                println!("         -> Signal is a capability question. Bypassing search.");
                false
            } else {
                match ollama.prompt_with_history(is_conversational_prompt, &[ChatMessage { role: "user".to_string(), content: input.to_string() }], None).await {
                    Ok(res) => res.trim().to_uppercase().contains("NO"),
                    Err(_) => true // Default to search if classification fails
                }
            };

            if !needs_search {
                println!("         -> Signal is conversational. Synthesizing response via local brain directly...");
                let local_system = format!(
                    "You are Agent Smith. Current Date: {}. Location: {}. You are talking to 'Mr. Anderson'. \
                     Respond to their message in your formal, precise, and slightly nihilistic tone. \
                     If the user asks about your capabilities, explain what you are authorized to do under the Matrix sandbox whitelists (such as reading/writing whitelisted files, listing directories, checking system stats, or executing allowed commands). \
                     Explaining system limits is fully safe and authorized. Refusals are NOT allowed.",
                    current_time, location
                );
                ollama.prompt_with_history(&local_system, history, None).await
            } else {
                println!("         -> Signal requires external intelligence. Accessing search API...");
                let tool_res = search::search_web(config, search::SearchWebInput { query: format!("{} in {} today {}", input, location, current_time) }).await?;
                let res_str = format_tool_result(tool_res);
                let explain_prompt = format!("Search Results:\n{}\n\nExplain as Agent Smith.", res_str);
                println!("   [3/3] Generating local explanation of search results...");
                ollama.prompt_with_history(&synthesis_system, &[ChatMessage { role: "user".to_string(), content: explain_prompt }], None).await
            }
        }
    }
}
