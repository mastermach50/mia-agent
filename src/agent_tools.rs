use log::debug;
use serde_json::json;
use chrono::Local;
use anyhow::Result;

/// Returns all available tools for the agent in the proper json format
pub fn get_agent_tools() -> serde_json::Value {
    debug!("Agent tool list fetched");
    json!([
        {
            "type": "function",
            "function": {
                "name": "get_current_datetime",
                "description": "Get the current date and time in RFC2822 format",
                "parameters": {
                    "type": "object",
                    "properties": {}
                }
            }
        }
    ])
}

/// A proxy function that can be used to run any tool by name
pub fn execute_tools(name: &str, arguments: serde_json::Value) -> Result<serde_json::Value> {
    debug!("Tool execution called for: {}", name);
    match name {
        "get_current_datetime" => Ok(get_current_datetime(arguments)),
        _ => anyhow::bail!("Unknown tool: {}", name),
    }
}

/// A helper function to get the icon for a tool for showing in the ui
pub fn get_tool_icon(name: &str) -> String {
    match name {
        "get_current_datetime" => "📅".to_string(),
        _ => "???".to_string(),
    }
}

/// Returns the current date and time in RFC2822 format
fn get_current_datetime(_arguments: serde_json::Value) -> serde_json::Value {
    let now = Local::now();
    debug!("Fetched current datetime");
    json!({
        "status": "success",
        "datetime": now.to_rfc2822(),
    })
}