use serde_json::json;

use crate::agent_tools::Tool;

#[derive(Debug)]
pub struct FileSearch;
impl Tool for FileSearch {
    fn name(&self) -> String { "file_search".to_string() }
    fn icon(&self) -> String { "🔍".to_string() }
    fn short(&self, args: serde_json::Value) -> String {
        let pattern = args["pattern"].as_str()
            .unwrap_or("(no pattern provided)").to_string();
        let path = args["path"].as_str()
            .unwrap_or(".").to_string();
        format!("{} -> {}", pattern, path)
    }
    fn is_available(&self) -> bool { true }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": &self.name(),
                "description": "Search files contents using 'ripgrep', uses smart-case and respects gitignore and hidden files",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "The pattern to search for"
                        },
                        "path": {
                            "type": "string",
                            "description": "The path to search in (defaults to . )"
                        },
                    },
                    "required": ["pattern"]
                }
            }
        })
    }
    fn execute(&self, args: serde_json::Value) -> serde_json::Value {
        let path = args["path"].as_str()
            .unwrap_or(".");
        if let Some(pattern) = args["pattern"].as_str() {
            let output = std::process::Command::new("rg")
                .arg(pattern)
                .arg(path)
                .arg("--smart-case")
                .output()
                .expect("Failed to execute rg");
            
            json!({
                "status": if output.status.success() { "success" } else { "error" },
                "exit_code": output.status.code().unwrap(),
                "stdout": String::from_utf8(output.stdout).unwrap(),
                "stderr": String::from_utf8(output.stderr).unwrap()
            })
        } else {
            return json!({
                "status": "error",
                "message": "pattern argument not found"
            });
        }
        
        
    }
}
