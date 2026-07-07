use serde_json::json;
use std::process::Command;
use std::process::Stdio;

use crate::agent_loop::AgentHandle;
use crate::{
    agent_tools::Tool,
    utils::{highlight_text, stdio_capture_and_print},
};

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
    fn short(&self, args: serde_json::Value) -> String {
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
        Execute a Python 3 script and return stdout, stderr, and exit code.
        Best for: numerical computation, data parsing (JSON/CSV/XML), string processing, generating formatted output, and tasks that benefit from Python's standard library without shell pipelines.
        Runs in a fresh interpreter — no state persists between calls.
        For tasks involving shell utilities or system commands, use exec_shell instead.
        "};
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
                            "description": description
                        }
                    },
                    "required": [ "code" ]
                }
            }
        })
    }
    // TODO refactor this and the shell code
    async fn execute(&self, handle: &AgentHandle, args: serde_json::Value) -> serde_json::Value {
        let code = args["code"].as_str().expect("Code argument not found");

        let colored_code = highlight_text("something.py", code);

        if handle
            .ask_permission("Execute Python?", &colored_code)
            .await
        {
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
