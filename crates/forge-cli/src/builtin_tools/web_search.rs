//! DuckDuckGo Instant Answer web search tool.

use reqwest::Client;
use serde::Deserialize;
use liteforge::ToolCall;

#[derive(Deserialize)]
struct DuckDuckGoResponse {
    #[serde(rename = "Abstract")]
    abstract_text: Option<String>,
    #[serde(rename = "AbstractURL")]
    abstract_url: Option<String>,
    #[serde(rename = "Heading")]
    heading: Option<String>,
    #[serde(rename = "RelatedTopics")]
    related_topics: Option<Vec<RelatedTopic>>,
}

#[derive(Deserialize)]
struct RelatedTopic {
    #[serde(rename = "Text")]
    text: Option<String>,
    #[serde(rename = "FirstURL")]
    first_url: Option<String>,
}

/// Execute a web search using DuckDuckGo Instant Answers API.
pub async fn execute(call: &ToolCall) -> Result<serde_json::Value, String> {
    let args = call
        .function
        .parse_arguments()
        .map_err(|e| format!("Invalid arguments: {}", e))?;

    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'query' argument")?;

    let client = Client::new();
    let response = client
        .get("https://api.duckduckgo.com/")
        .query(&[("q", query), ("format", "json"), ("no_html", "1")])
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let data: DuckDuckGoResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let mut results = Vec::new();

    // Add abstract (main answer)
    if let Some(abstract_text) = &data.abstract_text {
        if !abstract_text.is_empty() {
            results.push(serde_json::json!({
                "type": "abstract",
                "title": data.heading.as_deref().unwrap_or(""),
                "text": abstract_text,
                "url": data.abstract_url.as_deref().unwrap_or("")
            }));
        }
    }

    // Add related topics (up to 3)
    if let Some(topics) = &data.related_topics {
        for topic in topics.iter().take(3) {
            if let Some(text) = &topic.text {
                results.push(serde_json::json!({
                    "type": "related",
                    "text": text,
                    "url": topic.first_url.as_deref().unwrap_or("")
                }));
            }
        }
    }

    if results.is_empty() {
        Ok(serde_json::json!({
            "message": format!("No instant answers found for '{}'. Try a more specific query.", query)
        }))
    } else {
        Ok(serde_json::Value::Array(results))
    }
}
