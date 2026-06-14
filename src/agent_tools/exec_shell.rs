use serde_json::json;
use std::process::Command;
use std::process::Stdio;
use termimad::crossterm::style::Stylize;

use crate::{
    agent_tools::Tool,
    utils::{ask_permission, stdio_capture_and_print},
};

#[derive(Debug)]
pub struct ExecShell;
#[async_trait::async_trait]
impl Tool for ExecShell {
    fn name(&self) -> String {
        "exec_shell".to_string()
    }
    fn icon(&self) -> String {
        "🐚".to_string()
    }
    fn short(&self, args: serde_json::Value) -> String {
        args["command"].as_str().unwrap_or_default().to_string()
    }
    fn availability(&self) -> Result<(), String> {
        #[cfg(unix)]
        return which::which("bash")
            .map(|_| ())
            .map_err(|_| "bash not found".to_string());

        #[cfg(windows)]
        return which::which("powershell")
            .map(|_| ())
            .map_err(|_| "powershell not found".to_string());
    }
    fn schema(&self) -> serde_json::Value {
        #[cfg(unix)]
        let description = "Run commands in bash.";

        #[cfg(windows)]
        let description = "Run commands in powershell.";

        json!({
            "type": "function",
            "function": {
                "name": &self.name(),
                "description": description,
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The command to run."
                        }
                    },
                    "required": [ "command" ]
                }
            }
        })
    }
    async fn execute(&self, args: serde_json::Value) -> serde_json::Value {
        let command = args["command"]
            .as_str()
            .expect("Command argument not found");

        if ask_permission("Execute?".red(), command) {
            #[cfg(unix)]
            let mut child_process = Command::new("bash");
            #[cfg(unix)]
            child_process.arg("-c").arg(command);

            #[cfg(windows)]
            let mut child_process = Command::new("powershell");
            #[cfg(windows)]
            child_process.arg("-command").arg(command);

            // Configure the process to inherit the current terminal's stdout/stderr
            // AND pipe them if you still need to capture the text for the JSON return.
            let mut child = child_process
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to start command");

            let (stdout_captured, stderr_captured) = stdio_capture_and_print(&mut child);

            let status = child.wait().expect("Failed to wait on child process");

            json!({
                "status": if status.success() { "success" } else { "error" },
                "exit_code": status.code().unwrap_or(-1), // handle potential None if signaled on Unix
                "stdout": stdout_captured,
                "stderr": stderr_captured
            })
        } else {
            json!({
                "status": "error",
                "message": "User declined to execute command"
            })
        }
    }
}
