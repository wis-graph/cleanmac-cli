use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cleanx")]
#[command(about = "A CLI tool for cleaning macOS system", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Scan for cleanable items")]
    Scan {
        #[arg(short, long, default_value = "all")]
        category: String,
    },
    #[command(about = "Clean scanned items")]
    Clean {
        #[arg(short, long, default_value = "all")]
        category: String,
        #[arg(long)]
        execute: bool,
    },
    #[command(about = "Uninstall an application completely")]
    Uninstall {
        #[arg(short, long)]
        name: String,
        #[arg(long)]
        execute: bool,
    },
    #[command(about = "Browse and uninstall apps (TUI)")]
    Apps,
    #[command(about = "Manage configuration")]
    Config {
        #[command(subcommand)]
        action: ConfigActions,
    },
    #[command(about = "View deletion history")]
    History {
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
}

#[derive(Subcommand)]
pub enum ConfigActions {
    #[command(about = "Show current configuration")]
    Show,
    #[command(about = "Set a configuration value")]
    Set {
        #[arg(short, long)]
        key: String,
        #[arg(short, long)]
        value: String,
    },
    #[command(about = "Add excluded path")]
    AddExclude {
        #[arg(short, long)]
        path: String,
    },
}

impl Cli {
    pub fn parse_args() -> Self {
        Parser::parse()
    }
}
