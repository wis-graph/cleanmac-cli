mod cleaner;
mod cli;
mod config;
mod history;
mod mcp;
mod metadata;
mod output;
mod plugin;
mod safety;
mod scanner;
mod tui;
mod uninstaller;
mod utils;

use anyhow::Result;
use chrono::Utc;
use cleaner::DefaultCleaner;
use cli::{Cli, Commands, ConfigActions, OutputFormat, ReportFormat};
use config::Config;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use history::HistoryLogger;
use output::{
    CategoryExecutionResult, CategoryPlanResult, CategoryScanResult as JsonCategoryScanResult,
    ExecutionResult, ExecutionStatus, FailedItem, PlanItem, PlanResult, ScanItem,
    ScanResult as JsonScanResult,
};
use plugin::{CleanConfig, Cleaner, PluginRegistry, ScanConfig};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::process::ExitCode;
use std::time::Instant;
use tui::App;
use utils::format_size;

fn main() -> ExitCode {
    let cli = Cli::parse_args();

    let result = match Config::load() {
        Ok(config) => run(cli, config),
        Err(e) => Err(e),
    };

    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli, config: Config) -> Result<ExitCode> {
    match cli.command {
        None => run_tui(config)?,
        Some(Commands::Scan {
            category,
            format,
            out,
            metadata,
        }) => run_scan(&category, &config, format, out.as_deref(), metadata)?,
        Some(Commands::Plan {
            from,
            category,
            format,
            out,
        }) => run_plan(from.as_deref(), category.as_deref(), format, out.as_deref())?,
        Some(Commands::Apply {
            plan,
            category,
            yes,
            format,
            out,
        }) => run_apply(
            plan.as_deref(),
            category.as_deref(),
            yes,
            &config,
            format,
            out.as_deref(),
        )?,
        Some(Commands::Report { from, format, out }) => run_report(&from, format, out.as_deref())?,
        Some(Commands::Clean { category, execute }) => run_clean(&category, execute, &config)?,
        Some(Commands::Uninstall { name, execute }) => run_uninstall(&name, execute)?,
        Some(Commands::Apps) => run_apps_tui()?,
        Some(Commands::Space {
            path,
            single,
            threads,
        }) => run_space_tui(path.as_deref(), single, threads)?,
        Some(Commands::Config { action }) => run_config(action, config)?,
        Some(Commands::History { limit }) => run_history(limit)?,
        Some(Commands::Mcp) => {
            tokio::runtime::Runtime::new()
                .map_err(|e| anyhow::anyhow!("Failed to create tokio runtime: {}", e))?
                .block_on(mcp::run_mcp_server())?;
        }
    }

    Ok(ExitCode::SUCCESS)
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

fn run_space_tui(path: Option<&str>, single: bool, threads: usize) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new_space_lens_mode(path);
    app.space_lens.parallel_scan = !single;
    app.space_lens.thread_count = threads.max(1);
    let result = app.run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_scan(
    category: &str,
    config: &Config,
    format: OutputFormat,
    out: Option<&str>,
    collect_metadata: bool,
) -> Result<()> {
    let start = Instant::now();

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
        progress_callback: None,
        item_callback: None,
    };

    let report = registry.scan_all(&scan_config)?;

    let categories: Vec<JsonCategoryScanResult> = report
        .categories
        .iter()
        .filter(|cat_result| {
            category == "all" || cat_result.scanner_id.contains(&category.to_lowercase())
        })
        .map(|cat_result| {
            let items: Vec<ScanItem> = cat_result
                .items
                .iter()
                .map(|item| {
                    let (last_used, use_count) = if collect_metadata {
                        match metadata::get_file_metadata(&item.path) {
                            Some(meta) => (meta.last_used, meta.use_count),
                            None => (None, None),
                        }
                    } else {
                        (None, None)
                    };

                    ScanItem {
                        path: item.path.clone(),
                        size_bytes: item.size,
                        modified: item.last_modified.unwrap_or_else(Utc::now),
                        last_used,
                        use_count,
                    }
                })
                .collect();

            JsonCategoryScanResult {
                id: cat_result.scanner_id.clone(),
                name: cat_result.name.clone(),
                description: String::new(),
                size_bytes: cat_result.total_size(),
                item_count: items.len(),
                items,
            }
        })
        .collect();

    let scan_result = JsonScanResult::new(categories, start.elapsed().as_millis() as u64);

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&scan_result)?;
            if let Some(path) = out {
                fs::write(path, &json)?;
            } else {
                println!("{}", json);
            }
        }
        OutputFormat::Human => {
            for cat_result in &scan_result.categories {
                println!("{}:", cat_result.name);
                println!("  Items: {}", cat_result.item_count);
                println!("  Size: {}", format_size(cat_result.size_bytes));
                println!();

                for item in cat_result.items.iter().take(10) {
                    println!(
                        "  - {} ({})",
                        item.path.display(),
                        format_size(item.size_bytes)
                    );
                }

                if cat_result.items.len() > 10 {
                    println!("  ... and {} more", cat_result.items.len() - 10);
                }
                println!();
            }

            println!(
                "Total: {} items, {} (in {}ms)",
                scan_result.total_item_count,
                format_size(scan_result.total_size_bytes),
                scan_result.scan_duration_ms
            );
        }
    }

    Ok(())
}

