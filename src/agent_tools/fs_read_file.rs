use std::fs;
use serde_json::json;

use crate::agent_tools::Tool;

#[derive(Debug)]
pub struct FSReadFile;
impl Tool for FSReadFile {
    fn name(&self) -> String { "fs_read_file".to_string() }
    fn icon(&self) -> String { "📖".to_string() }
    fn short(&self, args: serde_json::Value) -> String {
        args["path"].as_str()
            .unwrap_or_default().to_string()
    }
    fn availability(&self) -> Result<(), String> { Ok(()) }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": &self.name(),
                "description": "Read a file from the filesystem.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The file path to read (relative to current directory)"
                        }
                    },
                    "required": ["path"]
                }
            }
        })
    }
    fn execute(&self, args: serde_json::Value) -> serde_json::Value {
        let path = args["path"].as_str()
            .expect("Path argument not found");
        
        match fs::read_to_string(path) {
            Ok(content) => {
                let preview = if content.len() > 10000 {
                    format!("{}... {} more characters (content > 10000 characters)", 
                        &content[..10000], content.len() - 10000)
                } else {
                    content.clone()
                };
                
                json!({
                    "status": "success",
                    "path": path,
                    "size": content.len(),
                    "content": preview,
                    "full_content": content.len() <= 10000
                })
            },
            Err(e) => json!({
                "status": "error",
                "message": format!("Failed to read file: {}", e)
            })
        }
    }
}
