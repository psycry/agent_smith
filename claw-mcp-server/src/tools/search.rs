use anyhow::Result;
use serde::{Deserialize, Serialize};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT, CONTENT_TYPE};
use schemars::JsonSchema;
use crate::config::SandboxConfig;
use rmcp::model::CallToolResult;
use std::io::{self, Write};
use std::time::Duration;

#[derive(Deserialize, JsonSchema)]
pub struct SearchWebInput {
    pub query: String,
}

#[derive(Serialize)]
pub struct SearchResult {
    pub title: String,
    pub link: String,
    pub snippet: String,
}

pub async fn search_web(config: &SandboxConfig, input: SearchWebInput) -> Result<CallToolResult> {
    let start_time = std::time::Instant::now();
    let query = input.query.clone();
    let _ = crate::agent::DIAGNOSTICS.try_with(|d| {
        d.borrow_mut().search_query = Some(query);
    });

    let res = search_web_inner(config, input).await;

    let duration = start_time.elapsed();
    let _ = crate::agent::DIAGNOSTICS.try_with(|d| {
        d.borrow_mut().search_latency = Some(duration);
    });

    res
}

async fn search_web_inner(config: &SandboxConfig, input: SearchWebInput) -> Result<CallToolResult> {
    // 1. Check if user provided a Serper API Key
    if let Some(serper_config) = config.ai_providers.get("serper") {
        let serper_key = &serper_config.api_key;
        if serper_key != "none" && !serper_key.is_empty() {
            print!("\r-> Accessing the Global Grid (Serper)... ");
            io::stdout().flush()?;

            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()?;
            let response = client.post("https://google.serper.dev/search")
                .header("X-API-KEY", serper_key)
                .header(CONTENT_TYPE, "application/json")
                .json(&serde_json::json!({ "q": input.query }))
                .send()
                .await?;

            if response.status().is_success() {
                let json: serde_json::Value = response.json().await?;
                let mut output = String::new();
                if let Some(results) = json["organic"].as_array() {
                    if results.is_empty() {
                        return Ok(CallToolResult::success(vec![rmcp::model::Content::text("The search was successful, but no active events or data matching that query were found for the current date.")]));
                    }
                    for r in results.iter().take(5) {
                        output.push_str(&format!("### {}\nLink: {}\n{}\n\n", 
                            r["title"].as_str().unwrap_or_default(),
                            r["link"].as_str().unwrap_or_default(),
                            r["snippet"].as_str().unwrap_or_default()
                        ));
                    }
                    return Ok(CallToolResult::success(vec![rmcp::model::Content::text(output)]));
                }
            } else {
                println!("Error: Serper API returned {}", response.status());
            }
        }
    }

    // 2. Fallback to Legacy Google Scraper
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/115.0"));
    let client = reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(10))
        .build()?;
    
    let url = format!("https://www.google.com/search?q={}&gbv=1", urlencoding::encode(&input.query));
    let response = client.get(&url).send().await?;
    let html = response.text().await?;

    let mut results = Vec::new();
    let document = scraper::Html::parse_document(&html);
    let result_selector = scraper::Selector::parse("div.g, div.ZIN69b").unwrap();
    let title_selector = scraper::Selector::parse("h3").unwrap();
    let link_selector = scraper::Selector::parse("a").unwrap();
    let snippet_selector = scraper::Selector::parse("div.VwiC3b, span.st, div.kvH9C").unwrap();

    for element in document.select(&result_selector).take(5) {
        let title = element.select(&title_selector).next().map(|e| e.text().collect::<String>()).unwrap_or_default();
        let link = element.select(&link_selector).next().and_then(|e| e.value().attr("href")).unwrap_or_default().to_string();
        let snippet = element.select(&snippet_selector).next().map(|e| e.text().collect::<String>()).unwrap_or_default();
        if !title.is_empty() { results.push((title, link, snippet)); }
    }

    if results.is_empty() {
        return Ok(CallToolResult::success(vec![rmcp::model::Content::text("The grid is silent. No data matching your query was found. (Internal: Scraper Blocked)")]));
    }

    let mut output = String::new();
    for (t, l, s) in results {
        output.push_str(&format!("### {}\nLink: {}\n{}\n\n", t, l, s));
    }
    Ok(CallToolResult::success(vec![rmcp::model::Content::text(output)]))
}
