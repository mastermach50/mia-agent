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
    /// Model commands
    #[command(visible_alias = "models")]
    Model {
        #[command(subcommand)]
        sub_command: Option<ModelSubCommands>,
    },

    /// Manage sessions
    #[command(visible_alias = "sessions")]
    Session {
        #[command(subcommand)]
        sub_command: Option<SessionSubCommands>,
    },

    /// Setup the agent
    Setup,

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
    List(ModelListArgs),

    /// Show the current model info
    Show,
}

#[derive(Parser)]
pub struct ModelListArgs {
    // Maximum price per million completion tokens
    #[arg(long, conflicts_with = "free")]
    pub max_price: Option<f64>,

    // Show only free models
    #[arg(long, conflicts_with = "max_price")]
    pub free: bool,

    // Minimum context length
    #[arg(long)]
    pub min_context: Option<String>,
}

#[derive(Subcommand)]
pub enum SessionSubCommands {
    /// List all the sessions
    List,
}
