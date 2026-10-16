use log::{trace, warn};
use serde_json::{self, json};
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use crate::agent_loop::AgentHandle;

/// The result of a tool call
/// Only for use within the codebase, the LLM never sees this
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolStatus {
    /// The tool call succeeded
    Success,
    /// Tool call failed
    Error(String),
    /// The tool call was denied by the user
    Denied,
}

/// The session related context passed to the tool
#[derive(Debug)]
pub struct ToolContext<'a> {
    pub handle: &'a AgentHandle,
    pub session_id: &'a str,
}

/// The permission class of the tool
/// Decided whether the tool requires a permission prompt
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionClass {
    /// Zero side effects
    ReadOnly,
    /// Has side effects, but limited to a single file
    Write,
    /// Arbitrary side effects as a result of some command execution
    Execute,
}

/// The tool trait
///
/// This must be implemented by all tools
#[async_trait::async_trait]
pub trait Tool: Send + Sync + std::fmt::Debug {
    // --- Metadata ---
    /// Name of the tools.
    fn name(&self) -> &'static str;
    /// Icon of the tool.
    fn icon(&self) -> &'static str;
    /// Short description of what this tool call does.
    fn call_summary(&self, args: &serde_json::Value) -> String;
    /// OpenAI function schema.
    fn schema(&self) -> serde_json::Value;
    /// Permission class of the tool.
    fn permission(&self) -> PermissionClass;
    /// (header, content) shown in the permission prompt. Only required for non
    /// read-only tools.
    fn permission_prompt(&self, args: &serde_json::Value) -> (String, String);
    /// Availability of the tool, subject to some condition such as command or
    /// api key availability. If not available then returns the reason.
    fn availability(&self) -> Result<(), String>;

    // --- Execution ---
    /// Execute the tool. Returns (status for the harness, JSON payload for the LLM).
    async fn execute(
        &self,
        ctx: &ToolContext,
        args: serde_json::Value,
    ) -> (ToolStatus, serde_json::Value);
}

/// A registered tool.
struct ToolEntry {
    tool: Arc<dyn Tool>,
    is_enabled: bool,
}

type RegistryType = HashMap<String, ToolEntry>;
static REGISTRY: OnceLock<RegistryType> = OnceLock::new();

/// Tool registry management.
pub struct ToolRegistry;

impl ToolRegistry {
    /// Create the tool registry. Called once from `main.rs`.
    ///
    /// Tools are registered here as they are authored against the new architecture.
    pub fn init() {
        let mut registry = HashMap::new();
        // Register tools here as they are rewritten, e.g.:
        //   Self::register(&mut registry, datetime::DateTime);
        let _ = &mut registry; // placeholder until tools are ported
        assert!(
            REGISTRY.set(registry).is_ok(),
            "Failed to set REGISTRY (init called twice)"
        );
    }

