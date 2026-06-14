use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "Mia Agent", version, about = "Your personal agent")]
pub struct Cli {
    #[command(subcommand)]
    pub sub_command: Option<MainSubCommands>,

    /// Run a command/prompt without entering into the tui
    #[arg(short, long)]
    pub command: Option<String>,
}

#[derive(Subcommand)]
pub enum MainSubCommands {
    /// Start the messaging gateway
    // Gateway,

    /// Model commands
    Model {
        /// List all available models
        #[command(subcommand)]
        sub_command: Option<ModelSubCommands>,
    },

    /// List all agent tools and their status
    Tools,

    /// Run in interactive mode
    Tui {
        /// Start a new session
        #[arg(short, long)]
        new: bool,
    },
}

#[derive(Subcommand)]
pub enum ModelSubCommands {
    /// List all the models available on the server
    List {
        // Maximum price per million completion tokens
        #[arg(long)]
        max_price: Option<f64>,

        // Minimum context length
        #[arg(long)]
        min_context: Option<String>,
    },

    /// Show the current model info
    Show,
}
