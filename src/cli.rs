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

    /// Setup the agent (defaults to full setup)
    Setup(SetupArgs),

    /// List all agent tools and their status
    Tools,

    /// Run the interactive tui
    Tui {
        /// Start a new session
        #[arg(short, long)]
        new: bool,
    },

    /// Run the old, direct stdio tui
    #[command(hide(true))]
    OldTui {
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
    /// Maximum price per million completion tokens
    #[arg(long, conflicts_with = "free")]
    pub max_price: Option<f64>,

    /// Show only free models
    #[arg(long, conflicts_with = "max_price")]
    pub free: bool,

    /// Minimum context length
    #[arg(long)]
    pub min_context: Option<String>,
}

#[derive(Subcommand)]
pub enum SessionSubCommands {
    /// List all the sessions
    List,

    /// Clear all sessions
    Clear,
}

#[derive(Parser)]
pub struct SetupArgs {
    /// Setup the model only
    #[arg(short, long)]
    pub model: bool,

    /// Setup the TUI only
    #[arg(short, long)]
    pub tui: bool,

    /// Setup the agent only
    #[arg(short, long)]
    pub agent: bool,
}