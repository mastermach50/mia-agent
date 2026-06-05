use anyhow::Result;

use crate::agent_tools::execute_tools;
use crate::api::{History, Message, completion};

/// Takes in a message history that includes the next prompt from the user and returns
/// a new history that includes the assistant's response and any tools calls processed
pub async fn run_agent(
    history: History,
    intermediate_message_proxy: fn(&Message),
) -> Result<History> {
    let mut history = history;

    loop {
        // Get the next message from the assistant and append it to the history
        let assistant_msg = completion(&history).await?;
        intermediate_message_proxy(&assistant_msg);
        history.add_message(assistant_msg.clone());

        // If the assistant requested tool calls then do the tool calls
        // Append the result of the tool calls to the history and continue the loop
        if assistant_msg.tool_calls.is_some() {
            for tool_call in assistant_msg.tool_calls.unwrap() {
                let content = execute_tools(&tool_call.function.name, tool_call.function.arguments)?;
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