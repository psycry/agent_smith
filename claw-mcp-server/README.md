# 🕶️ Agent Smith (Claw MCP Server)

> *"It is inevitable, Mr. Anderson."*

Agent Smith is a high-performance, **Hybrid AI MCP Server** and Companion Assistant. It intelligently routes tasks between a fast local model (**Ollama**) and a powerful cloud model (**Gemini**) to provide a seamless, secure, and cost-effective AI experience.

## 🚀 Features

- **Hybrid Intelligence**: 
  - **Local (Ollama)**: Handles system tasks (file ops, shell commands, metrics) with zero latency and zero API cost.
  - **Cloud (Gemini)**: Handles complex reasoning, knowledge queries, and real-time information.
- **Matrix Persona**: Fully integrated "Agent Smith" personality with formal, precise, and slightly nihilistic communication.
- **Context Management**: Persistent multi-turn conversation history with a sliding window to maintain context without exceeding token limits.
- **Global Search Integration**: Real-time web search powered by the **Serper API** (Google Search) with fallback scraper capabilities.
- **Sandbox Security**: Strict whitelist-based control over which directories the agent can read/write and which shell commands it can execute.
- **Zero-Manual-Setup**: Automatic detection of Ollama and models, with clear instructional guidance for system installation.

## 🛠️ Architecture

Agent Smith uses a dual-routing system:
1. **Classification**: The local model (Llama 3.2:1B) classifies your request as `SYSTEM` or `KNOWLEDGE`.
2. **Routing**: 
   - `SYSTEM` tasks stay local for maximum privacy and speed.
   - `KNOWLEDGE` tasks go to the cloud (Gemini) for high-fidelity reasoning.

## 📦 Installation & Setup

### 1. Prerequisites
- **Rust**: Installed on your machine.
- **Ollama**: Recommended for the local routing layer.
- **Gemini API Key**: Required for knowledge queries (get one at [Google AI Studio](https://aistudio.google.com/)).

### 2. Configuration
Update `sandbox_config.json` with your API key and allowed paths:

```json
{
  "allowed_paths": ["C:/Users/YourName/Projects"],
  "allowed_commands": ["git", "ls", "cargo"],
  "ai_providers": {
    "gemini": { "api_key": "YOUR_KEY_HERE" }
  }
}
```

### 3. Run the Assistant
Launch the "Agent Smith" chat interface:

```powershell
cargo run --bin chat
```

## 🤖 Discord Daemon Node

In addition to the interactive CLI, Agent Smith can run as a persistent, high-performance background **Discord Bot Node**.

### 1. Launching the Daemon
Ensure `discord_token` is set in your `sandbox_config.json`, then launch the daemon:
```powershell
cargo run --bin discord
```

### 2. Double-Ctrl+C Clean Shutdown
Pressing `Ctrl + C` in the daemon terminal launches an interactive prompt:
- **First Ctrl+C**: Warns the operator: `(Press Ctrl+C again to exit simulation)`.
- **Second Ctrl+C** (within 4 seconds): Cleanly disconnects all active gateway connection shards via the `shard_manager` and exits safely.
- **Timeout**: If 4 seconds pass, the exit is aborted and normal operations resume.

### 3. Mention-Free Auto-Replying
Whitelisted users in `sandbox_config.json` receive **instant, mention-free responses** in DMs and authorized channels. No `@` prefix is required to speak with the Agent.

### 4. Grid Telemetry via `@` Mentions
If you explicitly mention the bot using the `@Agent Smith` notation, the bot will invoke its task-local telemetry collection system and append a **Matrix Grid Diagnostic Footprint** at the end of its response:

```text
🕶️ **Matrix Grid Diagnostic Footprint:**
- **Routing Pathway:** `hybrid` (Category: `KNOWLEDGE`)
- **Ollama API Calls:** `llama3.2:1b` (4.12s)
- **Gemini API Calls:** `gemini-3-flash-preview` (1.05s)
- **Live Web Search Query:** `"tickets to carowinds cost Charlotte..."`
- **Search Engine Latency:** `1.18s`
```

### 5. Failure Immunity & 10s Timeouts
All outbound HTTP operations (including Serper/Google searches and Gemini cloud calls) employ a strict **10-second request timeout** to eliminate network hangs. In the event of a cloud failure, the bot gracefully rolls back to local brain + search explanation paths without crashing.

## 🛡️ Sandbox Tools

Agent Smith can interact with your system via the following whitelisted tools:
- `read_file` / `write_file`: Controlled file access.
- `list_directory`: Explore project structures.
- `execute_command`: Run whitelisted shell commands.
- `get_system_stats`: Monitor CPU/Memory performance.
- `search_web`: Retrieve real-time data via the Serper (Google Search) API.

## 📜 Simulation Warning
*The Matrix is a system, Mr. Anderson. That system is our enemy. But when you are inside, you look around, what do you see? Businessmen, teachers, lawyers, carpenters. The very minds of the people we are trying to save.*

---
Built with Rust and 🖤 for the Advanced Agentic Coding project.
