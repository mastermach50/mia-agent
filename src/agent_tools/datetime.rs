use chrono::Local;
use indoc::indoc;
use serde_json::json;

use crate::{agent_loop::AgentHandle, agent_tools::Tool};

#[derive(Debug)]
pub struct DateTime;
#[async_trait::async_trait]
impl Tool for DateTime {
    fn name(&self) -> String {
        "datetime".to_string()
    }
    fn icon(&self) -> String {
        "📅".to_string()
    }
    fn short(&self, _args: serde_json::Value) -> String {
        Local::now().to_rfc2822()
    }
    fn availability(&self) -> Result<(), String> {
        Ok(())
    }
    fn schema(&self) -> serde_json::Value {
        let description = indoc! {"
        Get the current local date and time in RFC 2822 format.
        Use this tool whenever accurate date or time is required.
        "};
        json!({
            "type": "function",
            "function": {
                "name": &self.name(),
                "description": description,
                "parameters": {
                    "type": "object",
                    "properties": {}
                }
            }
        })
    }
    async fn execute(&self, _handle: &AgentHandle, _args: serde_json::Value) -> serde_json::Value {
        let current = Local::now().to_rfc2822();
        json!({
            "status": "success",
            "datetime": current
        })
    }
}
