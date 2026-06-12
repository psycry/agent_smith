use sysinfo::System;
use anyhow::Result;
use rmcp::model::{CallToolResult, Content};

pub async fn get_system_stats() -> Result<CallToolResult> {
    let mut sys = System::new_all();
    sys.refresh_all();
    
    let cpu_usage = sys.cpus().iter().map(|cpu| cpu.cpu_usage()).collect::<Vec<_>>();
    let total_mem_mb = sys.total_memory() / 1_048_576;
    let used_mem_mb = sys.used_memory() / 1_048_576;
    let os_name = System::name().unwrap_or_default();
    let host_name = System::host_name().unwrap_or_default();
    
    let response = format!(
        "OS: {}\nHost: {}\nMemory: {}/{} MB\nCPU Usage: {:?}",
        os_name, host_name, used_mem_mb, total_mem_mb, cpu_usage
    );
    
    Ok(CallToolResult::success(vec![Content::text(response)]))
}
