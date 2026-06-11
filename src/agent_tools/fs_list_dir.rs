use serde_json::json;

use crate::agent_tools::Tool;

#[derive(Debug)]
pub struct FSListDir;
impl Tool for FSListDir {
    fn name(&self) -> String { "fs_list_dir".to_string() }
    fn icon(&self) -> String { "📁".to_string() }
    fn short(&self, args: serde_json::Value) -> String {
        args["path"].as_str()
            .unwrap_or(".").to_string()
    }
    fn availability(&self) -> Result<(), String> {
        #[cfg(unix)]
        return which::which("ls")
            .map(|_| ())
            .map_err(|_| "ls not found".to_string());
        
        #[cfg(windows)]
        return which::which("cmd")
            .map(|_| ())
            .map_err(|_| "cmd not found".to_string());
    }
    fn schema(&self) -> serde_json::Value {
        #[cfg(unix)]
        let description = "List the contents of a folder. Uses 'ls -la' under the hood.";

        #[cfg(windows)]
        let description = "List the contents of a folder. Uses 'dir /a' under the hood.";

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
                            "description": "The folder path to list (relative to current directory, defaults to . )"
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
        
        #[cfg(unix)]
        let output = std::process::Command::new("ls")
            .arg("-la")
            .arg(path)
            .output()
            .expect("Failed to execute ls");

        #[cfg(windows)]
        let output = std::process::Command::new("cmd")
            .arg("/c")
            .arg("dir")
            .arg("/a")
            .arg(path)
            .output()
            .expect("Failed to execute dir");
        
        json!({
            "status": if output.status.success() { "success" } else { "error" },
            "exit_code": output.status.code().unwrap(),
            "stdout": String::from_utf8(output.stdout).unwrap(),
            "stderr": String::from_utf8(output.stderr).unwrap()
        })
        
    }
}
