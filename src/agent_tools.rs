use std::{collections::HashMap, sync::OnceLock};
use itertools::Itertools;
use log::{debug, warn};
use serde_json::{self, json};

mod datetime;
mod shell;
mod file_read;
mod python;
mod memory;
mod web_search;
mod web_extract;


/// All tools must have this trait implemented
/// Individual tools can be defined in agent_tools/
trait Tool: Send + Sync + std::fmt::Debug {
    // Name of the tool
    fn name(&self) -> String;
    // Emoji icon used in the UI
    fn icon(&self) -> String;
    // A short description of what the tool will do, on each tool call
    fn short(&self, args: serde_json::Value) -> String;
    // Check if the tool is available
    fn is_available(&self) -> bool;
    // OpenAI compatible tool schema
    fn schema(&self) -> serde_json::Value;
    // Execute the tool logic
    fn execute(&self, args: serde_json::Value) -> serde_json::Value;
}


#[derive(Debug)]
pub struct ToolEntry {
    tool: Box<dyn Tool>,
    is_available: bool,
}
type ToolRegistryType = HashMap<String, ToolEntry>;

// function_name -> { tool, is_available }
static TOOL_REGISTRY: OnceLock<ToolRegistryType> = OnceLock::new();
pub struct ToolRegistry;
impl ToolRegistry {
    pub fn init() {
        let mut registry = HashMap::new();
        
        Self::register(&mut registry, datetime::DateTime);
        Self::register(&mut registry, shell::Shell);
        Self::register(&mut registry, file_read::FileReader);
        Self::register(&mut registry, python::Python);
        Self::register(&mut registry, memory::Memory);
        Self::register(&mut registry, web_search::WebSearch);
        Self::register(&mut registry, web_extract::WebExtract);

        TOOL_REGISTRY.set(registry).expect("Failed to set TOOL_REGISTRY");
    }

    fn global() -> &'static ToolRegistryType {
        TOOL_REGISTRY.get().expect("Tool registry not initialized")
    }

    fn register(registry: &mut ToolRegistryType, tool: impl Tool + 'static) {
        let name = tool.name();
        let tool: Box<dyn Tool> = Box::new(tool);
        let is_available = tool.is_available();
        registry.insert(name, ToolEntry { tool, is_available });
    }

    /// Generate the full tools schema that has to be sent to the API
    /// Only tools that are available are included
    pub fn schema() -> serde_json::Value {
        let schema_list = Self::global().values()
            .filter(|tool_entry| tool_entry.is_available)
            .map(|tool_entry| tool_entry.tool.schema())
            .collect::<Vec<serde_json::Value>>();
        debug!("Tools Schema: {}", json!(&schema_list));
        json!(&schema_list)
    }

    pub fn call(name: &str, args: &str) -> serde_json::Value {
        if let Ok(args) = Self::deserialize_tool_arguments(args.to_string()) {
            match Self::global().get(name) {
                Some(tool_entry) => tool_entry.tool.execute(args),
                None => {
                    warn!("Unregistered tool requested");
                    json!({
                        "status": "error",
                        "message": "Unregistered tool requested"
                    })
                }
            }
        } else {
            warn!("Invalid tool arguments");
            json!({
                "status": "error",
                "message": "Invalid tool arguments or malformed json"
            })
        }
    }

    pub fn tool_icon(name: &str) -> String {
        match Self::global().get(name) {
            Some(tool_entry) => tool_entry.tool.icon(),
            None => {
                warn!("Unregistered tool icon requested");
                "❓".to_string()
            },
        }
    }

    pub fn tool_short(name: &str, args: &str) -> String {
        if let Ok(args) = Self::deserialize_tool_arguments(args.to_string()) {
        match Self::global().get(name) {
            Some(tool_entry) => tool_entry.tool.short(args),
            None => {
                warn!("Unregistered tool short requested");
                "❓".to_string()
            },
        }
        } else {
            warn!("Invalid tool arguments");
            "❓".to_string()
        }
    }

    pub fn tools_status() -> Vec<(String, bool)> {
        Self::global().iter()
            .map(|(tool_name,tool_entry)| (tool_name.clone(), tool_entry.is_available))
            .sorted()
            .collect()
    }

    fn deserialize_tool_arguments(args: String) -> anyhow::Result<serde_json::Value> {
        if let Ok(new_args) = serde_json::from_str(&args) {
            Ok(new_args)
        } else {
            warn!("Invalid tool arguments");
            anyhow::bail!("Invalid tool arguments or malformed json");
        }
    }
}
