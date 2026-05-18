@echo off
cargo run --manifest-path "%~dp0claw-mcp-server\Cargo.toml" --bin chat %*
