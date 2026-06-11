use termimad::crossterm::style::Stylize;
use std::process::Command;
use std::process::Stdio;
use serde_json::json;

use crate::{agent_tools::Tool, utils::{ask_permission, highlight_text, stdio_capture_and_print}};

#[cfg(unix)]
static PYTHON_CMD: &str = "python3";

#[cfg(windows)]
static PYTHON_CMD: &str = "python";

#[derive(Debug)]
pub struct ExecPython;
impl Tool for ExecPython {
    fn name(&self) -> String { "exec_python".to_string() }
    fn icon(&self) -> String { "🐍".to_string() }
    fn short(&self, args: serde_json::Value) -> String {
        args["code"].as_str()
            .unwrap_or_default().to_string()
    }
    fn availability(&self) -> Result<(), String> {
        which::which(PYTHON_CMD)
            .map(|_| ())
            .map_err(|_| "python3 not found".to_string())
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": &self.name(),
                "description": "Execute Python 3 code.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "code": {
                            "type": "string",
                            "description": "The Python code to execute."
                        }
                    },
                    "required": [ "code" ]
                }
            }
        })
    }
    // TODO refactor this and the shell code
    fn execute(&self, args: serde_json::Value) -> serde_json::Value {
        let code = args["code"].as_str()
            .expect("Code argument not found");

        let colored_code = highlight_text("something.py", code);

        if ask_permission("Execute Python?".red(), &colored_code) {
            let mut child_process = Command::new(PYTHON_CMD);
            child_process.arg("-c").arg(code);

            let mut child = child_process
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to start command");

            let (stdout_captured, stderr_captured) = stdio_capture_and_print(&mut child);

            let status = child.wait().expect("Failed to wait on child process");

            json!({
                "status": if status.success() { "success" } else { "error" },
                "command_status_code": status.code().unwrap_or(-1),
                "stdout": stdout_captured,
                "stderr": stderr_captured
            })
        } else {
            json!({
                "status": "error",
                "message": "User declined to execute Python code"
            })
        }
    }
}