    /// Get the cached tool registry.
    fn global() -> &'static RegistryType {
        REGISTRY.get().expect("Tool registry not initialized")
    }

    /// Register a tool.
    fn register(registry: &mut RegistryType, tool: impl Tool + 'static) {
        Self::register_with(registry, tool, true);
    }

    /// Register a tool with an explicit enabled flag (user config layer).
    fn register_with(registry: &mut RegistryType, tool: impl Tool + 'static, is_enabled: bool) {
        let shared = Arc::new(tool);

        let name = shared.name().to_string();
        registry.insert(
            name,
            ToolEntry {
                tool: shared,
                is_enabled,
            },
        );
    }

    /// Full tool schema sent to the API. Only `enabled && available` tools are included;
    /// availability is probed live on every call.
    pub fn schema() -> serde_json::Value {
        let schema_list = Self::global()
            .values()
            .filter(|entry| entry.is_enabled && entry.tool.availability().is_ok())
            .map(|entry| entry.tool.schema())
            .collect::<Vec<serde_json::Value>>();
        trace!("Tools Schema: {}", json!(&schema_list));
        json!(&schema_list)
    }

    /// Call a tool by its function name.
    ///
    /// Flow: parse args -> lookup -> permission gate (non-ReadOnly) -> execute.
    pub async fn call(
        ctx: &ToolContext<'_>,
        name: &str,
        args: &str,
    ) -> (ToolStatus, serde_json::Value) {
        let args = match Self::deserialize_tool_arguments(args) {
            Ok(args) => args,
            Err(_) => {
                warn!("Invalid tool arguments for {name}");
                return err("Invalid tool arguments or malformed json");
            }
        };

        let entry = match Self::global().get(name) {
            Some(entry) => entry,
            None => {
                warn!("Unregistered tool requested: {name}");
                return err("Unregistered tool requested");
            }
        };

        if !entry.is_enabled {
            warn!("Disabled tool requested: {name}");
            return err("Tool is disabled");
        }

        // Clone the Arc so the registry borrow ends before we await.
        let tool = entry.tool.clone();

        if tool.permission() != PermissionClass::ReadOnly {
            let (header, content) = tool.permission_prompt(&args);
            if !ctx.handle.ask_permission(header, content).await {
                warn!("Permission denied for tool {name}");
                return denied();
            }
        }

        tool.execute(ctx, args).await
    }

    /// Emoji associated with a tool (UI).
    pub fn tool_icon(name: &str) -> &'static str {
        match Self::global().get(name) {
            Some(entry) => entry.tool.icon(),
            None => {
                warn!("Unregistered tool icon requested: {name}");
                "❓"
            }
        }
    }

    /// Short info about what arguments went into a tool call (UI).
    pub fn tool_short(name: &str, args: &str) -> String {
        let Ok(args) = Self::deserialize_tool_arguments(args) else {
            warn!("Invalid tool arguments for {name}");
            return "❓".to_string();
        };
        match Self::global().get(name) {
            Some(entry) => entry.tool.call_summary(&args),
            None => {
                warn!("Unregistered tool short requested: {name}");
                "❓".to_string()
            }
        }
    }

    /// Status of all tools: (tool_name, is_available, reason). Probes availability live.
    pub fn tools_status() -> Vec<(String, bool, String)> {
        let mut status = Self::global()
            .iter()
            .map(|(tool_name, entry)| {
                let (is_available, reason) = match entry.tool.availability() {
                    Ok(()) => (true, String::new()),
                    Err(reason) => (false, reason),
                };
                (tool_name.clone(), is_available, reason)
            })
            .collect::<Vec<_>>();
        status.sort_by(|a, b| a.0.cmp(&b.0));
        status
    }

    /// The assistant always returns tool arguments as a JSON string.
    fn deserialize_tool_arguments(args: &str) -> anyhow::Result<serde_json::Value> {
        serde_json::from_str(args).map_err(|e| anyhow::anyhow!("Invalid tool arguments: {e}"))
    }
}

/// Build a success result: status + payload in one shot.
pub fn ok(payload: serde_json::Value) -> (ToolStatus, serde_json::Value) {
    (ToolStatus::Success, payload)
}

/// Build a standard error result; the enum reason and the LLM payload always agree.
pub fn err(reason: impl Into<String>) -> (ToolStatus, serde_json::Value) {
    let reason = reason.into();
    (
        ToolStatus::Error(reason.clone()),
        json!({ "status": "error", "message": reason }),
    )
}

