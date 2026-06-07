use anyhow::Result;

use crate::agent_tools::ToolRegistry;
use crate::api::{History, Message, completion};
use crate::config::AppConfig;

/// Takes in a message history that includes the next prompt from the user and returns
/// a new history that includes the assistant's response and any tools calls processed
pub async fn run_agent(
    history: History,
    on_message: impl Fn(&Message)
) -> Result<History> {
    let mut history = history;

    for iterations in 1..=AppConfig::global().agent.max_iterations {
        // Send a message if the agent does a lot of iterations
        if iterations >= 3 && (
                iterations % 10 == 0 ||
                iterations == 3 ||
                iterations == AppConfig::global().agent.max_iterations
            )
        {
            on_message(&Message::new(
                "assistant", 
                format!("🔁 Iteration {}/{}", iterations, AppConfig::global().agent.max_iterations)
            ));
        }


        // Get the next message from the assistant and append it to the history
        let assistant_msg = completion(&history).await?;
        on_message(&assistant_msg);
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