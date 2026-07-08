use indoc::indoc;
use serde_json::json;
use std::fs;

use crate::{
    agent_loop::AgentHandle,
    agent_tools::Tool,
    utils::{self},
};

#[derive(Debug)]
pub struct FSWriteFile;
#[async_trait::async_trait]
impl Tool for FSWriteFile {
    fn name(&self) -> String {
        "fs_write_file".to_string()
    }
    fn icon(&self) -> String {
        "✍️".to_string()
    }
    fn short(&self, args: serde_json::Value) -> String {
        let path = args["path"].as_str().unwrap_or_default().to_string();
        let content_len = args["content"].as_str().unwrap_or_default().len();
        format!("{} ({} bytes)", path, content_len)
    }
    fn availability(&self) -> Result<(), String> {
        Ok(())
    }
    fn schema(&self) -> serde_json::Value {
        let description = indoc! {"
        Write text content to a file, creating it if it does not exist or fully overwriting it if it does.
        Requires explicit user approval before writing.
        Use for saving generated code, configs, and new files.
        This replaces the entire file — for targeted edits to existing files, prefer exec_shell with sed, awk, or a Python one-liner to avoid clobbering unchanged content.
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
                            "description": "The file path to write to (can be relative to the current directory)"
                        },
                        "content": {
                            "type": "string",
                            "description": "The content to write to the file."
                        }
                    },
                    "required": ["path", "content"]
                }
            }
        })
    }
    async fn execute(&self, handle: &AgentHandle, args: serde_json::Value) -> serde_json::Value {
        let path = args["path"].as_str().expect("Path argument not found");
        let content = args["content"]
            .as_str()
            .expect("Content argument not found");

        let colored_content = utils::highlight_text(path, content);

        let header = format!("Write to {}", path);
        if handle.ask_permission(header, &colored_content).await {
            match fs::write(path, content) {
                Ok(_) => {
                    json!({
                        "status": "success",
                        "path": path,
                        "size": content.len(),
                        "message": format!("File written successfully ({} bytes)", content.len())
                    })
                }
                Err(e) => json!({
                    "status": "error",
                    "message": format!("Failed to write file: {}", e)
                }),
            }
        } else {
            json!({
                "status": "error",
                "message": "User declined to write file"
            })
        }
    }
}
