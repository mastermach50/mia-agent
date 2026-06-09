use serde_json::json;

use crate::agent_tools::Tool;

#[derive(Debug)]
pub struct FileList;
impl Tool for FileList {
    fn name(&self) -> String { "file_list".to_string() }
    fn icon(&self) -> String { "📁".to_string() }
    fn short(&self, args: serde_json::Value) -> String {
        args["path"].as_str()
            .unwrap_or(".").to_string()
    }
    fn availability(&self) -> Result<(), String> {
        which::which("ls")
            .map(|_| ())
            .map_err(|_| "ls not found".to_string())
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": &self.name(),
                "description": "List the contents of a folder using 'ls -la'",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The folder path to list (relative to current directory, defaults to current directory)"
                        },
                    },
                    "required": ["path"]
                }
            }
        })
    }
    fn execute(&self, args: serde_json::Value) -> serde_json::Value {
        let path = args["path"].as_str()
            .unwrap_or(".");
        
        let output = std::process::Command::new("ls")
            .arg("-la")
            .arg(path)
            .output()
            .expect("Failed to execute ls");
        
        json!({
            "status": if output.status.success() { "success" } else { "error" },
            "exit_code": output.status.code().unwrap(),
            "stdout": String::from_utf8(output.stdout).unwrap(),
            "stderr": String::from_utf8(output.stderr).unwrap()
        })
        
    }
}
