use std::io::Write;

use anyhow::Result;
use crossterm::style::Stylize;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::{
    agent_loop::{self, AgentEvent, AgentHandle},
    agent_tools::ToolRegistry,
    api::Message,
    config::AppConfig,
    sessions::Session,
    system_prompt,
};

pub async fn run() -> Result<()> {
    let mut session = Session::new(&AppConfig::global().tui.username, "debug-chat", "nil");
    session
        .history
        .set_system_prompt(system_prompt::get_system_prompt()?);

    println!("{}", AppConfig::global().model.name);
    println!("{}", AppConfig::global().model.provider);
    println!("{}", AppConfig::global().model.base_url);

    let mut query = String::new();

    loop {
        print!("\n> ");
        std::io::stdout().flush()?;
        std::io::stdin().read_line(&mut query)?;

        query = query.trim().to_owned();

        if &query == "exit" {
            break;
        }

        session.history.add_message(Message::new("user", &query));

        let (mut event_rx, handle) = AgentHandle::new();
        let history = session.history.clone();
        let passed_handle = handle.clone();
        let session_id = session.get_extended_session_id();
        tokio::spawn(async move {
            agent_loop::run_agent(history, &session_id, true, passed_handle)
                .await
                .unwrap();
        });

        handle_agent_events(&mut event_rx, &mut session).await?;
    }

    Ok(())
}

async fn handle_agent_events(
    event_rx: &mut UnboundedReceiver<AgentEvent>,
    session: &mut Session,
) -> Result<()> {
    while !event_rx.is_closed() {
        match event_rx.recv().await.unwrap() {
            AgentEvent::AssistantMessage(message) => {
                if let Some(tool_calls) = message.tool_calls {
                    for tool_call in tool_calls {
                        println!(
                            "[ {} {} ]",
                            tool_call.function.name,
                            ToolRegistry::tool_short(
                                &tool_call.function.name,
                                &tool_call.function.arguments
                            )
                        )
                    }
                }
            }
            AgentEvent::HarnessMessage(message) => {
                println!("{} {}", "Harness >".green(), message);
            }
            AgentEvent::AssistantStatusUpdate(status) => {
                println!("{} [{}]", "Mia".cyan(), status);
            }
            AgentEvent::PartialAssistantMessage(message) => {
                if let Some(reasoning) = message.reasoning
                    && !reasoning.is_empty() {
                    if message.reasoning_chunk_index == 0 {
                        println!("{} {}", "Mia >".cyan(), "Thinking...".italic().grey())
                    }
                    print!("{}", reasoning.italic().grey())
                }

                if let Some(content) = message.content
                    && !content.is_empty()
                {
                    if message.reasoning_chunk_index > -1 && message.content_chunk_index == 0 {
                        println!("\n{}", "...End of reasoning".italic().grey())
                    }
                    if message.content_chunk_index == 0 {
                        print!("{} ", "Mia >".cyan())
                    }
                    print!("{}", content);
                }
            }
            AgentEvent::PartialToolOutput { stdout, stderr } => {
                if let Some(stdout) = stdout {
                    println!("{}", stdout);
                }
                if let Some(stderr) = stderr {
                    println!("{}", stderr);
                }
            }
            #[allow(unused_variables)]
            AgentEvent::ToolOutput { stdout, stderr } => {}
            AgentEvent::PermissionRequest(perm) => {
                println!("{}", perm.header);
                println!("{}", perm.content);
                print!("y/n > ");
                std::io::stdout().flush()?;
                let mut inp = String::new();
                std::io::stdin().read_line(&mut inp)?;
                if inp.trim().starts_with("y") {
                    perm.response.send(true).unwrap();
                } else {
                    perm.response.send(false).unwrap();
                }
            }
            AgentEvent::ToolResponseMessage(_message) => {}
            AgentEvent::HistoryUpdate(new_history) => {
                session.history = new_history;
                event_rx.close();
            }
        }
    }

    Ok(())
}
