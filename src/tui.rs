use std::{fs, io::Write};
use anyhow::Result;

use crate::{api::{Message, completion}, config::AppConfig, utils::generate_think_lines};

pub async fn run() -> Result<()> {

    let mut history: Vec<Message> = Vec::new();
    let soul = fs::read_to_string(&AppConfig::global().documents.soul)?;
    history.push(Message {
        role: "system".to_string(),
        content: soul,
        reasoning: None
    });
    history.push(Message {
        role: "system".to_string(),
        content: format!("You are talking to {} via a CLI", AppConfig::global().cli.username),
        reasoning: None
    });
    
    loop {
        print!("User > ");
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        input = input.trim().to_string();

        if input == "/exit"  {
            break;
        }

        history.push(Message {
            role: "user".to_string(),
            content: input.clone(),
            reasoning: None
        });

        print!("Mia  > Thinking...");
        std::io::stdout().flush()?;

        let response = completion(&history).await?;
        if let Some(reasoning) = response.reasoning.clone() {
            println!("\rMia  > 💭       ");
            println!("{}", generate_think_lines(reasoning.trim()))
        }
        println!("\rMia  > {}", response.content.trim());

        history.push(response);
    }
    Ok(())
}