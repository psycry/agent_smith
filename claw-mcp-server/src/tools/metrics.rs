use sysinfo::System;
use anyhow::Result;
use rmcp::model::{CallToolResult, Content};

pub async fn get_system_stats() -> Result<CallToolResult> {
    let mut sys = System::new_all();
    sys.refresh_all();
    
    let cpu_usage = sys.cpus().iter().map(|cpu| cpu.cpu_usage()).collect::<Vec<_>>();
    let total_mem = sys.total_memory();
    let used_mem = sys.used_memory();
    let os_name = System::name().unwrap_or_default();
    let host_name = System::host_name().unwrap_or_default();
    
    let response = format!(
        "OS: {}\nHost: {}\nMemory: {}/{} KB\nCPU Usage: {:?}",
        os_name, host_name, used_mem, total_mem, cpu_usage
    );
    
    Ok(CallToolResult::success(vec![Content::text(response)]))
}
