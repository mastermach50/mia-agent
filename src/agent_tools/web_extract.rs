use serde_json::json;

use crate::agent_tools::Tool;

#[derive(Debug)]
pub struct WebExtract;
impl Tool for WebExtract {
    fn name(&self) -> String { "web_extract".to_string() }
    fn icon(&self) -> String { "🪏".to_string() }
    fn short(&self, args: serde_json::Value) -> String {
        args["url"].as_str()
            .unwrap_or_default()
            .to_string()
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
                "description": "Extract content from a URL using Tavily API.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "The URL to extract content from."
                        }
                    },
                    "required": ["url"]
                }
            }
        })
    }
    fn execute(&self, args: serde_json::Value) -> serde_json::Value {
        let url = args["url"].as_str()
            .expect("URL argument not found");
        
        let api_key = match std::env::var("TAVILY_API_KEY") {
            Ok(key) => key,
            Err(_) => return json!({
                "status": "error",
                "message": "TAVILY_API_KEY not set in environment"
            }),
        };
        
        let client = reqwest::blocking::Client::new();
        let response = match client
            .post("https://api.tavily.com/extract")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&json!({
                "urls": [url]
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
            "url": url,
            "content": result
        })
    }
}
