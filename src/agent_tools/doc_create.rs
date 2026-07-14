use std::{env::temp_dir, fs, path::PathBuf, process::Command};

use indoc::indoc;
use log::debug;
use serde_json::json;

use crate::agent_tools::Tool;

#[derive(Debug)]
pub struct DocCreate;
#[async_trait::async_trait]
impl Tool for DocCreate {
    fn name(&self) -> String {
        "doc_create".to_string()
    }
    fn icon(&self) -> String {
        "📄".to_string()
    }
    fn short(&self, args: serde_json::Value) -> String {
        let input_format = args["input_format"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let output_path = args["output_file"].as_str().unwrap_or_default().to_string();
        let content_len = args["contents"].as_str().unwrap_or_default().len();
        format!("{input_format} ({content_len} bytes) -> {output_path}")
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
        A tool to create documents from markdown or latex.
        Should be used to create documents like pdf, docx, odt, etc.
        Use doc_create to create a brand new document, use doc_convert only if the source document already exists.
        For any scientific or research documents prefer latex input.
        To simply create a pdf or docx file always use this tool
        "};
        json!({
            "type": "function",
            "function": {
                "name": self.name(),
                "description": description,
                "parameters": {
                    "type": "object",
                    "properties": {
                        "input_format": {
                            "type": "string",
                            "description": "One of 'markdown' or 'latex'"
                        },
                        "contents": {
                            "type": "string",
                            "description": "The contents of the document to create either in markdown or latex format."
                        },
                        "output_file": {
                            "type": "string",
                            "description": "The full path to the output file. Must contain the file extension."
                        }
                    },
                    "required": [ "input_format", "contents", "output_file" ]
                }
            }
        })
    }
    async fn execute(
        &self,
        handle: &crate::agent_loop::AgentHandle,
        args: serde_json::Value,
    ) -> serde_json::Value {
        let input_format = match args["input_format"].as_str() {
            Some(input_format) => {
                if input_format != "markdown" && input_format != "latex" {
                    return json!({
                        "status": "error",
                        "message": "input_format must be 'markdown' or 'latex'"
                    });
                } else {
                    input_format
                }
            }
            None => {
                return json!({
                    "status": "error",
                    "message": "input_format argument not found"
                });
            }
        };
        let contents = match args["contents"].as_str() {
            Some(content) => content,
            None => {
                return json!({
                    "status": "error",
                    "message": "content argument not found"
                });
            }
        };
        let output_file = match args["output_file"].as_str() {
            Some(output_filename) => PathBuf::from(shellexpand::tilde(output_filename).to_string()),
            None => {
                return json!({
                    "status": "error",
                    "message": "output_file argument not found"
                });
            }
        };

        let input_file = temp_dir().join(format!("mia_doc_create-{}.{input_format}", output_file.file_name().unwrap().to_string_lossy()));
        debug!("Writing input to {:?}", input_file);
        fs::write(&input_file, contents).expect("Failed to write input to temp dir");

        if input_format == "latex" && output_file.ends_with(".pdf") {
            let output = Command::new("pdflatex")
                .arg(format!(
                    "-output-directory={}",
                    temp_dir().to_string_lossy()
                ))
                .arg(input_file)
                .arg(output_file)
                .output()
                .expect("Failed to run pdflatex");

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            handle.tool_output(&stdout, &stderr);

            return json!({
                "status": if output.status.success() { "success" } else { "error" },
                "exit_code": output.status.code().unwrap(),
                "stdout": stdout,
                "stderr": stderr
            });
        }

        let output = Command::new("pandoc")
            .arg("--from")
            .arg(input_format)
            .arg(input_file)
            .arg("-o")
            .arg(output_file)
            .output()
            .expect("Failed to run pandoc");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        handle.tool_output(&stdout, &stderr);

        json!({
            "status": if output.status.success() { "success" } else { "error" },
            "exit_code": output.status.code().unwrap(),
            "stdout": stdout,
            "stderr": stderr
        })
    }
}
