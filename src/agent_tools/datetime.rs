use chrono::Local;
use indoc::indoc;
use serde_json::json;

use crate::agent_tools::Tool;

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
        String::new()
    }
    fn availability(&self) -> Result<(), String> {
        Ok(())
    }
    fn schema(&self) -> serde_json::Value {
        let description = indoc! {"
        Return the current local date and time in RFC 2822 format.
        Call this at the start of any time-sensitive task — scheduling, logging, timestamping, or when the user asks what time it is.
        More accurate than the approximate datetime in the system prompt.
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
    async fn execute(&self, _args: serde_json::Value) -> serde_json::Value {
        let current = Local::now().to_rfc2822();
        json!({
            "status": "success",
            "datetime": current
        })
    }
}
