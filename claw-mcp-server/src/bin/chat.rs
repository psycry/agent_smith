use std::io::{self, Write};
use std::sync::Arc;
use tokio;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use claw_mcp_server::config::SandboxConfig;
use claw_mcp_server::agent::{ensure_ollama_setup, handle_command};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Arc::new(SandboxConfig::load()?);
    let mut history = Vec::new();

    println!("--- Agent Smith (CLI) ---");
    
    if config.ai_mode == "hybrid" {
        let ollama_config = config.get_ai_config("ollama").unwrap();
        let _ = ensure_ollama_setup(&ollama_config.default_model, ollama_config.base_url.as_deref()).await;
    }

    let location = claw_mcp_server::agent::get_current_location().await;
    println!("-> Terminal calibrated to Grid Node: {}", location);

    println!("\n\"Hello, Mr. Anderson. How can I help you?\"");
    println!("(Arrows for history, Double-Tap Ctrl+C to exit)\n");

    let mut rl = DefaultEditor::new()?;
    let _ = rl.load_history("history.txt");

    let mut exit_attempt = false;

    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                exit_attempt = false; // Reset exit attempt on input
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(input);

                if input.eq_ignore_ascii_case("exit") {
                    println!("\nSmith: Disconnecting... Goodbye, Mr. Anderson.");
                    break;
                }

                print!("-> Smith is calculating... ");
                io::stdout().flush().unwrap();

                match handle_command(&config, &mut history, input, &location).await {
                    Ok(response) => {
                        print!("\r\x1B[K"); 
                        io::stdout().flush().unwrap();
                        println!("Smith:\n{}\n", response);
                        io::stdout().flush().unwrap();
                    }
                    Err(e) => {
                        println!("\nError: {}\n", e);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                if !exit_attempt {
                    println!("\n(Press Ctrl+C again to exit simulation)");
                    exit_attempt = true;
                    continue;
                } else {
                    println!("\nSmith: The simulation is over. Goodbye, Mr. Anderson.");
                    break;
                }
            }
            Err(ReadlineError::Eof) => {
                println!("\nSmith: Connection lost.");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    let _ = rl.save_history("history.txt");
    Ok(())
}
