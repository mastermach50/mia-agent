use serde_json::json;

use crate::agent_tools::Tool;

#[derive(Debug)]
pub struct WebSearch;
impl Tool for WebSearch {
    fn name(&self) -> String { "web_search".to_string() }
    fn icon(&self) -> String { "🌐".to_string() }
    fn short(&self, args: serde_json::Value) -> String {
        let query = args["query"].as_str()
            .unwrap_or_default()
            .to_string();
        let max_results = args["max_results"].as_u64()
            .unwrap_or(10);
        format!("{query} ({max_results})")
    }
    fn availability(&self) -> Result<(), String> {
        std::env::var("TAVILY_API_KEY")
            .map(|_| ())
            .map_err(|_| "TAVILY_API_KEY not found".to_string())
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": &self.name(),
                "description": "Search the web using Tavily API for relevant results.",
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
    fn execute(&self, args: serde_json::Value) -> serde_json::Value {
        let query = args["query"].as_str()
            .expect("Query argument not found");
        let max_results = args["max_results"].as_u64().unwrap_or(5) as i32;
        
        let api_key = match std::env::var("TAVILY_API_KEY") {
            Ok(key) => key,
            Err(_) => return json!({
                "status": "error",
                "message": "TAVILY_API_KEY not set in environment"
            }),
        };
        
        let client = reqwest::blocking::Client::new();
        let response = match client
            .post("https://api.tavily.com/search")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&json!({
                "query": query,
                "max_results": max_results
            }))
            .send()
        {
            Ok(res) => res,
            Err(e) => return json!({
                "status": "error",
                "message": format!("Request failed: {}", e)
            }),
        };
        
        let result: serde_json::Value = match response.json() {
            Ok(json) => json,
            Err(e) => return json!({
                "status": "error",
                "message": format!("Failed to parse response: {}", e)
            }),
        };
        
        json!({
            "status": "success",
            "query": query,
            "results": result
        })
    }
}
