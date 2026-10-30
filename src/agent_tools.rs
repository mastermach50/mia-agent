use log::{trace, warn};
use serde_json::{self, json};
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use crate::agent_loop::AgentHandle;

pub mod common;
mod datetime;
mod doc_convert;
mod doc_create;
mod exec_python;
mod exec_shell;
mod fs_edit_file;
mod fs_file_ops;
mod fs_list_dir;
mod fs_read_file;
mod fs_search_dirs;
mod fs_search_files;
mod fs_tree;
mod fs_write_file;
mod memory;
mod notify;
mod web_extract;
mod web_search;

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
        Self::register(&mut registry, datetime::DateTime);
        Self::register(&mut registry, fs_read_file::FSReadFile);
        Self::register(&mut registry, fs_write_file::FSWriteFile);
        Self::register(&mut registry, fs_search_files::FSSearchFiles);
        Self::register(&mut registry, fs_search_dirs::FSSearchDirs);
        Self::register(&mut registry, fs_tree::FSTree);
        Self::register(&mut registry, fs_list_dir::FSListDir);
        Self::register(&mut registry, fs_edit_file::FSEditFile);
        Self::register(&mut registry, fs_file_ops::FSFileOps);
        Self::register(&mut registry, exec_shell::ExecShell);
        Self::register(&mut registry, exec_python::ExecPython);
        Self::register(&mut registry, notify::Notify);
        Self::register(&mut registry, doc_create::DocCreate);
        Self::register(&mut registry, doc_convert::DocConvert);
        Self::register(&mut registry, web_search::WebSearch);
        Self::register(&mut registry, web_extract::WebExtract);
        Self::register(&mut registry, memory::Memory);
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

        trace!("Calling tool {name} in session {}", ctx.session_id);

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