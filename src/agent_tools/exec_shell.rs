use indoc::indoc;
use serde_json::json;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use crate::agent_loop::AgentHandle;
use crate::agent_tools::Tool;
use crate::utils::stdio_capture_and_send;

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
    fn call_summary(&self, args: serde_json::Value) -> String {
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
        let description = indoc! {"
        Execute bash (Unix) or PowerShell (Windows) commands and return stdout, stderr, and exit code.
        The shell used is OS dependent.
        Use this tool to do tasks that do can't be done using other tools.
        "};

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
                        },
                        "working_dir": {
                            "type": "string",
                            "description": "The working directory to run the command in. Defaults to current directory."
                        }
                    },
                    "required": [ "command" ]
                }
            }
        })
    }
    async fn execute(&self, handle: &AgentHandle, args: serde_json::Value) -> serde_json::Value {
        let cmd = match args["command"].as_str() {
            Some(cmd) => cmd,
            None => {
                return json!({
                    "status": "error",
                    "message": "command argument not found"
                });
            }
        };

        let working_dir = match args["working_dir"].as_str() {
            Some(dir) => PathBuf::from(shellexpand::tilde(dir).to_string()),
            None => {
                if let Ok(cwd) = std::env::current_dir() {
                    cwd
                } else {
                    return json!({
                        "status": "error",
                        "message": "Failed to get current working directory"
                    });
                }
            }
        };

        if !handle.ask_permission("Execute?", cmd).await {
            return json!({
                "status": "error",
                "message": "User declined to execute command"
            });
        }

        #[cfg(unix)]
        let mut shell = Command::new("bash");
        #[cfg(unix)]
        shell.arg("-c").arg(cmd);

        #[cfg(windows)]
        let mut shell = Command::new("powershell");
        #[cfg(windows)]
        shell.arg("-command").arg(cmd);

        shell.current_dir(working_dir);

        let mut child = shell
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start command");

        let (stdout_captured, stderr_captured) =
            stdio_capture_and_send(&mut child, |stdout, stderr| {
                handle.partial_tool_output(stdout, stderr)
            });

        let status = child.wait().expect("Failed to wait on child process");

        handle.tool_output(&stdout_captured, &stderr_captured);

        json!({
            "status": if status.success() { "success" } else { "error" },
            "exit_code": status.code().unwrap_or(-1),
            "stdout": stdout_captured,
            "stderr": stderr_captured
        })
    }
}