fn run_plan(
    from: Option<&str>,
    category: Option<&str>,
    format: OutputFormat,
    out: Option<&str>,
) -> Result<()> {
    let scan_result = if let Some(path) = from {
        let content = fs::read_to_string(path)?;
        serde_json::from_str::<JsonScanResult>(&content)?
    } else {
        let config = Config::load()?;
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
            progress_callback: None,
            item_callback: None,
        };

        let report = registry.scan_all(&scan_config)?;

        let categories: Vec<JsonCategoryScanResult> = report
            .categories
            .iter()
            .filter(|cat_result| {
                category
                    .map(|c| cat_result.scanner_id.contains(&c.to_lowercase()))
                    .unwrap_or(true)
            })
            .map(|cat_result| JsonCategoryScanResult {
                id: cat_result.scanner_id.clone(),
                name: cat_result.name.clone(),
                description: String::new(),
                size_bytes: cat_result.total_size(),
                item_count: cat_result.items.len(),
                items: cat_result
                    .items
                    .iter()
                    .map(|item| ScanItem {
                        path: item.path.clone(),
                        size_bytes: item.size,
                        modified: Utc::now(),
                        last_used: None,
                        use_count: None,
                    })
                    .collect(),
            })
            .collect();

        JsonScanResult::new(categories, report.duration.as_millis() as u64)
    };

    let categories: Vec<CategoryPlanResult> = scan_result
        .categories
        .iter()
        .map(|cat| CategoryPlanResult {
            id: cat.id.clone(),
            action: "delete".to_string(),
            items: cat
                .items
                .iter()
                .map(|item| PlanItem {
                    path: item.path.clone(),
                    size_bytes: item.size_bytes,
                })
                .collect(),
        })
        .collect();

    let plan_result = PlanResult::new(categories, from.map(|s| s.to_string()));

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&plan_result)?;
            if let Some(path) = out {
                fs::write(path, &json)?;
            } else {
                println!("{}", json);
            }
        }
        OutputFormat::Human => {
            println!("Cleanup Plan:\n");
            for cat in &plan_result.categories {
                println!("{} ({}):", cat.id, cat.action);
                for item in cat.items.iter().take(10) {
                    println!(
                        "  - {} ({})",
                        item.path.display(),
                        format_size(item.size_bytes)
                    );
                }
                if cat.items.len() > 10 {
                    println!("  ... and {} more", cat.items.len() - 10);
                }
                println!();
            }
            println!("Total: {}", format_size(plan_result.total_size_bytes));
        }
    }

    Ok(())
}

fn run_apply(
    plan_path: Option<&str>,
    category: Option<&str>,
    yes: bool,
    config: &Config,
    format: OutputFormat,
    out: Option<&str>,
) -> Result<()> {
    let start = Instant::now();

    let items_to_clean: Vec<plugin::ScanResult> = if let Some(path) = plan_path {
        let content = fs::read_to_string(path)?;
        let plan: PlanResult = serde_json::from_str(&content)?;

        plan.categories
            .iter()
            .flat_map(|cat| cat.items.iter())
            .map(|item| plugin::ScanResult {
                id: item.path.to_string_lossy().to_string(),
                name: item
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                path: item.path.clone(),
                size: item.size_bytes,
                file_count: 1,
                last_accessed: None,
                last_modified: None,
                safety_level: plugin::SafetyLevel::Safe,
                category: plugin::ScannerCategory::System,
                metadata: HashMap::new(),
            })
            .collect()
    } else {
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
            progress_callback: None,
            item_callback: None,
        };

        let report = registry.scan_all(&scan_config)?;

        report
            .categories
            .iter()
            .filter(|cat_result| {
                category
                    .map(|c| cat_result.scanner_id.contains(&c.to_lowercase()))
                    .unwrap_or(true)
            })
            .flat_map(|cat| cat.items.clone())
            .collect()
    };

    if !yes {
        println!(
            "Found {} items to clean ({})",
            items_to_clean.len(),
            format_size(items_to_clean.iter().map(|i| i.size).sum())
        );
        println!("Use --yes to execute");
        return Ok(());
    }

    let cleaner = DefaultCleaner::new();
    let clean_config = CleanConfig {
        dry_run: false,
        log_history: config.clean.log_history,
    };

    let result = cleaner.clean(&items_to_clean, &clean_config)?;

    let category_results = vec![CategoryExecutionResult {
        id: "all".to_string(),
        status: if result.failed_count == 0 {
            ExecutionStatus::Success
        } else if result.success_count > 0 {
            ExecutionStatus::Partial
        } else {
            ExecutionStatus::Failed
        },
        deleted_count: result.success_count,
        deleted_size_bytes: result.total_freed,
        failed_count: result.failed_count,
        failed_items: result
            .failed_items
            .iter()
            .map(|(path, error)| FailedItem {
                path: path.clone(),
                error: error.clone(),
            })
            .collect(),
    }];

    let exec_result = ExecutionResult::new(
        plan_path.map(|s| s.to_string()),
        category_results,
        start.elapsed().as_millis() as u64,
    );

    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&exec_result)?;
            if let Some(path) = out {
                fs::write(path, &json)?;
            } else {
                println!("{}", json);
            }
        }
        OutputFormat::Human => {
            println!("\nResults:");
            println!("  Cleaned: {} items", exec_result.total_deleted_size);
            println!("  Status: {:?}", exec_result.status);
            println!("  Duration: {}ms", exec_result.duration_ms);
        }
    }

    Ok(())
}

