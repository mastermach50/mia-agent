use anyhow::Result;
use log::trace;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::agent_tools::ToolRegistry;
use crate::api::{Completion, History, Message, PartialMessage, completion};
use crate::config::AppConfig;

#[derive(Clone)]
pub struct AgentHandle {
    tx: UnboundedSender<AgentEvent>,
    pub cancel: CancellationToken,
}

pub enum AgentEvent {
    AssistantMessage(Message),
    PartialAssistantMessage(PartialMessage),
    AssistantStatusUpdate(String),
    ToolResponseMessage(Message),
    HarnessMessage(String),
    HistoryUpdate(History),
    PermissionRequest {
        header: String,
        content: String,
        response: oneshot::Sender<bool>,
    },
    PartialToolOutput {
        stdout: Option<String>,
        stderr: Option<String>,
    },
    ToolOutput {
        stdout: String,
        stderr: String,
    },
}

impl AgentHandle {
    pub fn new() -> (UnboundedReceiver<AgentEvent>, Self) {
        let (tx, rx) = mpsc::unbounded_channel::<AgentEvent>();
        let cancel = CancellationToken::new();

        trace!("Agent handle created");

        (rx, AgentHandle { tx, cancel })
    }

    pub fn reset_cancellation(&mut self) {
        self.cancel = CancellationToken::new();

        trace!("Agent cancellation token reset");
    }

    fn assistant_msg(&self, msg: &Message) {
        self.tx
            .send(AgentEvent::AssistantMessage(msg.clone()))
            .unwrap();

        trace!("Assistant message sent");
    }

    fn partial_assistant_msg(&self, msg: &PartialMessage) {
        self.tx
            .send(AgentEvent::PartialAssistantMessage(msg.clone()))
            .unwrap();

        // trace!("Partial assistant message sent")
    }

    fn assistant_status_update(&self, msg: impl ToString) {
        self.tx
            .send(AgentEvent::AssistantStatusUpdate(msg.to_string()))
            .unwrap();

        trace!("Assistant status update sent ({})", msg.to_string());
    }

    fn tool_response_msg(&self, msg: &Message) {
        self.tx
            .send(AgentEvent::ToolResponseMessage(msg.clone()))
            .unwrap();

        trace!("Tool call response message sent");
    }

    fn harness_msg(&self, msg: impl ToString) {
        self.tx
            .send(AgentEvent::HarnessMessage(msg.to_string()))
            .unwrap();

        trace!("Harness message sent ({})", msg.to_string());
    }

    fn update_history(&self, history: History) {
        self.tx.send(AgentEvent::HistoryUpdate(history)).unwrap();

        trace!("History update message sent");
    }

    pub async fn ask_permission(
        &self,
        header: impl Into<String>,
        content: impl Into<String>,
    ) -> bool {
        let (respond, rx) = oneshot::channel();
        let sent = self.tx.send(AgentEvent::PermissionRequest {
            header: header.into(),
            content: content.into(),
            response: respond,
        });

        if sent.is_err() {
            return false;
        }

        rx.await.unwrap_or(false)
    }

    pub fn partial_tool_output(&self, stdout: Option<String>, stderr: Option<String>) {
        self.tx
            .send(AgentEvent::PartialToolOutput { stdout, stderr })
            .unwrap();

        trace!("Partial tool output sent");
    }

    pub fn tool_output(&self, stdout: impl ToString, stderr: impl ToString) {
        let stdout = stdout.to_string();
        let stderr = stderr.to_string();

        self.tx
            .send(AgentEvent::ToolOutput { stdout, stderr })
            .unwrap();

        trace!("Tool output sent");
    }
}

/// Takes in a message history that includes the next prompt from the user and returns
/// a new history that includes the assistant's response and any tools calls processed
pub async fn run_agent(
    history: History,
    session_id: &str,
    stream: bool,
    handle: AgentHandle,
) -> Result<()> {
    // Make history mutable
    let mut history = history;

    // Max number of iterations is configurable
    for iterations in 1..=AppConfig::global().agent.max_iterations {
        // Initially mark the assistant as waiting
        handle.assistant_status_update("Waiting");

        // Check if the request is cancelled
        if handle.cancel.is_cancelled() {
            break;
        }

        // Send a message if the agent does a lot of iterations
        if iterations >= 3
            && (iterations % 10 == 0
                || iterations == 3
                || iterations == AppConfig::global().agent.max_iterations)
        {
            handle.harness_msg(format!(
                "🔁 Iteration {}/{}",
                iterations,
                AppConfig::global().agent.max_iterations
            ));
            handle.assistant_status_update("Waiting");
        }

        // Get the next message from the assistant and append it to the history
        // Pass over the cancellation token and thinking notifier too
        // Also accept Ctrl-C signal and break out of loop if it arises
        let assistant_msg = match completion(
            &history,
            session_id,
            stream,
            &handle.cancel,
            |kind: &str| handle.assistant_status_update(kind),
            |msg: &PartialMessage| handle.partial_assistant_msg(msg),
        )
        .await
        {
            Ok(completion) => {
                if let Completion::Completed(msg) = completion {
                    msg
                } else {
                    handle.harness_msg("Assistant turn cancelled.");
                    break;
                }
            }
            Err(err) => {
                handle.harness_msg(format!("Assistant returned error:\n\t{err}"));
                break;
            }
        };

        // Forward the assistant's message
        handle.assistant_msg(&assistant_msg);

        // Append the assistant's message to the history
        history.add_message(assistant_msg.clone());

        // If the assistant requested tool calls then do the tool calls
        // Append the result of the tool calls to the history and continue the loop
        if let Some(tool_calls) = assistant_msg.tool_calls {
            for tool_call in tool_calls {
                handle.assistant_status_update("Calling Tools");
                let tool_name = tool_call.function.name.clone();
                let tool_args = tool_call.function.arguments.clone();
                let content = tokio::select! {
                    content = ToolRegistry::call(
                        &handle,
                        &tool_name,
                        &tool_args
                    ) => {
                        content
                    },
                    _ = handle.cancel.cancelled() => {
                        handle.harness_msg("Assistant turn cancelled during tool call.");
                        break;
                    }
                };
                let tc_response =
                    Message::new_tool_call_response(tool_call.id.clone(), content.to_string());
                handle.tool_response_msg(&tc_response);
                history.add_message(tc_response);
            }
            continue;
        }

        // If the assistant did not request any more tool calls then break the loop
        break;
    }

    // Return the updated history
    handle.update_history(history);

    Ok(())
}
