use serde_json::json;
use std::fs;
use termimad::crossterm::style::Stylize;

use crate::{
    agent_tools::Tool,
    utils::{self, ask_permission},
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
        args["path"].as_str().unwrap_or_default().to_string()
    }
    fn availability(&self) -> Result<(), String> {
        Ok(())
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": &self.name(),
                "description": "Write content to a file. Creates new files or overwrites existing ones.",
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
    async fn execute(&self, args: serde_json::Value) -> serde_json::Value {
        let path = args["path"].as_str().expect("Path argument not found");
        let content = args["content"]
            .as_str()
            .expect("Content argument not found");

        let colored_content = utils::highlight_text(path, content);

        let header = format!("{} {}", "Write to".red(), path.yellow());
        if ask_permission(header, &colored_content) {
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