fn run_report(from: &str, format: ReportFormat, out: Option<&str>) -> Result<()> {
    let content = fs::read_to_string(from)?;

    let report = if let Ok(scan) = serde_json::from_str::<JsonScanResult>(&content) {
        generate_scan_report(&scan, &format)
    } else if let Ok(exec) = serde_json::from_str::<ExecutionResult>(&content) {
        generate_exec_report(&exec, &format)
    } else {
        anyhow::bail!("Unknown file format")
    };

    if let Some(path) = out {
        fs::write(path, &report)?;
    } else {
        println!("{}", report);
    }

    Ok(())
}

fn generate_scan_report(scan: &JsonScanResult, format: &ReportFormat) -> String {
    match format {
        ReportFormat::Json => serde_json::to_string_pretty(scan).unwrap_or_default(),
        ReportFormat::Md => {
            let mut md = String::new();
            md.push_str("# CleanMac Scan Report\n\n");
            md.push_str(&format!(
                "**Date**: {}\n\n",
                scan.timestamp.format("%Y-%m-%d %H:%M:%S")
            ));
            md.push_str(&format!(
                "**Total**: {} items, {}\n\n",
                scan.total_item_count,
                format_size(scan.total_size_bytes)
            ));

            for cat in &scan.categories {
                md.push_str(&format!(
                    "## {} ({} bytes)\n\n",
                    cat.name,
                    format_size(cat.size_bytes)
                ));
                md.push_str(&format!("Items: {}\n\n", cat.item_count));
            }

            md
        }
        ReportFormat::Txt => {
            let mut txt = String::new();
            txt.push_str("CleanMac Scan Report\n");
            txt.push_str("====================\n\n");
            txt.push_str(&format!(
                "Date: {}\n\n",
                scan.timestamp.format("%Y-%m-%d %H:%M:%S")
            ));
            txt.push_str(&format!(
                "Total: {} items, {}\n\n",
                scan.total_item_count,
                format_size(scan.total_size_bytes)
            ));

            for cat in &scan.categories {
                txt.push_str(&format!(
                    "{} ({} bytes)\n",
                    cat.name,
                    format_size(cat.size_bytes)
                ));
                txt.push_str(&format!("  Items: {}\n\n", cat.item_count));
            }

            txt
        }
    }
}

fn generate_exec_report(exec: &ExecutionResult, format: &ReportFormat) -> String {
    match format {
        ReportFormat::Json => serde_json::to_string_pretty(exec).unwrap_or_default(),
        ReportFormat::Md => {
            let mut md = String::new();
            md.push_str("# CleanMac Execution Report\n\n");
            md.push_str(&format!(
                "**Date**: {}\n\n",
                exec.timestamp.format("%Y-%m-%d %H:%M:%S")
            ));
            md.push_str(&format!("**Status**: {:?}\n\n", exec.status));
            md.push_str(&format!(
                "**Freed**: {}\n\n",
                format_size(exec.total_deleted_size)
            ));

            for cat in &exec.categories {
                md.push_str(&format!("## {} - {:?}\n\n", cat.id, cat.status));
                md.push_str(&format!(
                    "Deleted: {} items ({})\n",
                    cat.deleted_count,
                    format_size(cat.deleted_size_bytes)
                ));
                if cat.failed_count > 0 {
                    md.push_str(&format!("Failed: {} items\n", cat.failed_count));
                }
                md.push_str("\n");
            }

            md
        }
        ReportFormat::Txt => {
            let mut txt = String::new();
            txt.push_str("CleanMac Execution Report\n");
            txt.push_str("=========================\n\n");
            txt.push_str(&format!(
                "Date: {}\n\n",
                exec.timestamp.format("%Y-%m-%d %H:%M:%S")
            ));
            txt.push_str(&format!("Status: {:?}\n\n", exec.status));
            txt.push_str(&format!(
                "Freed: {}\n\n",
                format_size(exec.total_deleted_size)
            ));

            for cat in &exec.categories {
                txt.push_str(&format!("{} - {:?}\n", cat.id, cat.status));
                txt.push_str(&format!(
                    "  Deleted: {} items ({})\n",
                    cat.deleted_count,
                    format_size(cat.deleted_size_bytes)
                ));
                if cat.failed_count > 0 {
                    txt.push_str(&format!("  Failed: {} items\n", cat.failed_count));
                }
                txt.push_str("\n");
            }

            txt
        }
    }
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
        progress_callback: None,
        item_callback: None,
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
