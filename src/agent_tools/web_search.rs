use indoc::indoc;
use serde_json::json;

use crate::{agent_loop::AgentHandle, agent_tools::Tool};

#[derive(Debug)]
pub struct WebSearch;
#[async_trait::async_trait]
impl Tool for WebSearch {
    fn name(&self) -> String {
        "web_search".to_string()
    }
    fn icon(&self) -> String {
        "🌐".to_string()
    }
    fn short(&self, args: serde_json::Value) -> String {
        let query = args["query"].as_str().unwrap_or_default().to_string();
        let max_results = args["max_results"].as_u64().unwrap_or(10);
        format!("{query} (top: {max_results})")
    }
    fn availability(&self) -> Result<(), String> {
        std::env::var("TAVILY_API_KEY")
            .map(|_| ())
            .map_err(|_| "TAVILY_API_KEY not found".to_string())
    }
    fn schema(&self) -> serde_json::Value {
        let description = indoc! {"
        Search the web via Tavily and return top results with titles, URLs, and content snippets.
        Use for: current documentation, error message lookup, package discovery, and any question that may have changed since the model's training cutoff.
        Write specific queries — include version numbers, library names, or exact error text for better results.
        Do not keyword stuff queries. To search for multiple things, reuse this tool multiple times.
        Never append the year or any time related parameters to the query without using the datetime tool first.
        Follow up with web_extract on promising URLs to get their full content.
        "};
        json!({
            "type": "function",
            "function": {
                "name": &self.name(),
                "description": description,
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The search query to execute."
                        },
                        "max_results": {
                            "type": "integer",
                            "description": "Maximum number of results to return (default: 10)",
                            "default": 10
                        }
                    },
                    "required": ["query"]
                }
            }
        })
    }
    async fn execute(&self, _handle: &AgentHandle, args: serde_json::Value) -> serde_json::Value {
        let query = args["query"].as_str().expect("Query argument not found");
        let max_results = args["max_results"].as_u64().unwrap_or(5) as i32;

        let api_key = match std::env::var("TAVILY_API_KEY") {
            Ok(key) => key,
            Err(_) => {
                return json!({
                    "status": "error",
                    "message": "TAVILY_API_KEY not set in environment"
                });
            }
        };

        let client = reqwest::Client::new();
        let response = match client
            .post("https://api.tavily.com/search")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&json!({
                "query": query,
                "max_results": max_results
            }))
            .send()
            .await
        {
            Ok(res) => res,
            Err(e) => {
                return json!({
                    "status": "error",
                    "message": format!("Request failed: {}", e)
                });
            }
        };

        let result: serde_json::Value = match response.json().await {
            Ok(json) => json,
            Err(e) => {
                return json!({
                    "status": "error",
                    "message": format!("Failed to parse response: {}", e)
                });
            }
        };

        json!({
            "status": "success",
            "query": query,
            "results": result
        })
    }
}
