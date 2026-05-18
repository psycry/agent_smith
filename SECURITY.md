# Security Policy

We take the security of **Agent Smith** extremely seriously. As a hybrid agent executing system tasks and file operations, maintaining a hardened security perimeter is our highest priority. 

This document outlines our supported versions, reporting procedures, and security guarantees.

---

## 🔒 Supported Versions

We actively monitor and patch the latest release branch. Please ensure you are running the most up-to-date micro-version before reporting a vulnerability.

| Version | Supported | Security Patches |
|---------|-----------|------------------|
| `0.1.x` | Yes       | Active           |
| `< 0.1` | No        | None             |

---

## 🛡️ Sandbox Guarantees & Scope

Agent Smith executes in a **user-configured sandbox**. The following elements are in-scope for security evaluations:
- **Directory Traversal**: Circumventing the path traversal shield to read, write, list, or delete files outside the whitelisted directories specified in `sandbox_config.json`.
- **Command Injection**: Bypassing the whitelist check to execute unapproved binaries or inject shell command chains (e.g., executing commands via operator chaining `&&`, `||`, `;`).
- **Telemetry Exposure**: Unintended credential leaks or system data leaks outside the configured Discord guild diagnostic parameters.

### Out of Scope
- Compromises resulting from exposing or leaking your local `sandbox_config.json` containing live Gemini, Serper, or Discord API keys.
- VRAM starvation or system lag caused by intensive local inference on resource-constrained host machines.

---

## 📬 Reporting a Vulnerability

**Please do not open a public GitHub Issue for security vulnerabilities.**

If you discover a security vulnerability, we request that you report it responsibly via private email:

- **Contact**: [security@digitalnomad.sh](mailto:security@digitalnomad.sh)
- **Subject**: `[SECURITY VULNERABILITY] Agent Smith - <Brief Description>`

### What to Include
To help us triage and patch the issue quickly, please provide:
1. A brief summary of the vulnerability type (e.g., path traversal, logic bypass).
2. A step-by-step Proof of Concept (PoC) or input payload that triggers the issue.
3. System details (OS, Ollama version, serenity/discord gateway state).

---

## ⏱️ Response & Patch SLA

We appreciate the security community's work in keeping open-source safe. 
- **Triage**: We will acknowledge and verify your report within **24 to 48 hours**.
- **Fix**: A patched release will be developed and committed directly to the main branch within **72 hours** of validation.
- **Credit**: Upon successful resolution, we will happily credit you in the release notes and `walkthrough.md` if you wish to remain public.
