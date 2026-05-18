use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

use claw_mcp_server::config::SandboxConfig;
use claw_mcp_server::agent::{handle_command, ensure_ollama_setup};
use claw_mcp_server::ai::ChatMessage;

async fn send_long_message(ctx: &Context, msg: &Message, content: &str) -> serenity::Result<()> {
    if content.chars().count() <= 1900 {
        let trimmed = content.trim();
        if !trimmed.is_empty() {
            msg.reply(&ctx.http, trimmed).await?;
        }
        return Ok(());
    }

    let mut remaining = content.to_string();
    let mut is_first = true;

    while !remaining.is_empty() {
        // Yield to prevent thread starvation
        tokio::task::yield_now().await;

        let char_len = remaining.chars().count();
        if char_len <= 1900 {
            let trimmed = remaining.trim();
            if !trimmed.is_empty() {
                if is_first {
                    msg.reply(&ctx.http, trimmed).await?;
                } else {
                    msg.channel_id.say(&ctx.http, trimmed).await?;
                }
            }
            break;
        }

        let mut split_char_idx = 1900;
        let chars_vec: Vec<char> = remaining.chars().collect();
        let chunk_chars = &chars_vec[..1900];

        if let Some(pos) = chunk_chars.iter().rposition(|&c| c == '\n') {
            split_char_idx = pos + 1;
        } else if let Some(pos) = chunk_chars.iter().rposition(|&c| c == ' ') {
            split_char_idx = pos + 1;
        }

        if split_char_idx == 0 {
            split_char_idx = 1900;
        }

        let chunk: String = chars_vec[..split_char_idx].iter().collect();
        let rest: String = chars_vec[split_char_idx..].iter().collect();

        let trimmed_chunk = chunk.trim();
        if !trimmed_chunk.is_empty() {
            println!("   [SEND] Dispatching message chunk of {} characters...", trimmed_chunk.len());
            if is_first {
                msg.reply(&ctx.http, trimmed_chunk).await?;
                is_first = false;
            } else {
                msg.channel_id.say(&ctx.http, trimmed_chunk).await?;
            }
        }

        remaining = rest;
    }

    Ok(())
}

struct Handler {
    config: Arc<SandboxConfig>,
    location: String,
    histories: Arc<Mutex<HashMap<u64, Vec<ChatMessage>>>>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot { return; }

        println!("[GRID MONITOR] Message from {} (ID: {}): {}", msg.author.name, msg.author.id, msg.content);

        // 2. Check Whitelist
        if let Some(whitelist) = &self.config.discord_whitelist {
            if !whitelist.contains(&msg.author.id.get()) {
                println!("   [!] Whitelist block. User ID {} is not authorized.", msg.author.id);
                return;
            }
        }

        let is_mentioned = msg.mentions_user_id(ctx.cache.current_user().id);

        println!("   -> Smith is responding...");
        
        let mut input = msg.content.clone();
        // Clean up mentions
        input = input.replace(&format!("<@!{}>", ctx.cache.current_user().id), "");
        input = input.replace(&format!("<@{}>", ctx.cache.current_user().id), "");
        let input = input.trim().to_string();

        if input.is_empty() { return; }

        let _ = msg.channel_id.broadcast_typing(&ctx.http).await;

        let mut history = {
            let mut histories = self.histories.lock().await;
            histories.entry(msg.author.id.get()).or_insert_with(Vec::new).clone()
        };

        // Wrap handle_command in task-local scope to collect diagnostics
        use claw_mcp_server::agent::{DIAGNOSTICS, DiagnosticInfo};
        use std::cell::RefCell;

        let (res, diagnostic) = DIAGNOSTICS.scope(RefCell::new(DiagnosticInfo::default()), async {
            let res = handle_command(&self.config, &mut history, &input, &self.location).await;
            let diag = DIAGNOSTICS.with(|d| d.borrow().clone());
            (res, diag)
        }).await;

