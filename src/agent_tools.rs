use std::{collections::HashMap, sync::OnceLock};
use itertools::Itertools;
use log::{trace, warn};
use serde_json::{self, json};

mod datetime;
mod file_list;
mod file_read;
mod file_search;
mod memory;
mod python;
mod shell;
mod web_extract;
mod web_search;

/// All tools must have this trait implemented
/// Individual tools can be defined in agent_tools/
trait Tool: Send + Sync + std::fmt::Debug {
    // Name of the tool
    fn name(&self) -> String;
    // Emoji icon used in the UI
    fn icon(&self) -> String;
    // A short description of what the tool will do, on each tool call
    fn short(&self, args: serde_json::Value) -> String;
    // Check if the tool is available, if not, get a reason
    fn availability(&self) -> Result<(), String>;
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

/// Tool registry management
pub struct ToolRegistry;
impl ToolRegistry {
    pub fn init() {
        let mut registry = HashMap::new();
        
        Self::register(&mut registry, datetime::DateTime);
        Self::register(&mut registry, file_list::FileList);
        Self::register(&mut registry, file_read::FileReader);
        Self::register(&mut registry, file_search::FileSearch);
        Self::register(&mut registry, memory::Memory);
        Self::register(&mut registry, python::Python);
        Self::register(&mut registry, shell::Shell);
        Self::register(&mut registry, web_extract::WebExtract);
        Self::register(&mut registry, web_search::WebSearch);

        TOOL_REGISTRY.set(registry).expect("Failed to set TOOL_REGISTRY");
    }

    /// Get the cached tool registry
    fn global() -> &'static ToolRegistryType {
        TOOL_REGISTRY.get().expect("Tool registry not initialized")
    }

    /// Register a tool by making a tool entry for it and caching it
    fn register(registry: &mut ToolRegistryType, tool: impl Tool + 'static) {
        let name = tool.name();
        let tool: Box<dyn Tool> = Box::new(tool);
        let is_available = tool.availability().is_ok();
        registry.insert(name, ToolEntry { tool, is_available });
    }

    /// Generate the full tools schema that has to be sent to the API
    /// Only tools that are available are included
    pub fn schema() -> serde_json::Value {
        let schema_list = Self::global().values()
            .filter(|tool_entry| tool_entry.is_available)
            .map(|tool_entry| tool_entry.tool.schema())
            .collect::<Vec<serde_json::Value>>();
        trace!("Tools Schema: {}", json!(&schema_list));
        json!(&schema_list)
    }

    /// Call a tool by its function name
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

    /// Get the emoji associated with a tool
    pub fn tool_icon(name: &str) -> String {
        match Self::global().get(name) {
            Some(tool_entry) => tool_entry.tool.icon(),
            None => {
                warn!("Unregistered tool icon requested");
                "❓".to_string()
            },
        }
    }

    /// Get a short info about what arguments went into a tool call
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

    /// Get the status of all the tools (tool_name, is_available, reason)
    pub fn tools_status() -> Vec<(String, bool, String)> {
        Self::global().iter()
            .map(|(tool_name,tool_entry)| {
                let _tool_name = tool_name.clone();
                let _available = tool_entry.is_available;
                let _reason = tool_entry.tool.availability().map(|_| "".to_string()).unwrap_or_else(|e| e);
                (_tool_name, _available, _reason)
            })
            .sorted()
            .collect()
    }

    /// The assistant always returns tool arguments as a JSON string
    fn deserialize_tool_arguments(args: String) -> anyhow::Result<serde_json::Value> {
        if let Ok(new_args) = serde_json::from_str(&args) {
            Ok(new_args)
        } else {
            warn!("Invalid tool arguments");
            anyhow::bail!("Invalid tool arguments or malformed json");
        }
    }
}
