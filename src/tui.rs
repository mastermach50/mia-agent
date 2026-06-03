use std::io::Write;

use anyhow::Result;


pub fn run() -> Result<()> {
    
    loop {
        print!("User > ");
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        input = input.trim().to_string();

        if input == "/exit"  {
            break;
        }
        
        println!("Mia  > {}", input);

    }
    Ok(())
}