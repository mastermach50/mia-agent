use std::{collections::HashMap, sync::OnceLock};
use log::{debug, warn};
use serde_json::{self, json};

mod datetime;
mod shell;
mod file_read;
mod python;
mod memory;

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

static TOOL_REGISTRY: OnceLock<HashMap<String, Box<dyn Tool>>> = OnceLock::new();

pub struct ToolRegistry;
impl ToolRegistry {
    pub fn init() {
        let mut registry = HashMap::new();
        
        Self::register(&mut registry, datetime::DateTime);
        Self::register(&mut registry, shell::Shell);
        Self::register(&mut registry, file_read::FileReader);
        Self::register(&mut registry, python::Python);
        Self::register(&mut registry, memory::Memory);

        TOOL_REGISTRY.set(registry).unwrap();
    }

    fn global() -> &'static HashMap<String, Box<dyn Tool>> {
        TOOL_REGISTRY.get().expect("Tool registry not initialized")
    }

    fn register(registry: &mut HashMap<String, Box<dyn Tool>>, tool: impl Tool + 'static) {
        if !tool.is_available() {
            return;
        }
        let name = tool.name();
        let tool: Box<dyn Tool> = Box::new(tool);
        registry.insert(name, tool);
    }

    /// Generate the full tools schema that has to be sent to the API
    /// Tools that are not available will return a null schema
    pub fn schema() -> serde_json::Value {
        let schema_list = Self::global().values()
            .map(|tool| tool.schema())
            .filter(|s| !s.is_null())
            .collect::<Vec<serde_json::Value>>();
        debug!("Tools Schema: {}", json!(&schema_list));
        json!(schema_list)
    }

    pub fn call(name: &str, args: &str) -> serde_json::Value {
        if let Ok(args) = Self::deserialize_tool_arguments(args.to_string()) {
            match Self::global().get(name) {
                Some(tool) => tool.execute(args),
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
            Some(tool) => tool.icon(),
            None => {
                warn!("Unregistered tool icon requested");
                "❓".to_string()
            },
        }
    }

    pub fn tool_short(name: &str, args: &str) -> String {
        if let Ok(args) = Self::deserialize_tool_arguments(args.to_string()) {
        match Self::global().get(name) {
            Some(tool) => tool.short(args),
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

    fn deserialize_tool_arguments(args: String) -> anyhow::Result<serde_json::Value> {
        if let Ok(new_args) = serde_json::from_str(&args) {
            Ok(new_args)
        } else {
            warn!("Invalid tool arguments");
            anyhow::bail!("Invalid tool arguments or malformed json");
        }
    }
}