/// Build a permission-denied result.
pub fn denied() -> (ToolStatus, serde_json::Value) {
    (
        ToolStatus::Denied,
        json!({ "status": "error", "message": "User declined" }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_loop::AgentEvent;
    use std::sync::Once;

    // The registry is a process-wide OnceLock; the first test to run seeds it with
    // the dummy tools and every other test reuses it.
    fn init_test_registry() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            let mut registry = HashMap::new();
            ToolRegistry::register(&mut registry, DummyTool);
            ToolRegistry::register(&mut registry, WriteDummyTool);
            ToolRegistry::register(&mut registry, UnavailableDummyTool);
            ToolRegistry::register_with(&mut registry, DisabledDummyTool, false);
            assert!(
                REGISTRY.set(registry).is_ok(),
                "Test registry already initialized"
            );
        });
    }

    #[derive(Debug)]
    struct DummyTool;

    #[async_trait::async_trait]
    impl Tool for DummyTool {
        fn name(&self) -> &'static str {
            "dummy"
        }
        fn icon(&self) -> &'static str {
            "🛠️"
        }
        fn call_summary(&self, args: &serde_json::Value) -> String {
            args["query"].as_str().unwrap_or_default().to_string()
        }
        fn schema(&self) -> serde_json::Value {
            json!({
                "type": "function",
                "function": {
                    "name": self.name(),
                    "description": "A dummy read-only tool for registry tests.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string" }
                        },
                        "required": ["query"]
                    }
                }
            })
        }
        fn permission(&self) -> PermissionClass {
            PermissionClass::ReadOnly
        }
        fn permission_prompt(&self, _args: &serde_json::Value) -> (String, String) {
            ("dummy".to_string(), "dummy".to_string())
        }
        fn availability(&self) -> Result<(), String> {
            Ok(())
        }
        async fn execute(
            &self,
            _ctx: &ToolContext,
            args: serde_json::Value,
        ) -> (ToolStatus, serde_json::Value) {
            ok(json!({ "status": "success", "query": args["query"] }))
        }
    }

    #[derive(Debug)]
    struct WriteDummyTool;

    #[async_trait::async_trait]
    impl Tool for WriteDummyTool {
        fn name(&self) -> &'static str {
            "write_dummy"
        }
        fn icon(&self) -> &'static str {
            "✍️"
        }
        fn call_summary(&self, _args: &serde_json::Value) -> String {
            "write_dummy".to_string()
        }
        fn schema(&self) -> serde_json::Value {
            json!({
                "type": "function",
                "function": {
                    "name": self.name(),
                    "description": "A dummy write tool for registry tests.",
                    "parameters": {
                        "type": "object",
                        "properties": {}
                    }
                }
            })
        }
        fn permission(&self) -> PermissionClass {
            PermissionClass::Write
        }
        fn permission_prompt(&self, _args: &serde_json::Value) -> (String, String) {
            ("Write?".to_string(), "dummy write".to_string())
        }
        fn availability(&self) -> Result<(), String> {
            Ok(())
        }
        async fn execute(
            &self,
            _ctx: &ToolContext,
            _args: serde_json::Value,
        ) -> (ToolStatus, serde_json::Value) {
            ok(json!({ "status": "success", "executed": true }))
        }
    }

    #[derive(Debug)]
    struct DisabledDummyTool;

    #[async_trait::async_trait]
    impl Tool for DisabledDummyTool {
        fn name(&self) -> &'static str {
            "disabled_dummy"
        }
        fn icon(&self) -> &'static str {
            "🚫"
        }
        fn call_summary(&self, _args: &serde_json::Value) -> String {
            "disabled_dummy".to_string()
        }
        fn schema(&self) -> serde_json::Value {
            json!({
                "type": "function",
                "function": {
                    "name": self.name(),
                    "description": "A dummy tool registered as disabled by the user.",
                    "parameters": {
                        "type": "object",
                        "properties": {}
                    }
                }
            })
        }
        fn permission(&self) -> PermissionClass {
            PermissionClass::ReadOnly
        }
        fn permission_prompt(&self, _args: &serde_json::Value) -> (String, String) {
            ("dummy".to_string(), "dummy".to_string())
        }
        fn availability(&self) -> Result<(), String> {
            Ok(())
        }
        async fn execute(
            &self,
            _ctx: &ToolContext,
            _args: serde_json::Value,
        ) -> (ToolStatus, serde_json::Value) {
            ok(json!({ "status": "success" }))
        }
    }

    #[derive(Debug)]
    struct UnavailableDummyTool;

    #[async_trait::async_trait]
    impl Tool for UnavailableDummyTool {
        fn name(&self) -> &'static str {
            "unavailable_dummy"
        }
        fn icon(&self) -> &'static str {
            "⚠️"
        }
        fn call_summary(&self, _args: &serde_json::Value) -> String {
            "unavailable_dummy".to_string()
        }
        fn schema(&self) -> serde_json::Value {
            json!({
                "type": "function",
                "function": {
                    "name": self.name(),
                    "description": "A dummy tool whose dependency is missing.",
                    "parameters": {
                        "type": "object",
                        "properties": {}
                    }
                }
            })
        }
        fn permission(&self) -> PermissionClass {
            PermissionClass::ReadOnly
        }
        fn permission_prompt(&self, _args: &serde_json::Value) -> (String, String) {
            ("dummy".to_string(), "dummy".to_string())
        }
        fn availability(&self) -> Result<(), String> {
            Err("missing dependency: rg".to_string())
        }
        async fn execute(
            &self,
            _ctx: &ToolContext,
            _args: serde_json::Value,
        ) -> (ToolStatus, serde_json::Value) {
            ok(json!({ "status": "success" }))
        }
    }

    fn ctx(handle: &AgentHandle) -> ToolContext<'_> {
        ToolContext {
            handle,
            session_id: "test-session",
        }
    }

    #[tokio::test]
    async fn call_success_readonly() {
        init_test_registry();
        let (_rx, handle) = AgentHandle::new();
        let (status, payload) =
            ToolRegistry::call(&ctx(&handle), "dummy", r#"{"query":"hi"}"#).await;
        assert_eq!(status, ToolStatus::Success);
        assert_eq!(payload["query"], "hi");
    }

    #[tokio::test]
    async fn call_unknown_tool_returns_error() {
        init_test_registry();
        let (_rx, handle) = AgentHandle::new();
        let (status, payload) = ToolRegistry::call(&ctx(&handle), "does_not_exist", "{}").await;
        assert_eq!(
            status,
            ToolStatus::Error("Unregistered tool requested".to_string())
        );
        assert_eq!(payload["message"], "Unregistered tool requested");
    }

    #[tokio::test]
    async fn call_malformed_args_returns_error() {
        init_test_registry();
        let (_rx, handle) = AgentHandle::new();
        let (status, payload) = ToolRegistry::call(&ctx(&handle), "dummy", "not json").await;
        assert!(matches!(status, ToolStatus::Error(_)));
        assert_eq!(
            payload["message"],
            "Invalid tool arguments or malformed json"
        );
    }

    #[tokio::test]
    async fn write_tool_denied_when_receiver_dropped() {
        init_test_registry();
        // Dropping the receiver makes ask_permission fail (channel closed) -> denied.
        let (rx, handle) = AgentHandle::new();
        drop(rx);
        let (status, payload) = ToolRegistry::call(&ctx(&handle), "write_dummy", "{}").await;
        assert_eq!(status, ToolStatus::Denied);
        assert_eq!(payload["message"], "User declined");
        // The tool's own payload must never leak through for a denied call.
        assert_eq!(payload.get("executed"), None);
    }

    #[tokio::test]
    async fn write_tool_approved_runs_execute() {
        init_test_registry();
        let (mut rx, handle) = AgentHandle::new();
        // Respond "yes" to every permission request.
        let responder = tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let AgentEvent::PermissionRequest(request) = event {
                    let _ = request.response.send(true);
                }
            }
        });
        let (status, payload) = ToolRegistry::call(&ctx(&handle), "write_dummy", "{}").await;
        assert_eq!(status, ToolStatus::Success);
        // `executed: true` only appears if the tool's execute actually ran.
        assert_eq!(payload["executed"], true);
        responder.abort();
    }

    #[test]
    fn schema_includes_only_available_tools() {
        init_test_registry();
        let schema = ToolRegistry::schema();
        let names = schema
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["function"]["name"].as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        assert!(names.contains(&"dummy".to_string()));
        assert!(names.contains(&"write_dummy".to_string()));
        // A tool whose dependency is missing is excluded by the live probe.
        assert!(!names.contains(&"unavailable_dummy".to_string()));
    }

    #[test]
    fn disabled_tools_excluded_from_schema() {
        init_test_registry();
        let schema = ToolRegistry::schema();
        let names = schema
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["function"]["name"].as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        // A user-disabled tool's schema must never reach the LLM.
        assert!(!names.contains(&"disabled_dummy".to_string()));
    }

    #[tokio::test]
    async fn call_disabled_tool_returns_error() {
        init_test_registry();
        let (_rx, handle) = AgentHandle::new();
        let (status, payload) = ToolRegistry::call(&ctx(&handle), "disabled_dummy", "{}").await;
        assert_eq!(status, ToolStatus::Error("Tool is disabled".to_string()));
        assert_eq!(payload["message"], "Tool is disabled");
    }

    #[test]
    fn tool_icon_and_short() {
        init_test_registry();
        assert_eq!(ToolRegistry::tool_icon("dummy"), "🛠️");
        assert_eq!(ToolRegistry::tool_icon("nope"), "❓");
        assert_eq!(
            ToolRegistry::tool_short("dummy", r#"{"query":"hello"}"#),
            "hello"
        );
        assert_eq!(ToolRegistry::tool_short("dummy", "bad json"), "❓");
    }

    #[test]
    fn tools_status_probes_availability() {
        init_test_registry();
        let status = ToolRegistry::tools_status();
        let dummy = status.iter().find(|(name, _, _)| name == "dummy").unwrap();
        assert_eq!(dummy.1, true);
        assert_eq!(dummy.2, "");

        let unavailable = status
            .iter()
            .find(|(name, _, _)| name == "unavailable_dummy")
            .unwrap();
        assert_eq!(unavailable.1, false);
        assert_eq!(unavailable.2, "missing dependency: rg");
    }

    #[test]
    fn helpers_pair_status_with_payload() {
        let (status, _payload) = ok(json!({"status": "success"}));
        assert_eq!(status, ToolStatus::Success);

        let (status, payload) = err("boom");
        assert_eq!(status, ToolStatus::Error("boom".to_string()));
        assert_eq!(payload["message"], "boom");

        let (status, payload) = denied();
        assert_eq!(status, ToolStatus::Denied);
        assert_eq!(payload["message"], "User declined");
    }
}
