use std::{fs, path::PathBuf};

use indoc::indoc;
use serde_json::json;
use xberg::{ExtractInput, ExtractionConfig};

use crate::{agent_loop::AgentHandle, agent_tools::Tool, config::AppConfig};

#[derive(Debug)]
pub struct ReadFile;
#[async_trait::async_trait]
impl Tool for ReadFile {
    fn name(&self) -> String {
        "read_file".to_string()
    }

    fn icon(&self) -> String {
        if AppConfig::global().tui.use_nerdfont {
            "󰈙".to_string()
        } else {
            "📄".to_string()
        }
    }

    fn call_summary(&self, args: serde_json::Value) -> String {
        args["path"].as_str().unwrap_or_default().to_string()
    }

    fn availability(&self) -> Result<(), String> {
        Ok(())
    }

    fn schema(&self) -> serde_json::Value {
        let description = indoc! {"
        Read text files, documents, presentations, spreadsheets or PDFs and get its contents.
        Plain text files are retreived as is, any other document files are converted to markdown.
        Since retreived document files are converted to markdown they may not be an accurate representation of the document formatting and structure.
        "};

        json!({
            "type": "function",
            "function": {
                "name": &self.name(),
                "description": description,
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The path of the file to be read."
                        },
                    },
                    "required": ["path"]
                }
            }
        })
    }

    async fn execute(&self, _handle: &AgentHandle, args: serde_json::Value) -> serde_json::Value {
        let path = match args["path"].as_str() {
            Some(path) => shellexpand::tilde(path).to_string(),
            None => {
                return json!({
                    "status": "error",
                    "message": "path argument not found"
                });
            }
        };

        if !PathBuf::from(&path).exists() {
            return json!({
                "status": "error",
                "message": "File not found"
            });
        };

        if !PathBuf::from(&path).is_file() {
            return json!({
                "status": "error",
                "message": "Not a file"
            });
        };

        let res = fs::read_to_string(&path);
        if res.is_ok() {
            return json!({
                "status": "success",
                "path": &path,
                "converted": false,
                "contents": res.unwrap()
            });
        } else {
            let err = res.unwrap_err();
            match err.kind() {
                std::io::ErrorKind::InvalidData => {
                    return extract_with_xberg(path).await;
                }
                err => {
                    return json!({
                        "status": "error",
                        "message": "Failed to read file: ".to_owned() + &err.to_string()
                    });
                }
            }
        };
    }
}

async fn extract_with_xberg(path: String) -> serde_json::Value {
    let extraction_config = ExtractionConfig::default();
    let input = ExtractInput::from_uri(&path);
    match xberg::extract(input, &extraction_config).await {
        Ok(extracted) => {
            return json!({
                "status": "success",
                "path": path,
                "converted": true,
                "content": &extracted.results[0].content
            });
        }
        Err(e) => {
            return json!({
                "status": "error",
                "message": "Failed to extract data: ".to_owned() + &e.to_string()
            });
        }
    }
}

mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_file() {
        let (_rx, handle) = AgentHandle::new();
        let tool = ReadFile;
        let out = tool
            .execute(&handle, json!({"path":"./testing/certificate.pdf"}))
            .await;
        println!("{}", &serde_json::to_string_pretty(&out).unwrap());
    }
}
