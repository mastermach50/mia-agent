use serde_json::json;

use crate::agent_tools::Tool;

#[derive(Debug)]
pub struct FSSearchDirs;
impl Tool for FSSearchDirs {
    fn name(&self) -> String { "fs_search_dirs".to_string() }
    fn icon(&self) -> String { "🧭".to_string() }
    fn short(&self, args: serde_json::Value) -> String {
        let pattern = args["pattern"].as_str()
            .unwrap_or("(no pattern provided)").to_string();
        let path = args["path"].as_str()
            .unwrap_or(".").to_string();
        let max_depth = args["max_depth"].as_u64()
            .unwrap_or(5).to_string();
        format!("{} -> {} ({})", pattern, path, max_depth)
    }
    fn availability(&self) -> Result<(), String> { 
        which::which("fd")
            .map(|_| ())
            .map_err(|_| "fd not found".to_string())
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": &self.name(),
                "description": "Search for files in a directory using 'fd'. Uses regex based search.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "The pattern to search for."
                        },
                        "path": {
                            "type": "string",
                            "description": "The path to search in (relative to current directory, defaults to . )"
                        },
                        "max_depth": {
                            "type": "integer",
                            "description": "Maximum depth on recurse into (default: 5)"
                        }
                    },
                    "required": ["pattern"]
                }
            }
        })
    }
    fn execute(&self, args: serde_json::Value) -> serde_json::Value {
        let path = args["path"].as_str()
            .unwrap_or(".");
        let max_depth = args["max_depth"].as_u64()
            .unwrap_or(5).to_string();
        if let Some(pattern) = args["pattern"].as_str() {
            let output = std::process::Command::new("fd")
                .args(["--color", "never", "--hyperlink", "never", "--max-depth", &max_depth])
                .arg(pattern)
                .arg(path)
                .output()
                .expect("Failed to execute fd");
            
            return json!({
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
