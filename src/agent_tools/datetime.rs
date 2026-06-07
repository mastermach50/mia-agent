use chrono::Local;
use serde_json::json;

use crate::agent_tools::Tool;

#[derive(Debug)]
pub struct DateTime;
impl Tool for DateTime {
    fn name(&self) -> String { "datetime".to_string() }
    fn icon(&self) -> String { "📅".to_string() }
    fn short(&self, _args: serde_json::Value) -> String { String::new() }
    fn is_available(&self) -> bool { true }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "function",
            "function": {
                "name": "datetime",
                "description": "Get the current date and time in RFC2822 format",
                "parameters": {
                    "type": "object",
                    "properties": {}
                }
            }
        })
    }
    fn execute(&self, _args: serde_json::Value) -> serde_json::Value {
        let current = Local::now().to_rfc2822();
        json!({
            "status": "success",
            "datetime": current
        })
    }
}