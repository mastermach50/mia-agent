use indoc::indoc;
use serde_json::json;

use crate::{agent_loop::AgentHandle, agent_tools::Tool};

#[derive(Debug)]
pub struct DocConvert;
#[async_trait::async_trait]
impl Tool for DocConvert {
    fn name(&self) -> String {
        "doc_convert".to_string()
    }
    fn icon(&self) -> String {
        "📃".to_string()
    }
    fn short(&self, args: serde_json::Value) -> String {
        let src = args["input_path"].as_str().unwrap_or_default().to_string();
        let dest = args["output_path"].as_str().unwrap_or_default().to_string();
        format!("{src} -> {dest}")
    }
    fn availability(&self) -> Result<(), String> {
        let mut missing_items = Vec::new();
        if which::which("pandoc").is_err() {
            missing_items.push("pandoc");
        }
        if which::which("pdflatex").is_err() {
            missing_items.push("pdflatex");
        }

        if missing_items.is_empty() {
            Ok(())
        } else {
            Err(format!("{} not found", missing_items.join(", ")))
        }
    }
    fn schema(&self) -> serde_json::Value {
        let description = indoc! {"
        Convert any document into any other document using pandoc.
        Use this to read non text documents by converting them into text files.
        Any intermediate files should be created only in the temp folder of the system.
        "};
        json!({
            "type": "function",
            "function": {
                "name": &self.name(),
                "description": description,
                "parameters": {
                    "type": "object",
                    "properties": {
                        "input_path": {
                            "type": "string",
                            "description": "Path to the source document to convert."
                        },
                        "output_path": {
                            "type": "string",
                            "description": "Path to write the converted document to. Extension determines output format if to_format is not set."
                        },
                        "from_format": {
                            "type": "string",
                            "description": "Input format (e.g. markdown, html, docx, latex). Defaults to pandoc's extension-based detection."
                        },
                        "to_format": {
                            "type": "string",
                            "description": "Output format (e.g. markdown, html, pdf, docx). Defaults to pandoc's extension-based detection."
                        },
                        "extra_args": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Additional raw pandoc flags, e.g. [\"--standalone\", \"--toc\"]."
                        }
                    },
                    "required": ["input_path", "output_path"]
                }
            }
        })
    }

    async fn execute(&self, _handle: &AgentHandle, args: serde_json::Value) -> serde_json::Value {
        let input_path = match args["input_path"].as_str() {
            Some(path) => shellexpand::tilde(path).to_string(),
            None => {
                return json!({
                    "status": "error",
                    "message": "input_path argument not found"
                });
            }
        };
        let output_path = match args["output_path"].as_str() {
            Some(path) => shellexpand::tilde(path).to_string(),
            None => {
                return json!({
                    "status": "error",
                    "message": "output_path argument not found"
                });
            }
        };

        let mut cmd = std::process::Command::new("pandoc");
        cmd.arg(input_path).arg("-o").arg(&output_path);

        if let Some(from_format) = args["from_format"].as_str() {
            cmd.arg("--from").arg(from_format);
        }
        if let Some(to_format) = args["to_format"].as_str() {
            cmd.arg("--to").arg(to_format);
        }
        let allowed_flags = [
            "--standalone",
            "--toc",
            "--number-sections",
            "--self-contained",
        ];
        if let Some(extra_args) = args["extra_args"].as_array() {
            for arg in extra_args {
                if let Some(a) = arg.as_str() {
                    if !allowed_flags.contains(&a) {
                        return json!({
                            "status": "error",
                            "message": format!("flag not permitted: {a}")
                        });
                    }
                    cmd.arg(a);
                }
            }
        }

        let output = match cmd.output() {
            Ok(o) => o,
            Err(e) => {
                return json!({
                    "status": "error",
                    "message": format!("Failed to execute pandoc: {e}")
                });
            }
        };

        json!({
            "status": if output.status.success() { "success" } else { "error" },
            "exit_code": output.status.code().unwrap_or(-1),
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "output_path": output_path
        })
    }
}