        match res {
            Ok(response) => {
                {
                    let mut histories = self.histories.lock().await;
                    histories.insert(msg.author.id.get(), history);
                }
                
                let mut final_response = response;
                // If they explicitly mentioned the bot using @ notation, append the diagnostic footer!
                if is_mentioned {
                    let diag_text = format!(
                        "\n\n🕶️ **Matrix Grid Diagnostic Footprint:**\n\
                         - **Routing Pathway:** `{}` (Category: `{}`)\n\
                         - **Ollama API Calls:** {}\n\
                         - **Gemini API Calls:** {}\n\
                         - **Live Web Search Query:** {}\n\
                         - **Search Engine Latency:** {}",
                        diagnostic.ai_mode,
                        diagnostic.category,
                        if diagnostic.ollama_calls.is_empty() {
                            "`Bypassed`".to_string()
                        } else {
                            diagnostic.ollama_calls.iter().map(|(m, d)| format!("`{}` ({:.2?})", m, d)).collect::<Vec<_>>().join(", ")
                        },
                        if diagnostic.gemini_calls.is_empty() {
                            "`Bypassed`".to_string()
                        } else {
                            diagnostic.gemini_calls.iter().map(|(m, d)| format!("`{}` ({:.2?})", m, d)).collect::<Vec<_>>().join(", ")
                        },
                        diagnostic.search_query.as_ref().map(|q| format!("`\"{}\"`", q)).unwrap_or_else(|| "`None`".to_string()),
                        diagnostic.search_latency.as_ref().map(|d| format!("`{:.2?}`", d)).unwrap_or_else(|| "`N/A`".to_string())
                    );
                    final_response.push_str(&diag_text);
                }

                if let Err(why) = send_long_message(&ctx, &msg, &final_response).await {
                    println!("Error sending message: {:?}", why);
                }
            }
            Err(e) => {
                let _ = send_long_message(&ctx, &msg, &format!("Error: {}", e)).await;
            }
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("\n[SUCCESS] Agent Smith is online in the Matrix as {}", ready.user.name);
        println!("Grid Node Location: {}", self.location);
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("--- Agent Smith (Discord Node) ---");
    
    // Single-instance lock to prevent duplicate bot instances from running simultaneously
    let _lock_socket = match std::net::TcpListener::bind("127.0.0.1:19989") {
        Ok(listener) => listener,
        Err(_) => {
            println!("[!] WARNING: Another instance of Agent Smith Discord Node is already running. Exiting.");
            std::process::exit(0);
        }
    };

    let config = Arc::new(SandboxConfig::load()?);
    let token = config.discord_token.as_ref().expect("DISCORD_TOKEN must be in sandbox_config.json");

    let location = claw_mcp_server::agent::get_current_location().await;

    if config.ai_mode == "hybrid" {
        let ollama_config = config.get_ai_config("ollama").unwrap();
        let _ = ensure_ollama_setup(&ollama_config.default_model).await;
    }

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let handler = Handler {
        config: config.clone(),
        location,
        histories: Arc::new(Mutex::new(HashMap::new())),
    };

    let mut client = Client::builder(token, intents)
        .event_handler(handler)
        .await
        .expect("Err creating client");

    let shard_manager = client.shard_manager.clone();
    tokio::spawn(async move {
        loop {
            if tokio::signal::ctrl_c().await.is_err() {
                break;
            }
            println!("\n(Press Ctrl+C again to exit simulation)");
            
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    println!("\nSmith: The simulation is over. Goodbye, Mr. Anderson.");
                    shard_manager.shutdown_all().await;
                    std::process::exit(0);
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(4)) => {
                    println!("\nSmith: Exit aborted. Simulation continues.\n");
                }
            }
        }
    });

    println!("-> Connecting to Discord Gateway...");
    if let Err(why) = client.start().await {
        println!("Critical Handshake Error: {:?}", why);
    }

    Ok(())
}
