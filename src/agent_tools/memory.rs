use std::{fs, iter};
use serde_json::json;

use crate::agent_tools::Tool;

#[derive(Debug)]
pub struct Memory;
impl Tool for Memory {
    fn name(&self) -> String { "memory".to_string() }
    fn icon(&self) -> String { "🧠".to_string() }
    fn short(&self, args: serde_json::Value) -> String {
        let memory_type = args["memory_type"].as_str().unwrap_or_default();
        let operation = args["operation"].as_str().unwrap_or_default();
        let content = args["content"].as_str().unwrap_or_default();
        let operator = if operation == "insert" { "+" } else if operation == "delete" { "-" } else { "?" };
        format!("{} {}{}", memory_type, operator, content)
    }
    fn availability(&self) -> Result<(), String> { Ok(()) }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": &self.name(),
                "description": "Manage memory files - insert, or delete lines. Use this to remember things about the user or yourself.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "memory_type": {
                            "type": "string",
                            "description": "Type of memory: 'user' or 'system'."
                        },
                        "operation": {
                            "type": "string",
                            "description": "Operation to perform: 'insert' or 'delete'."
                        },
                        "content": {
                            "type": "string",
                            "description": "The full line to insert or delete."
                        }
                    },
                    "required": ["memory_type", "operation", "content"]
                }
            }
        })
    }
    fn execute(&self, args: serde_json::Value) -> serde_json::Value {
        let memory_type = args["memory_type"].as_str()
            .expect("Memory type argument not found");
        let operation = args["operation"].as_str()
            .expect("Operation argument not found");
        let content = args["content"].as_str()
            .expect("Content argument not found");
        
        let path = if memory_type == "user" {
            crate::config::AppConfig::global().documents.user_memory.clone()
        } else if memory_type == "system" {
            crate::config::AppConfig::global().documents.system_memory.clone()
        } else {
            return json!({
                "status": "error",
                "message": "Invalid memory type. Use 'user' or 'system'"
            });
        };

        // Remove the bullet point from the content if it exists
        // let content = content.strip_prefix("- ").unwrap_or(content);
        
        match operation {
            "insert" => {
                let current = fs::read_to_string(&path).unwrap_or_default();
                let new = current.lines()
                    .filter(|&l| l != "§")
                    .chain(iter::once(content))
                    .intersperse("§")
                    .collect::<Vec<&str>>()
                    .join("\n");
                fs::write(&path, new)
                    .expect("Failed to write to memory file");
                json!({
                    "status": "success",
                    "operation": "insert",
                    "memory_type": memory_type,
                    "path": path,
                    "message": "Line inserted successfully"
                })
            },
            "delete" => {
                let current = fs::read_to_string(&path).unwrap_or_default();
                let new = current.lines()
                    .filter(|&l| l != "§")
                    .filter(|&l| l != content)
                    .intersperse("§")
                    .collect::<Vec<&str>>()
                    .join("\n");
                fs::write(&path, new).expect("Failed to write to memory file");
                json!({
                    "status": "success",
                    "operation": "delete",
                    "memory_type": memory_type,
                    "path": path,
                    "message": format!("Line deleted successfully")
                })
                
            },
            _ => json!({
                "status": "error",
                "message": "Invalid operation. Use 'insert', 'list', or 'delete'"
            })
        }
    }
}
