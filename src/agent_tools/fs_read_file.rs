use indoc::indoc;
use serde_json::json;
use std::fs;

use crate::{agent_loop::AgentHandle, agent_tools::Tool};

#[derive(Debug)]
pub struct FSReadFile;
#[async_trait::async_trait]
impl Tool for FSReadFile {
    fn name(&self) -> String {
        "fs_read_file".to_string()
    }
    fn icon(&self) -> String {
        "📖".to_string()
    }
    fn short(&self, args: serde_json::Value) -> String {
        args["path"].as_str().unwrap_or_default().to_string()
    }
    fn availability(&self) -> Result<(), String> {
        Ok(())
    }
    fn schema(&self) -> serde_json::Value {
        let description = indoc! {"
        Read a text file and return its content.
        Returns up to 10,000 characters; larger files are truncated with the remaining byte count noted.
        Use for source code, configs, logs, and any plain-text file.
        For binary files use exec_shell (e.g. `file`, `xxd`).
        If a file is truncated, use exec_shell with `sed -n 'X,Yp'` or `tail -n N` to read a specific range.
        "};
        json!({
            "type": "function",
            "function": {
                "name": &self.name(),
                "description": description,
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
    async fn execute(&self, _handle: &AgentHandle, args: serde_json::Value) -> serde_json::Value {
        let path = args["path"].as_str().expect("Path argument not found");

        match fs::read_to_string(path) {
            Ok(content) => {
                let preview = if content.len() > 10000 {
                    format!(
                        "{}... {} more characters (content > 10000 characters)",
                        &content[..10000],
                        content.len() - 10000
                    )
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
            }
            Err(e) => json!({
                "status": "error",
                "message": format!("Failed to read file: {}", e)
            }),
        }
    }
}
