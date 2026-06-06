use std::{collections::HashMap, sync::OnceLock};
use log::{debug, warn};
use serde_json::{self, json};

mod datetime;

/// All tools must have this trait implemented
/// Individual tools can be defined in agent_tools/
trait Tool: Send + Sync + std::fmt::Debug {
    fn name(&self) -> String;
    fn icon(&self) -> String;
    fn schema(&self) -> serde_json::Value;
    fn execute(&self, args: serde_json::Value) -> serde_json::Value;
}

static TOOL_REGISTRY: OnceLock<HashMap<String, Box<dyn Tool>>> = OnceLock::new();

pub struct ToolRegistry;
impl ToolRegistry {
    pub fn init() {
        let mut registry = HashMap::new();
        
        Self::register(&mut registry, datetime::DateTime.name(), datetime::DateTime);

        TOOL_REGISTRY.set(registry).unwrap();
    }

    fn global() -> &'static HashMap<String, Box<dyn Tool>> {
        TOOL_REGISTRY.get().expect("Tool registry not initialized")
    }

    fn register(registry: &mut HashMap<String, Box<dyn Tool>>, name: String, tool: impl Tool + 'static) {
        let tool: Box<dyn Tool> = Box::new(tool);
        registry.insert(name, tool);
    }

    pub fn schema() -> serde_json::Value {
        let schema_list = Self::global().values()
            .map(|tool| tool.schema())
            .collect::<Vec<serde_json::Value>>();
        debug!("Tools Schema: {}", json!(&schema_list));
        json!(schema_list)
    }

    pub fn call(name: &str, args: serde_json::Value) -> serde_json::Value {
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
}
