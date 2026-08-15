use serde_json::json;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use crate::agent_loop::AgentHandle;
use crate::utils::stdio_capture_and_send;
use crate::{agent_tools::Tool, utils::highlight_text};

#[cfg(unix)]
static PYTHON_CMD: &str = "python3";

#[cfg(windows)]
static PYTHON_CMD: &str = "python";

#[derive(Debug)]
pub struct ExecPython;
#[async_trait::async_trait]
impl Tool for ExecPython {
    fn name(&self) -> String {
        "exec_python".to_string()
    }
    fn icon(&self) -> String {
        "🐍".to_string()
    }
    fn call_summary(&self, args: serde_json::Value) -> String {
        let lines = args["code"].as_str().unwrap_or_default().lines().count();
        format!("{lines} lines")
    }
    fn availability(&self) -> Result<(), String> {
        which::which(PYTHON_CMD)
            .map(|_| ())
            .map_err(|_| "python3 not found".to_string())
    }
    fn schema(&self) -> serde_json::Value {
        let description = indoc::indoc! {"
        Execute Python 3 code snippet and get the stdout, stderr, and exit code.
        Use this for tasks that require some scripting or deterministic output such as math.
        Runs in a fresh interpreter — no state persists between calls.
        "};
        json!({
            "type": "function",
            "function": {
                "name": &self.name(),
                "description": description,
                "parameters": {
                    "type": "object",
                    "properties": {
                        "code": {
                            "type": "string",
                            "description": "The code to run."
                        },
                        "working_dir": {
                            "type": "string",
                            "description": "The working directory. Defaults to current directory"
                        }
                    },
                    "required": [ "code" ]
                }
            }
        })
    }
    // TODO refactor this and the shell code
    async fn execute(&self, handle: &AgentHandle, args: serde_json::Value) -> serde_json::Value {
        let code = match args["code"].as_str() {
            Some(code) => code,
            None => {
                return json!({
                    "status": "error",
                    "message": "code argument not found"
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

        let colored_code = highlight_text("something.py", code);

        if !handle
            .ask_permission("Execute Python?", &colored_code)
            .await
        {
            return json!({
                "status": "error",
                "message": "User declined to execute Python code"
            });
        }

        let mut python = Command::new(PYTHON_CMD);
        python.current_dir(working_dir);
        python.arg("-c").arg(code);

        let mut child = python
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
            "command_status_code": status.code().unwrap_or(-1),
            "stdout": stdout_captured,
            "stderr": stderr_captured
        })
    }
}
