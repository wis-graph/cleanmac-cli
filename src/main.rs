mod cleaner;
mod cli;
mod config;
mod history;
mod plugin;
mod safety;
mod scanner;
mod tui;
mod uninstaller;
mod utils;

use anyhow::Result;
use cleaner::DefaultCleaner;
use cli::{Cli, Commands, ConfigActions};
use config::Config;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use history::HistoryLogger;
use plugin::{CleanConfig, Cleaner, PluginRegistry, ScanConfig};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use tui::App;
use utils::format_size;

fn main() -> Result<()> {
    let cli = Cli::parse_args();
    let config = Config::load()?;

    match cli.command {
        None => run_tui(config)?,
        Some(Commands::Scan { category }) => run_scan(&category, &config)?,
        Some(Commands::Clean { category, execute }) => run_clean(&category, execute, &config)?,
        Some(Commands::Uninstall { name, execute }) => run_uninstall(&name, execute)?,
        Some(Commands::Apps) => run_apps_tui()?,
        Some(Commands::Config { action }) => run_config(action, config)?,
        Some(Commands::History { limit }) => run_history(limit)?,
    }

    Ok(())
}

fn run_tui(config: Config) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(config);
    let result = app.run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_apps_tui() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new_apps_mode();
    let result = app.run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_scan(category: &str, config: &Config) -> Result<()> {
    println!("Scanning {}...\n", category);

    let registry = PluginRegistry::default();
    let scan_config = ScanConfig {
        min_size: config.scan.min_size_bytes,
        max_depth: config.scan.max_depth,
        excluded_paths: config
            .scan
            .excluded_paths
            .iter()
            .map(|s| s.into())
            .collect(),
        follow_symlinks: config.scan.follow_symlinks,
        progress_callback: None,
    };

    let report = registry.scan_all(&scan_config)?;

    for cat_result in &report.categories {
        if category != "all" && !cat_result.scanner_id.contains(&category.to_lowercase()) {
            continue;
        }

        println!("{}:", cat_result.name);
        println!("  Items: {}", cat_result.items.len());
        println!("  Size: {} bytes", cat_result.total_size());
        println!("  Files: {}", cat_result.total_files());
        println!();

        for item in cat_result.items.iter().take(10) {
            println!("  - {} ({})", item.path.display(), format_size(item.size));
        }

        if cat_result.items.len() > 10 {
            println!("  ... and {} more", cat_result.items.len() - 10);
        }
        println!();
    }

    println!(
        "Total: {} items, {} (in {:?})",
        report.total_items,
        format_size(report.total_size),
        report.duration
    );

    Ok(())
}

fn run_clean(category: &str, execute: bool, config: &Config) -> Result<()> {
    let registry = PluginRegistry::default();
    let cleaner = DefaultCleaner::new();

    println!("{} mode\n", if execute { "Execute" } else { "Dry-run" });

    let scan_config = ScanConfig {
        min_size: config.scan.min_size_bytes,
        max_depth: config.scan.max_depth,
        excluded_paths: config
            .scan
            .excluded_paths
            .iter()
            .map(|s| s.into())
            .collect(),
        follow_symlinks: config.scan.follow_symlinks,
        progress_callback: None,
    };

    let report = registry.scan_all(&scan_config)?;

    let mut all_items = Vec::new();
    for cat_result in &report.categories {
        if category != "all" && !cat_result.scanner_id.contains(&category.to_lowercase()) {
            continue;
        }
        all_items.extend(cat_result.items.clone());
    }

    let clean_config = CleanConfig {
        dry_run: !execute,
        log_history: config.clean.log_history,
    };

    let result = cleaner.clean(&all_items, &clean_config)?;

    println!();
    println!("Results:");
    println!("  Cleaned: {} items", result.success_count);
    println!("  Failed: {} items", result.failed_count);
    println!("  Freed: {}", format_size(result.total_freed));
    println!("  Duration: {:?}", result.duration);

    if !result.failed_items.is_empty() {
        println!("\nFailed items:");
        for (path, error) in &result.failed_items {
            println!("  - {}: {}", path.display(), error);
        }
    }

    Ok(())
}

fn run_uninstall(name: &str, execute: bool) -> Result<()> {
    use uninstaller::{AppDetector, RelatedFileDetector, Uninstaller};

    let detector = AppDetector::new();
    let uninstaller = Uninstaller::new(!execute);

    println!("Searching for app: {}\n", name);

    match detector.find_by_name(name) {
        Some(app) => {
            println!("Found: {} ({})", app.name(), app.path.display());
            if let Some(info) = app.info() {
                println!("  Bundle ID: {}", info.bundle_id);
                println!("  Version: {}", info.version);
            }
            println!("  Size: {}", format_size(app.size()));

            println!("\nSearching for related files...");
            let related_detector = RelatedFileDetector::new();
            let related_files = related_detector.find_related_files(&app);

            if related_files.is_empty() {
                println!("No related files found.");
            } else {
                println!("Related files ({}):", related_files.len());
                for file in &related_files {
                    let protected = if file.category.is_protected() {
                        " (Protected)"
                    } else {
                        ""
                    };
                    println!(
                        "  - {} [{}] {}{}",
                        file.path.display(),
                        file.category.display_name(),
                        format_size(file.size),
                        protected
                    );
                }
            }

            println!();
            let result = uninstaller.uninstall(&app, &related_files)?;

            println!("\nResults:");
            if result.deleted_app {
                println!("  App deleted: Yes");
            }
            println!("  Related deleted: {} items", result.deleted_related.len());
            println!("  Skipped (protected): {} items", result.skipped.len());
            println!("  Errors: {} items", result.errors.len());
            println!("  Freed: {}", format_size(result.total_freed));

            if !result.errors.is_empty() {
                println!("\nErrors:");
                for error in &result.errors {
                    println!("  - {}", error);
                }
            }
        }
        None => {
            println!("App not found: {}", name);
        }
    }

    Ok(())
}

fn run_config(action: ConfigActions, mut config: Config) -> Result<()> {
    match action {
        ConfigActions::Show => {
            println!("Current configuration:");
            println!("  Min size: {}", format_size(config.scan.min_size_bytes));
            println!("  Max depth: {}", config.scan.max_depth);
            println!("  Excluded paths:");
            for path in &config.scan.excluded_paths {
                println!("    - {}", path);
            }
            println!("  Dry run by default: {}", config.clean.dry_run_by_default);
            println!("  Log history: {}", config.clean.log_history);
        }
        ConfigActions::Set { key, value } => match key.as_str() {
            "min_size" => {
                config.scan.min_size_bytes = value.parse()?;
                config.save()?;
                println!("Set min_size to {}", value);
            }
            "max_depth" => {
                config.scan.max_depth = value.parse()?;
                config.save()?;
                println!("Set max_depth to {}", value);
            }
            _ => {
                println!("Unknown key: {}", key);
                println!("Available keys: min_size, max_depth");
            }
        },
        ConfigActions::AddExclude { path } => {
            config.add_excluded_path(path.clone());
            config.save()?;
            println!("Added exclusion: {}", path);
        }
    }

    Ok(())
}

fn run_history(limit: usize) -> Result<()> {
    let logger = HistoryLogger::new();
    let entries = logger.read_history(Some(limit))?;

    if entries.is_empty() {
        println!("No history found.");
        return Ok(());
    }

    println!("Last {} deletion(s):\n", entries.len());

    for entry in entries {
        println!(
            "{} {} {}",
            entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
            entry.action,
            entry.path.display()
        );
        if let Some(size) = entry.size {
            println!("    Size: {}", format_size(size));
        }
    }

    Ok(())
}
