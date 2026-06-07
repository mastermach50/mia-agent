use colored::Colorize;
use serde_json::json;

use crate::{agent_tools::Tool, utils::ask_permission};

#[derive(Debug)]
pub struct Shell;
impl Tool for Shell {
    fn name(&self) -> String { "shell".to_string() }
    fn icon(&self) -> String { "🐚".to_string() }
    fn short(&self, args: serde_json::Value) -> String {
        args["command"].as_str()
            .unwrap_or_default().to_string()
    }
    fn is_available(&self) -> bool {
        std::process::Command::new("which")
            .arg("bash")
            .status()
            .expect("Failed to execute 'which'")
            .success()
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": "shell",
                "description": "Run commands in bash",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The command to run"
                        }
                    },
                    "required": [ "command" ]
                }
            }
        })
    }
    fn execute(&self, args: serde_json::Value) -> serde_json::Value {
        let command = args["command"].as_str()
            .expect("Command argument not found");
        if ask_permission("Execute?".red(), command) {
            let output = std::process::Command::new("bash")
                .arg("-c")
                .arg(command)
                .output()
                .expect("Failed to execute command");
            println!("{}", String::from_utf8(output.stdout.clone()).unwrap());
            json!({
                "status": "success",
                "command_status_code": output.status.code().unwrap(),
                "output": String::from_utf8(output.stdout).unwrap()
            })
        } else {
            json!({
                "status": "error",
                "message": "User declined to execute command"
            })
        }
    }
}