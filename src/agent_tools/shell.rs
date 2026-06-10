use colored::Colorize;
use serde_json::json;
use std::process::Command;
use std::io::Read;
use std::process::Stdio;

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
        let description = "Run commands in bash";

        #[cfg(windows)]
        let description = "Run commands in powershell";

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

        // We capture the output into strings while printing them in real-time
        let mut stdout_captured = String::new();
        let mut stderr_captured = String::new();

        if let Some(mut stdout) = child.stdout.take() {
            let mut buffer = [0; u8::MAX as usize];
            while let Ok(bytes_read) = stdout.read(&mut buffer) {
                if bytes_read == 0 { break; }
                if let Ok(text) = std::str::from_utf8(&buffer[..bytes_read]) {
                    print!("{}", text);
                    std::io::Write::flush(&mut std::io::stdout()).unwrap(); // Force instant print
                    stdout_captured.push_str(text);
                }
            }
        }

        if let Some(mut stderr) = child.stderr.take() {
            let mut buffer = [0; u8::MAX as usize];
            while let Ok(bytes_read) = stderr.read(&mut buffer) {
                if bytes_read == 0 { break; }
                if let Ok(text) = std::str::from_utf8(&buffer[..bytes_read]) {
                    eprint!("{}", text);
                    std::io::Write::flush(&mut std::io::stderr()).unwrap();
                    stderr_captured.push_str(text);
                }
            }
        }

        // Wait for the process to fully exit
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
