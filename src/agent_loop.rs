use anyhow::Result;
use tokio_util::sync::CancellationToken;

use crate::agent_tools::ToolRegistry;
use crate::api::{History, Message, PartialMessage, completion};
use crate::config::AppConfig;

/// Takes in a message history that includes the next prompt from the user and returns
/// a new history that includes the assistant's response and any tools calls processed
pub async fn run_agent(
    history: History,
    session_id: &str,
    stream: bool,
    on_assistant_message: impl Fn(&Message),
    on_partial_assistant_message: impl Fn(&PartialMessage),
    on_assistant_status_update: impl Fn(&str),
    on_system_message: impl Fn(&str),
) -> Result<History> {
    // Make history mutable
    let mut history = history;

    // Setup a Ctrl-C listener to cancel the request
    // When a Ctrl-C is received the cancellation token is set to "cancelled"
    let cancel = CancellationToken::new();

    // Max number of iterations is configurable
    for iterations in 1..=AppConfig::global().agent.max_iterations {
        // Initially mark the assistant as waiting
        on_assistant_status_update("Waiting");

        // Check if the request is cancelled
        if cancel.is_cancelled() {
            break;
        }

        // Send a message if the agent does a lot of iterations
        if iterations >= 3
            && (iterations % 10 == 0
                || iterations == 3
                || iterations == AppConfig::global().agent.max_iterations)
        {
            on_assistant_message(&Message::new(
                "assistant",
                format!(
                    "🔁 Iteration {}/{}",
                    iterations,
                    AppConfig::global().agent.max_iterations
                ),
            ));
        }

        // Get the next message from the assistant and append it to the history
        // Pass over the cancellation token and thinking notifier too
        // Also accept Ctrl-C signal and break out of loop if it arises
        let assistant_msg = tokio::select! {
            res = completion(
                &history,
                &session_id,
                stream,
                &cancel,
                &on_assistant_status_update,
                &on_partial_assistant_message
            ) => {
                match res {
                    Ok(message) => message,
                    Err(err) => {
                        on_system_message(&format!("Assistant returned error:\n\t{err}"));
                        break;
                    }
                }
            },
            _ = tokio::signal::ctrl_c() => {
                on_system_message("Assistant turn cancelled.");
                cancel.cancel();
                break;
            }
        };

        // Forward the assistant's message
        on_assistant_message(&assistant_msg);

        // Append the assistant's message to the history
        history.add_message(assistant_msg.clone());

        // If the assistant requested tool calls then do the tool calls
        // Append the result of the tool calls to the history and continue the loop
        if let Some(tool_calls) = assistant_msg.tool_calls {
            for tool_call in tool_calls {
                let tool_name = tool_call.function.name.clone();
                let tool_args = tool_call.function.arguments.clone();
                let content = tokio::select! {
                    content = ToolRegistry::call(&tool_name, &tool_args) => {
                        content
                    },
                    _ = tokio::signal::ctrl_c() => {
                        on_system_message("Assistant turn cancelled during tool call.");
                        cancel.cancel();
                        break;
                    }
                };
                history.add_message(Message::new_tool_call_response(
                    tool_call.id.clone(),
                    content.to_string(),
                ));
            }
            continue;
        }

        // If the assistant did not request any more tool calls then break the loop
        break;
    }

    // Return the updated history
    Ok(history)
}
