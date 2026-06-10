use anyhow::Result;
use tokio_util::sync::CancellationToken;

use crate::agent_tools::ToolRegistry;
use crate::api::{History, Message, completion};
use crate::config::AppConfig;

/// Takes in a message history that includes the next prompt from the user and returns
/// a new history that includes the assistant's response and any tools calls processed
pub async fn run_agent(
    history: History,
    on_assistant_message: impl Fn(&Message),
    on_assistant_thinking: impl Fn(),
    on_system_message: impl Fn(&str),
) -> Result<History> {
    let mut history = history;

    // Setup a Ctrl-C listener to cancel the request
    // When a Ctrl-C is received the cancellation token is set to "cancelled"
    let cancel = CancellationToken::new();
    let cancel_watcher = cancel.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl-C");
        // Extra spaces to wipe out any thinking message
        println!("^C                       ");
        cancel_watcher.cancel();
    });

    for iterations in 1..=AppConfig::global().agent.max_iterations {

        // Check if the request is cancelled
        if cancel.is_cancelled() {
            break;
        }

        // Send a message if the agent does a lot of iterations
        if iterations >= 3 && (
                iterations % 10 == 0 ||
                iterations == 3 ||
                iterations == AppConfig::global().agent.max_iterations
            )
        {
            on_assistant_message(&Message::new(
                "assistant", 
                format!("🔁 Iteration {}/{}", iterations, AppConfig::global().agent.max_iterations)
            ));
        }

        // Notify the user that the agent is (about to start) thinking
        on_assistant_thinking();

        // Get the next message from the assistant and append it to the history
        let assistant_msg = match completion(&history, &cancel).await {
            // Success
            Ok(message) => { message }

            // Cancelled
            Err(_) if cancel.is_cancelled() => {
                on_system_message("Assistant turn cancelled.");
                break;
            }

            // Errored
            Err(err) => {
                on_system_message(&format!("Assistant returned error:\n\t{err}"));
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
                let content = ToolRegistry::call(&tool_call.function.name, &tool_call.function.arguments);
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