use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "cleanmac")]
#[command(about = "A CLI tool for cleaning macOS system", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Human
    }
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Scan for cleanable items")]
    Scan {
        #[arg(short, long, default_value = "all")]
        category: String,
        #[arg(short = 'F', long, default_value = "human")]
        format: OutputFormat,
        #[arg(short, long)]
        out: Option<String>,
        #[arg(short = 'M', long, help = "Collect Spotlight metadata (slower)")]
        metadata: bool,
    },
    #[command(about = "Create a cleanup plan from scan results")]
    Plan {
        #[arg(short, long)]
        from: Option<String>,
        #[arg(short, long)]
        category: Option<String>,
        #[arg(short = 'F', long, default_value = "human")]
        format: OutputFormat,
        #[arg(short, long)]
        out: Option<String>,
    },
    #[command(about = "Execute the cleanup plan")]
    Apply {
        #[arg(short, long)]
        plan: Option<String>,
        #[arg(short, long)]
        category: Option<String>,
        #[arg(long)]
        yes: bool,
        #[arg(short = 'F', long, default_value = "human")]
        format: OutputFormat,
        #[arg(short, long)]
        out: Option<String>,
    },
    #[command(about = "Generate a report from scan or execution results")]
    Report {
        #[arg(short, long)]
        from: String,
        #[arg(short = 'F', long, default_value = "md")]
        format: ReportFormat,
        #[arg(short, long)]
        out: Option<String>,
    },
    #[command(about = "Clean scanned items (legacy, use 'apply')")]
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
    #[command(about = "Visualize disk usage (TUI)")]
    Space {
        #[arg(short, long)]
        path: Option<String>,
        #[arg(short = 'S', long)]
        single: bool,
        #[arg(short = 't', long, default_value = "4")]
        threads: usize,
    },
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
    #[command(about = "Run as MCP server (for AI integration)")]
    Mcp,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum ReportFormat {
    Json,
    Md,
    Txt,
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
