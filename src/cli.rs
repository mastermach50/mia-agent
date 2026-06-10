use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "Mia Agent", version, about = "Your personal agent")]
pub struct Cli {
    #[command(subcommand)]
    pub sub_command: Option<SubCommands>,

    /// Run a command/prompt without entering into the tui
    #[arg(short, long)]
    pub command: Option<String>,
}

#[derive(Subcommand)]
pub enum SubCommands {
    /// Start the messaging gateway
    // Gateway,

    /// Run in interactive mode
    Tui {

        /// Start a new session
        #[arg(short, long)]
        new: bool,
    },

    /// List all agent tools and their status
    Tools,
}