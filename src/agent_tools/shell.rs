use std::io::stdin;
use colored::Colorize;
use log::debug;
use serde_json::json;

use crate::agent_tools::Tool;

#[derive(Debug)]
pub struct Shell;
impl Tool for Shell {
    fn name(&self) -> String { "shell".to_string() }
    fn icon(&self) -> String { "🐚".to_string() }
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
        debug!("{:?}", args);
        let command = args["command"].as_str()
            .expect("Command argument not found");
        let mut input = String::new();
        print!("{} > Do you want to execute:\n{}\n[y/n]", "System".yellow(), command);
        stdin().read_line(&mut input)
            .expect("Failed to read user input");
        if input.trim() == "y" {
            let output = std::process::Command::new("bash")
                .arg("-c")
                .arg(command)
                .output()
                .expect("Failed to execute command");
            json!({
                "status": "success",
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