use std::io::Write;
use anyhow::Result;

pub fn run() -> Result<()> {
    loop {
        print!("> ");
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim();
        if input == "exit" {
            break;
        }
        println!("Mia> echo: {}", input);
    }
    Ok(())
}