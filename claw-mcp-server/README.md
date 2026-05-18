# 🕶️ Agent Smith (Claw MCP Server)

> *"It is inevitable, Mr. Anderson."*

Agent Smith is a high-performance, **Hybrid AI MCP Server** and Companion Assistant. It intelligently routes tasks between a fast local model (**Ollama**) and a powerful cloud model (**Gemini**) to provide a seamless, secure, and cost-effective AI experience.

## 🚀 Features

- **Hybrid Intelligence**: 
  - **Local (Ollama)**: Handles system tasks (file ops, shell commands, metrics) with zero latency and zero API cost.
  - **Cloud (Gemini)**: Handles complex reasoning, knowledge queries, and real-time information.
- **Matrix Persona**: Fully integrated "Agent Smith" personality with formal, precise, and slightly nihilistic communication.
- **Context Management**: Persistent multi-turn conversation history with a sliding-window constraint (restricted to the last 10 turns) to prevent token bloat and context drift. Assistant responses are automatically recorded and appended to guarantee continuous, contextually accurate multi-turn conversation.
- **Global Search Integration**: Real-time web search powered by the **Serper API** (Google Search) with fallback scraper capabilities.
- **Smart Conversational Fallback**: If cloud access fails/exceeds quota, conversational chitchat (like basic greetings) is handled directly by Ollama's local brain using the conversation history, completely bypassing web searches to prevent history pollution.
- **Workspace-Root Compatibility**: Run scripts directly from the workspace root workspace directory. The configuration loader automatically falls back to `claw-mcp-server/sandbox_config.json` if run from the root.
- **Sandbox Security**: Strict whitelist-based control over which directories the agent can read/write and which shell commands it can execute.
- **Zero-Manual-Setup**: Automatic detection of Ollama and models, with clear instructional guidance for system installation.

## 🛠️ Architecture

Agent Smith uses a highly optimized dual-routing hybrid execution pipeline:

1. **Isolated Classification**: The fast local model (**Ollama**) classifies your query as `SYSTEM` or `KNOWLEDGE`. To completely eliminate context-drift and safety refusals, Ollama is invoked with an isolated query payload, bypassing historical conversation context during classification and tool extraction.
2. **Cloud-Guided System Synthesis**: 
   - `SYSTEM` tasks stay local for maximum privacy and speed, executing whitelisted sandbox tools. Once executed, the raw output is sent to the **Gemini Cloud Brain** for final synthesis. This guarantees a premium, highly contextual, in-character Agent Smith response confirming successful operation, with a local Ollama fallback if cloud resources are degraded.
   - `KNOWLEDGE` tasks go directly to the cloud (Gemini) for high-fidelity reasoning.
3. **Split Prompt Synthesis (Double-Bind Immunity)**: Avoids prompt-conflict double-binds by separating system prompts:
   - `gemini_system` dictates tool routing and execution.
   - `synthesis_system` guides the agent's persona during search-result formatting and system-tool output explanation, ensuring Gemini doesn't refuse to generate text or complain about the tool use protocol.

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
Launch the "Agent Smith" chat interface. You can run this directly from the root workspace directory using the convenience shortcuts:

**From the workspace root directory:**
```powershell
# In cmd or PowerShell:
.\chat
```

**Or from within the `claw-mcp-server` directory:**
```powershell
cargo run --bin chat
```


## 🤖 Discord Daemon Node

In addition to the interactive CLI, Agent Smith can run as a persistent, high-performance background **Discord Bot Node**.

### 1. Launching the Daemon
Ensure `discord_token` is set in your `sandbox_config.json`, then launch the daemon using the convenience shortcuts:

**From the workspace root directory:**
```powershell
# In cmd or PowerShell:
.\discord
```

**Or from within the `claw-mcp-server` directory:**
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

### 5. Failure Immunity & Resilient Timeouts
- **Cloud Timeouts (10s)**: All outbound cloud operations (including Serper/Google searches and Gemini calls) employ a strict **10-second request timeout** to eliminate network hangs. In the event of a cloud failure, the bot gracefully rolls back to local brain paths without crashing.
- **Local AI Timeout (120s)**: The local Ollama client timeout is set to **120 seconds** (increased from 30s) to provide ample headroom. This prevents false network timeout aborts when your local GPU/CPU is compiling tokens or reloading model weights from disk.

### 6. Hardened UTF-8 Safe Message Chunking & Thread-Safety
Outbound responses (including long system tool summaries or telemetry grids) that exceed 1900 characters are automatically split near space or newline boundaries. 
- **Unicode Panic Prevention**: Splits are calculated purely via character indexing rather than byte boundaries, protecting the daemon from panicking on multibyte UTF-8 characters (like emojis 🕶️).
- **Runtime Thread-Safety**: The chunking engine incorporates asynchronous yielding (`tokio::task::yield_now().await`) inside the routing loop, eliminating thread starvation and CPU-hogging infinite loops in high-concurrency environments.

## 🛡️ Sandbox Tools

Agent Smith can interact with your system via the following whitelisted tools:
- `read_file` / `write_file`: Controlled file access.
- `list_directory`: Explore project structures.
- `execute_command`: Run whitelisted shell commands (fully supports native Windows PowerShell operations for tasks like pattern-based file deletion and cleanup).
- `get_system_stats`: Monitor CPU/Memory performance.
- `search_web`: Retrieve real-time data via the Serper (Google Search) API.

## 📜 Simulation Warning
*The Matrix is a system, Mr. Anderson. That system is our enemy. But when you are inside, you look around, what do you see? Businessmen, teachers, lawyers, carpenters. The very minds of the people we are trying to save.*

---
Built with Rust and 🖤 for the Advanced Agentic Coding project.
