use crate::cleaner::DefaultCleaner;
use crate::config::Config;
use crate::plugin::{
    registry::{CategoryScanResult, ScanReport},
    CleanConfig, Cleaner, PluginRegistry, SafetyLevel, ScanConfig, ScanResult, Scanner,
};
use crate::uninstaller::{AppBundle, RelatedFile, RelatedFileDetector, Uninstaller};
use crate::utils::{format_number, format_size};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

enum ScanMessage {
    ScannerStart {
        name: String,
        index: usize,
        total: usize,
    },
    ScannerDone {
        category: CategoryScanResult,
    },
    ScanComplete {
        total_size: u64,
        total_items: usize,
    },
}

pub struct App {
    config: Config,
    registry: PluginRegistry,
    cleaner: DefaultCleaner,
    report: Option<ScanReport>,
    selected_category: usize,
    selected_items: HashSet<String>,
    list_state: ListState,
    mode: AppMode,
    prev_mode: Option<AppMode>,
    should_quit: bool,
    message: Option<String>,
    scan_progress: ScanProgress,
    clean_result: Option<CleanResultDisplay>,
    apps_mode: AppsModeState,
    scan_receiver: Option<Receiver<ScanMessage>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Review,
    ConfirmClean,
    ResultDisplay,
    Help,
    AppList,
    LoadingRelatedFiles,
    UninstallReview,
    UninstallResult,
}

struct AppsModeState {
    apps: Vec<AppBundle>,
    app_sizes: HashMap<usize, u64>,
    selected_app_idx: Option<usize>,
    selected_related: HashSet<usize>,
    uninstall_result: Option<UninstallResultDisplay>,
    cached_related_files: Vec<RelatedFile>,
    size_receiver: Option<Receiver<(usize, u64)>>,
}

impl Default for AppsModeState {
    fn default() -> Self {
        Self {
            apps: Vec::new(),
            app_sizes: HashMap::new(),
            selected_app_idx: None,
            selected_related: HashSet::new(),
            uninstall_result: None,
            cached_related_files: Vec::new(),
            size_receiver: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct UninstallResultDisplay {
    app_deleted: bool,
    related_deleted: usize,
    total_freed: u64,
    errors: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct ScanProgress {
    current_scanner: String,
    scanners_done: usize,
    total_scanners: usize,
    start_time: Option<Instant>,
}

#[derive(Debug, Clone)]
struct CleanResultDisplay {
    success_count: usize,
    failed_count: usize,
    total_freed: u64,
    duration: Duration,
}

impl App {
    pub fn new(config: Config) -> Self {
        let (tx, rx) = mpsc::channel();

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
        };

        thread::spawn(move || {
            let scanners: Vec<(&str, Box<dyn Scanner>, crate::plugin::ScannerCategory)> = vec![
                (
                    "System Caches",
                    Box::new(crate::scanner::CacheScanner::new()) as Box<dyn Scanner>,
                    crate::plugin::ScannerCategory::System,
                ),
                (
                    "System Logs",
                    Box::new(crate::scanner::LogScanner::new()) as Box<dyn Scanner>,
                    crate::plugin::ScannerCategory::System,
                ),
                (
                    "Trash",
                    Box::new(crate::scanner::TrashScanner::new()) as Box<dyn Scanner>,
                    crate::plugin::ScannerCategory::Trash,
                ),
                (
                    "Browser Caches",
                    Box::new(crate::scanner::BrowserCacheScanner::new()) as Box<dyn Scanner>,
                    crate::plugin::ScannerCategory::Browser,
                ),
                (
                    "Development Junk",
                    Box::new(crate::scanner::DevJunkScanner::new()) as Box<dyn Scanner>,
                    crate::plugin::ScannerCategory::Development,
                ),
                (
                    "Large & Old Files",
                    Box::new(crate::scanner::LargeOldFilesScanner::new()) as Box<dyn Scanner>,
                    crate::plugin::ScannerCategory::System,
                ),
            ];

            let total = scanners.len();
            let mut total_size: u64 = 0;
            let mut total_items: usize = 0;

            for (idx, (name, scanner, category)) in scanners.into_iter().enumerate() {
                let _ = tx.send(ScanMessage::ScannerStart {
                    name: name.to_string(),
                    index: idx,
                    total,
                });

                let results = scanner.scan(&scan_config).unwrap_or_default();

                let cat = CategoryScanResult {
                    scanner_id: scanner.id().to_string(),
                    name: name.to_string(),
                    category,
                    icon: String::new(),
                    items: results,
                };

                total_size += cat.total_size();
                total_items += cat.items.len();

                let _ = tx.send(ScanMessage::ScannerDone { category: cat });
            }

            let _ = tx.send(ScanMessage::ScanComplete {
                total_size,
                total_items,
            });
        });

        Self {
            config,
            registry: PluginRegistry::default(),
            cleaner: DefaultCleaner::new(),
            report: None,
            selected_category: 0,
            selected_items: HashSet::new(),
            list_state: ListState::default(),
            mode: AppMode::Review,
            prev_mode: None,
            should_quit: false,
            message: None,
            scan_progress: ScanProgress {
                current_scanner: "Initializing...".to_string(),
                scanners_done: 0,
                total_scanners: 6,
                start_time: None,
            },
            clean_result: None,
            apps_mode: AppsModeState::default(),
            scan_receiver: Some(rx),
        }
    }

    fn start_scan(&mut self) {
        let (tx, rx) = mpsc::channel();

        let scan_config = ScanConfig {
            min_size: self.config.scan.min_size_bytes,
            max_depth: self.config.scan.max_depth,
            excluded_paths: self
                .config
                .scan
                .excluded_paths
                .iter()
                .map(|s| s.into())
                .collect(),
            follow_symlinks: self.config.scan.follow_symlinks,
        };

        self.scan_progress = ScanProgress {
            current_scanner: "Initializing...".to_string(),
            scanners_done: 0,
            total_scanners: 6,
            start_time: None,
        };
        self.scan_receiver = Some(rx);

        thread::spawn(move || {
            let scanners: Vec<(&str, Box<dyn Scanner>, crate::plugin::ScannerCategory)> = vec![
                (
                    "System Caches",
                    Box::new(crate::scanner::CacheScanner::new()) as Box<dyn Scanner>,
                    crate::plugin::ScannerCategory::System,
                ),
                (
                    "System Logs",
                    Box::new(crate::scanner::LogScanner::new()) as Box<dyn Scanner>,
                    crate::plugin::ScannerCategory::System,
                ),
                (
                    "Trash",
                    Box::new(crate::scanner::TrashScanner::new()) as Box<dyn Scanner>,
                    crate::plugin::ScannerCategory::Trash,
                ),
                (
                    "Browser Caches",
                    Box::new(crate::scanner::BrowserCacheScanner::new()) as Box<dyn Scanner>,
                    crate::plugin::ScannerCategory::Browser,
                ),
                (
                    "Development Junk",
                    Box::new(crate::scanner::DevJunkScanner::new()) as Box<dyn Scanner>,
                    crate::plugin::ScannerCategory::Development,
                ),
                (
                    "Large & Old Files",
                    Box::new(crate::scanner::LargeOldFilesScanner::new()) as Box<dyn Scanner>,
                    crate::plugin::ScannerCategory::System,
                ),
            ];

            let total = scanners.len();
            let mut total_size: u64 = 0;
            let mut total_items: usize = 0;

            for (idx, (name, scanner, category)) in scanners.into_iter().enumerate() {
                let _ = tx.send(ScanMessage::ScannerStart {
                    name: name.to_string(),
                    index: idx,
                    total,
                });

                let results = scanner.scan(&scan_config).unwrap_or_default();

                let cat = CategoryScanResult {
                    scanner_id: scanner.id().to_string(),
                    name: name.to_string(),
                    category,
                    icon: String::new(),
                    items: results,
                };

                total_size += cat.total_size();
                total_items += cat.items.len();

                let _ = tx.send(ScanMessage::ScannerDone { category: cat });
            }

            let _ = tx.send(ScanMessage::ScanComplete {
                total_size,
                total_items,
            });
        });
    }

    pub fn new_apps_mode() -> Self {
        use crate::uninstaller::AppDetector;

        let detector = AppDetector::new();
        let apps = detector.list_all();

        let app_paths: Vec<(usize, std::path::PathBuf)> = apps
            .iter()
            .enumerate()
            .map(|(i, app)| (i, app.path.clone()))
            .collect();

        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            for (idx, path) in app_paths {
                use walkdir::WalkDir;
                let size: u64 = if path.exists() {
                    WalkDir::new(&path)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter_map(|e| e.metadata().ok())
                        .filter(|m| m.is_file())
                        .map(|m| m.len())
                        .sum()
                } else {
                    0
                };
                let _ = tx.send((idx, size));
            }
        });

        let mut app = Self {
            config: Config::default(),
            registry: PluginRegistry::default(),
            cleaner: DefaultCleaner::new(),
            report: None,
            selected_category: 0,
            selected_items: HashSet::new(),
            list_state: ListState::default(),
            mode: AppMode::AppList,
            prev_mode: None,
            should_quit: false,
            message: None,
            scan_progress: ScanProgress::default(),
            clean_result: None,
            apps_mode: AppsModeState {
                apps,
                size_receiver: Some(rx),
                ..Default::default()
            },
            scan_receiver: None,
        };

        if !app.apps_mode.apps.is_empty() {
            app.list_state.select(Some(0));
        }

        app
    }

    pub fn run(&mut self, terminal: &mut Terminal<impl ratatui::backend::Backend>) -> Result<()> {
        while !self.should_quit {
            if self.mode == AppMode::LoadingRelatedFiles {
                terminal.draw(|f| self.render_loading(f))?;
                self.load_related_files();
            }

            self.poll_app_sizes();
            self.poll_scan_messages();

            terminal.draw(|f| self.render(f))?;

            if event::poll(Duration::from_millis(16))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key(key.code, key.modifiers)?;
                }
            }
        }

        Ok(())
    }

    fn poll_app_sizes(&mut self) {
        if let Some(ref rx) = self.apps_mode.size_receiver {
            while let Ok((idx, size)) = rx.try_recv() {
                self.apps_mode.app_sizes.insert(idx, size);
            }
        }
    }

    fn poll_scan_messages(&mut self) {
        let rx_opt = self.scan_receiver.take();
        if let Some(ref rx) = rx_opt {
            let mut complete = false;
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ScanMessage::ScannerStart { name, index, total } => {
                        self.scan_progress.current_scanner = name;
                        self.scan_progress.scanners_done = index;
                        self.scan_progress.total_scanners = total;
                    }
                    ScanMessage::ScannerDone { category } => {
                        if self.report.is_none() {
                            self.report = Some(ScanReport {
                                categories: Vec::new(),
                                total_size: 0,
                                total_items: 0,
                                duration: std::time::Duration::from_secs(0),
                            });
                        }
                        if let Some(ref mut report) = self.report {
                            report.categories.push(category);
                            if report.categories.len() == 1 {
                                self.list_state.select(Some(0));
                            }
                        }
                        self.scan_progress.scanners_done += 1;
                    }
                    ScanMessage::ScanComplete {
                        total_size,
                        total_items,
                    } => {
                        if let Some(ref mut report) = self.report {
                            report.total_size = total_size;
                            report.total_items = total_items;
                        }
                        self.mode = AppMode::Review;
                        complete = true;
                    }
                }
            }
            if !complete {
                self.scan_receiver = rx_opt;
            }
        }
    }

    fn render_loading(&self, f: &mut Frame) {
        let area = centered_rect(40, 20, f.area());

        let loading = Paragraph::new(Line::from(vec![Span::styled(
            "Scanning related files...",
            Style::default().fg(Color::Cyan),
        )]))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));

        f.render_widget(Clear, area);
        f.render_widget(loading, area);
    }

    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> Result<()> {
        match self.mode {
            AppMode::Review => self.handle_review_key(code, modifiers),
            AppMode::ConfirmClean => self.handle_confirm_key(code),
            AppMode::ResultDisplay => self.handle_result_key(code),
            AppMode::Help => self.handle_help_key(code),
            AppMode::LoadingRelatedFiles => Ok(()),
            AppMode::AppList => self.handle_app_list_key(code),
            AppMode::UninstallReview => self.handle_uninstall_review_key(code),
            AppMode::UninstallResult => self.handle_uninstall_result_key(code),
        }
    }

    fn handle_review_key(&mut self, code: KeyCode, _modifiers: KeyModifiers) -> Result<()> {
        match code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Up => self.navigate_up(),
            KeyCode::Down => self.navigate_down(),
            KeyCode::Left => self.navigate_category_prev(),
            KeyCode::Right => self.navigate_category_next(),
            KeyCode::Char(' ') => self.toggle_selection(),
            KeyCode::Char('a') => self.select_all_in_category(),
            KeyCode::Char('n') => self.deselect_all(),
            KeyCode::Enter => {
                if !self.selected_items.is_empty() {
                    self.mode = AppMode::ConfirmClean;
                }
            }
            KeyCode::Char('?') => {
                self.prev_mode = Some(self.mode);
                self.mode = AppMode::Help;
            }
            KeyCode::Char('r') => {
                self.selected_items.clear();
                self.report = None;
                self.start_scan();
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_confirm_key(&mut self, code: KeyCode) -> Result<()> {
        match code {
            KeyCode::Char('y') | KeyCode::Enter => {
                self.execute_clean();
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.mode = AppMode::Review;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_result_key(&mut self, code: KeyCode) -> Result<()> {
        if matches!(code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q')) {
            self.mode = AppMode::Review;
        }
        Ok(())
    }

    fn handle_app_list_key(&mut self, code: KeyCode) -> Result<()> {
        match code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Up => self.navigate_apps_up(),
            KeyCode::Down => self.navigate_apps_down(),
            KeyCode::Enter => self.select_app_for_uninstall(),
            KeyCode::Char('?') => {
                self.prev_mode = Some(self.mode);
                self.mode = AppMode::Help;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_uninstall_review_key(&mut self, code: KeyCode) -> Result<()> {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.mode = AppMode::AppList;
                self.apps_mode.selected_related.clear();
                self.apps_mode.selected_app_idx = None;
            }
            KeyCode::Up => self.navigate_related_up(),
            KeyCode::Down => self.navigate_related_down(),
            KeyCode::Char(' ') => self.toggle_related_selection(),
            KeyCode::Char('a') => self.select_all_related(),
            KeyCode::Char('n') => self.deselect_all_related(),
            KeyCode::Enter => self.execute_uninstall()?,
            KeyCode::Char('?') => {
                self.prev_mode = Some(self.mode);
                self.mode = AppMode::Help;
            }
            _ => {}
        }
        Ok(())
    }

    fn execute_uninstall(&mut self) -> Result<()> {
        let app_idx = match self.apps_mode.selected_app_idx {
            Some(idx) => idx,
            None => return Ok(()),
        };

        let app = match self.apps_mode.apps.get(app_idx) {
            Some(a) => a.clone(),
            None => return Ok(()),
        };

        let selected_related: Vec<_> = self
            .apps_mode
            .cached_related_files
            .iter()
            .enumerate()
            .filter(|(i, _)| self.apps_mode.selected_related.contains(&(*i + 1)))
            .map(|(_, f)| f.clone())
            .collect();

        let uninstaller = Uninstaller::new(false);
        let result = uninstaller.uninstall(&app, &selected_related)?;

        self.apps_mode.uninstall_result = Some(UninstallResultDisplay {
            app_deleted: result.deleted_app,
            related_deleted: result.deleted_related.len(),
            total_freed: result.total_freed,
            errors: result.errors,
        });

        if result.deleted_app {
            self.apps_mode.apps.remove(app_idx);
        }

        self.mode = AppMode::UninstallResult;
        self.apps_mode.selected_related.clear();
        self.apps_mode.selected_app_idx = None;
        self.apps_mode.cached_related_files.clear();

        Ok(())
    }

    fn handle_uninstall_result_key(&mut self, code: KeyCode) -> Result<()> {
        if matches!(code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q')) {
            self.mode = AppMode::AppList;
            self.apps_mode.selected_related.clear();
        }
        Ok(())
    }

    fn navigate_apps_up(&mut self) {
        if let Some(current) = self.list_state.selected() {
            if current > 0 {
                self.list_state.select(Some(current - 1));
            }
        }
    }

    fn navigate_apps_down(&mut self) {
        let max = self.apps_mode.apps.len().saturating_sub(1);
        if let Some(current) = self.list_state.selected() {
            if current < max {
                self.list_state.select(Some(current + 1));
            }
        }
    }

    fn select_app_for_uninstall(&mut self) {
        if let Some(idx) = self.list_state.selected() {
            self.apps_mode.selected_app_idx = Some(idx);
            self.mode = AppMode::LoadingRelatedFiles;
        }
    }

    fn load_related_files(&mut self) {
        if let Some(idx) = self.apps_mode.selected_app_idx {
            if let Some(app) = self.apps_mode.apps.get(idx) {
                let detector = RelatedFileDetector::new();
                self.apps_mode.cached_related_files = detector.find_related_files(app);
            }
        }
        self.list_state.select(Some(0));
        self.select_all_related();
        self.mode = AppMode::UninstallReview;
    }

    fn navigate_related_up(&mut self) {
        if let Some(current) = self.list_state.selected() {
            if current > 0 {
                self.list_state.select(Some(current - 1));
            }
        }
    }

    fn navigate_related_down(&mut self) {
        let max = self.apps_mode.cached_related_files.len();
        if let Some(current) = self.list_state.selected() {
            if current < max {
                self.list_state.select(Some(current + 1));
            }
        }
    }

    fn toggle_related_selection(&mut self) {
        if let Some(idx) = self.list_state.selected() {
            if self.apps_mode.selected_related.contains(&idx) {
                self.apps_mode.selected_related.remove(&idx);
            } else {
                self.apps_mode.selected_related.insert(idx);
            }
        }
    }

    fn select_all_related(&mut self) {
        self.apps_mode.selected_related.insert(0);
        for (i, file) in self.apps_mode.cached_related_files.iter().enumerate() {
            if !file.category.is_protected() {
                self.apps_mode.selected_related.insert(i + 1);
            }
        }
    }

    fn deselect_all_related(&mut self) {
        self.apps_mode.selected_related.clear();
    }

    fn handle_help_key(&mut self, code: KeyCode) -> Result<()> {
        if matches!(code, KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('?')) {
            self.mode = self.prev_mode.unwrap_or(AppMode::Review);
            self.prev_mode = None;
        }
        Ok(())
    }

    fn navigate_up(&mut self) {
        if let Some(current) = self.list_state.selected() {
            if current > 0 {
                self.list_state.select(Some(current - 1));
            }
        }
    }

    fn navigate_down(&mut self) {
        if let Some(ref report) = self.report {
            if let Some(category) = report.categories.get(self.selected_category) {
                let max = category.items.len().saturating_sub(1);
                if let Some(current) = self.list_state.selected() {
                    if current < max {
                        self.list_state.select(Some(current + 1));
                    }
                }
            }
        }
    }

    fn navigate_category_prev(&mut self) {
        if self.selected_category > 0 {
            self.selected_category -= 1;
            self.list_state.select(Some(0));
        }
    }

    fn navigate_category_next(&mut self) {
        if let Some(ref report) = self.report {
            if self.selected_category < report.categories.len().saturating_sub(1) {
                self.selected_category += 1;
                self.list_state.select(Some(0));
            }
        }
    }

    fn toggle_selection(&mut self) {
        if let Some(item) = self.get_focused_item() {
            let id = item.id.clone();
            if self.selected_items.contains(&id) {
                self.selected_items.remove(&id);
            } else {
                self.selected_items.insert(id);
            }
        }
    }

    fn select_all_in_category(&mut self) {
        if let Some(ref report) = self.report {
            if let Some(category) = report.categories.get(self.selected_category) {
                for item in &category.items {
                    self.selected_items.insert(item.id.clone());
                }
            }
        }
    }

    fn deselect_all(&mut self) {
        self.selected_items.clear();
    }

    fn get_focused_item(&self) -> Option<ScanResult> {
        let report = self.report.as_ref()?;
        let category = report.categories.get(self.selected_category)?;
        let idx = self.list_state.selected()?;
        category.items.get(idx).cloned()
    }

    fn get_selected_items(&self) -> Vec<ScanResult> {
        let mut items = Vec::new();
        if let Some(ref report) = self.report {
            for category in &report.categories {
                for item in &category.items {
                    if self.selected_items.contains(&item.id) {
                        items.push(item.clone());
                    }
                }
            }
        }
        items
    }

    fn execute_clean(&mut self) {
        let items = self.get_selected_items();
        let clean_config = CleanConfig {
            dry_run: false,
            log_history: self.config.clean.log_history,
        };

        let start = Instant::now();
        match self.cleaner.clean(&items, &clean_config) {
            Ok(result) => {
                self.clean_result = Some(CleanResultDisplay {
                    success_count: result.success_count,
                    failed_count: result.failed_count,
                    total_freed: result.total_freed,
                    duration: start.elapsed(),
                });
                self.selected_items.clear();
                self.mode = AppMode::ResultDisplay;
            }
            Err(e) => {
                self.message = Some(format!("Error: {}", e));
                self.mode = AppMode::Review;
            }
        }
    }

    fn render(&mut self, f: &mut Frame) {
        match self.mode {
            AppMode::AppList => {
                self.render_app_list(f);
                return;
            }
            AppMode::UninstallReview => {
                self.render_uninstall_review(f);
                return;
            }
            AppMode::UninstallResult => {
                self.render_uninstall_result(f);
                return;
            }
            _ => {}
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(f.area());

        self.render_header(f, chunks[0]);
        self.render_main(f, chunks[1]);
        self.render_footer(f, chunks[2]);

        match self.mode {
            AppMode::ConfirmClean => self.render_confirm_modal(f),
            AppMode::ResultDisplay => self.render_result_modal(f),
            AppMode::Help => self.render_help_modal(f),
            _ => {}
        }
    }

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let total_size: u64 = self.report.as_ref().map(|r| r.total_size).unwrap_or(0);

        let selected_size: u64 = self.get_selected_items().iter().map(|i| i.size).sum();

        let is_scanning = self.scan_receiver.is_some();
        let scan_indicator = if is_scanning {
            format!(
                " [Scanning {} / {}]",
                self.scan_progress.scanners_done, self.scan_progress.total_scanners
            )
        } else {
            String::new()
        };

        let header = Paragraph::new(Line::from(vec![
            Span::styled(
                " CleanX ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("System Cleaner"),
            Span::raw("   "),
            Span::styled(
                format!(
                    "Total: {} | Selected: {}",
                    format_size(total_size),
                    format_size(selected_size)
                ),
                Style::default().fg(Color::Green),
            ),
            Span::styled(scan_indicator, Style::default().fg(Color::Yellow)),
        ]))
        .block(Block::default().borders(Borders::BOTTOM));

        f.render_widget(header, area);
    }

    fn render_main(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        self.render_sidebar(f, chunks[0]);
        self.render_detail(f, chunks[1]);
    }

    fn render_sidebar(&mut self, f: &mut Frame, area: Rect) {
        let mut items = Vec::new();

        if let Some(ref report) = self.report {
            for (i, category) in report.categories.iter().enumerate() {
                let is_selected = i == self.selected_category;
                let selected_count = category
                    .items
                    .iter()
                    .filter(|item| self.selected_items.contains(&item.id))
                    .count();

                let style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let prefix = if is_selected { "> " } else { "  " };
                let count_indicator = if selected_count > 0 {
                    format!(" [{}]", selected_count)
                } else {
                    String::new()
                };

                items.push(ListItem::new(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(&category.name, style),
                    Span::raw(count_indicator),
                    Span::raw(" "),
                    Span::styled(
                        format!("({})", format_size(category.total_size())),
                        Style::default().fg(Color::DarkGray),
                    ),
                ])));

                if is_selected {
                    for (idx, item) in category.items.iter().enumerate() {
                        let is_item_selected = self.selected_items.contains(&item.id);
                        let is_focused = self.list_state.selected() == Some(idx);
                        let check = if is_item_selected { "[x]" } else { "[ ]" };

                        let safety_color = match item.safety_level {
                            SafetyLevel::Safe => Color::Green,
                            SafetyLevel::Caution => Color::Yellow,
                            SafetyLevel::Protected => Color::Red,
                        };

                        let name_style = if is_focused {
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD)
                        } else if is_item_selected {
                            Style::default().fg(Color::White)
                        } else {
                            Style::default()
                        };

                        items.push(ListItem::new(Line::from(vec![
                            Span::raw("    "),
                            Span::styled(check, Style::default().fg(safety_color)),
                            Span::raw(" "),
                            Span::styled(
                                item.path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("?"),
                                name_style,
                            ),
                            Span::raw(" "),
                            Span::styled(
                                format!("({})", format_size(item.size)),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ])));
                    }
                }
            }
        }

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::RIGHT)
                .title(" Categories "),
        );

        f.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn render_detail(&self, f: &mut Frame, area: Rect) {
        let detail_text = if let Some(item) = self.get_focused_item() {
            self.format_item_detail(&item)
        } else if let Some(ref report) = self.report {
            if let Some(category) = report.categories.get(self.selected_category) {
                self.format_category_detail(category)
            } else {
                "No category selected".to_string()
            }
        } else {
            "No data".to_string()
        };

        let detail = Paragraph::new(detail_text)
            .block(Block::default().borders(Borders::NONE).title(" Details "))
            .wrap(Wrap { trim: true });

        f.render_widget(detail, area);
    }

    fn format_item_detail(&self, item: &ScanResult) -> String {
        let (safety_str, safety_desc) = match item.safety_level {
            SafetyLevel::Safe => ("Safe", "Can be safely deleted"),
            SafetyLevel::Caution => ("Caution", "May affect some applications"),
            SafetyLevel::Protected => ("Protected", "Cannot be deleted"),
        };

        format!(
            "Path:\n  {}\n\nSize:\n  {}\n\nFiles:\n  {}\n\nLast Accessed:\n  {}\n\nLast Modified:\n  {}\n\nSafety Level:\n  {}\n  ({})",
            item.path.display(),
            format_size(item.size),
            format_number(item.file_count),
            item.last_accessed
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
            item.last_modified
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
            safety_str,
            safety_desc
        )
    }

    fn format_category_detail(&self, category: &CategoryScanResult) -> String {
        let selected_count = category
            .items
            .iter()
            .filter(|item| self.selected_items.contains(&item.id))
            .count();

        let selected_size: u64 = category
            .items
            .iter()
            .filter(|item| self.selected_items.contains(&item.id))
            .map(|i| i.size)
            .sum();

        format!(
            "Category:\n  {}\n\nTotal Size:\n  {}\n\nItems:\n  {}\n\nSelected:\n  {} items ({})",
            category.name,
            format_size(category.total_size()),
            category.items.len(),
            selected_count,
            format_size(selected_size)
        )
    }

    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let selected_count = self.selected_items.len();
        let selected_size: u64 = self.get_selected_items().iter().map(|i| i.size).sum();

        let status = if selected_count > 0 {
            format!(
                "Selected: {} items ({})",
                selected_count,
                format_size(selected_size)
            )
        } else if let Some(ref msg) = self.message {
            msg.clone()
        } else {
            "Press ? for help".to_string()
        };

        let footer = Paragraph::new(Line::from(vec![
            Span::styled("↑↓", Style::default().fg(Color::Cyan)),
            Span::raw(" Nav  "),
            Span::styled("←→", Style::default().fg(Color::Cyan)),
            Span::raw(" Cat  "),
            Span::styled("Space", Style::default().fg(Color::Cyan)),
            Span::raw(" Select  "),
            Span::styled("a", Style::default().fg(Color::Cyan)),
            Span::raw(" All  "),
            Span::styled("n", Style::default().fg(Color::Cyan)),
            Span::raw(" None  "),
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::raw(" Clean  "),
            Span::styled("?", Style::default().fg(Color::Cyan)),
            Span::raw(" Help  "),
            Span::styled("q", Style::default().fg(Color::Cyan)),
            Span::raw(" Quit  |  "),
            Span::styled(status, Style::default().fg(Color::Gray)),
        ]))
        .block(Block::default().borders(Borders::TOP));

        f.render_widget(footer, area);
    }

    fn render_confirm_modal(&self, f: &mut Frame) {
        let area = centered_rect(60, 35, f.area());

        let selected = self.get_selected_items();
        let total_size: u64 = selected.iter().map(|i| i.size).sum();

        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Delete ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("{} items", selected.len()),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" (", Style::default().fg(Color::White)),
                Span::styled(
                    format_size(total_size),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(")?", Style::default().fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "This action cannot be undone.",
                Style::default().fg(Color::Red),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("[y/Enter]", Style::default().fg(Color::Green)),
                Span::raw(" Confirm     "),
                Span::styled("[n/Esc]", Style::default().fg(Color::Red)),
                Span::raw(" Cancel"),
            ]),
        ];

        let paragraph = Paragraph::new(text)
            .block(
                Block::default()
                    .title(" Confirm Clean ")
                    .borders(Borders::ALL),
            )
            .alignment(Alignment::Center);

        f.render_widget(Clear, area);
        f.render_widget(paragraph, area);
    }

    fn render_result_modal(&self, f: &mut Frame) {
        let area = centered_rect(60, 40, f.area());

        let result = self.clean_result.as_ref();

        let text = if let Some(r) = result {
            vec![
                Line::from(""),
                Line::from(vec![Span::styled(
                    "Clean Complete!",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Cleaned: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{} items", r.success_count),
                        Style::default().fg(Color::Green),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Failed: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{} items", r.failed_count),
                        Style::default().fg(if r.failed_count > 0 {
                            Color::Red
                        } else {
                            Color::Green
                        }),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Freed: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format_size(r.total_freed),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Duration: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{:.2}s", r.duration.as_secs_f64()),
                        Style::default().fg(Color::Gray),
                    ),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "Press Enter to continue",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        } else {
            vec![Line::from("No result")]
        };

        let paragraph = Paragraph::new(text)
            .block(Block::default().title(" Result ").borders(Borders::ALL))
            .alignment(Alignment::Center);

        f.render_widget(Clear, area);
        f.render_widget(paragraph, area);
    }

    fn render_help_modal(&self, f: &mut Frame) {
        let area = centered_rect(65, 65, f.area());

        let help_text = vec![
            Line::from(vec![Span::styled(
                "CleanX Help",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Navigation",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  ↑/↓    ", Style::default().fg(Color::Cyan)),
                Span::raw("Navigate items"),
            ]),
            Line::from(vec![
                Span::styled("  ←/→    ", Style::default().fg(Color::Cyan)),
                Span::raw("Switch category"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Selection",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  Space  ", Style::default().fg(Color::Cyan)),
                Span::raw("Toggle selection"),
            ]),
            Line::from(vec![
                Span::styled("  a      ", Style::default().fg(Color::Cyan)),
                Span::raw("Select all in category"),
            ]),
            Line::from(vec![
                Span::styled("  n      ", Style::default().fg(Color::Cyan)),
                Span::raw("Deselect all"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Actions",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  Enter  ", Style::default().fg(Color::Cyan)),
                Span::raw("Clean selected"),
            ]),
            Line::from(vec![
                Span::styled("  r      ", Style::default().fg(Color::Cyan)),
                Span::raw("Rescan"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Safety Levels",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  ● Safe     ", Style::default().fg(Color::Green)),
                Span::raw("Can be deleted"),
            ]),
            Line::from(vec![
                Span::styled("  ● Caution  ", Style::default().fg(Color::Yellow)),
                Span::raw("May affect apps"),
            ]),
            Line::from(vec![
                Span::styled("  ● Protected", Style::default().fg(Color::Red)),
                Span::raw("Cannot delete"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  ?      ", Style::default().fg(Color::Cyan)),
                Span::raw("Show this help"),
            ]),
            Line::from(vec![
                Span::styled("  q      ", Style::default().fg(Color::Cyan)),
                Span::raw("Quit"),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Press q, Esc, or ? to close",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let paragraph =
            Paragraph::new(help_text).block(Block::default().title(" Help ").borders(Borders::ALL));

        f.render_widget(Clear, area);
        f.render_widget(paragraph, area);
    }

    fn render_app_list(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(f.area());

        let title = Paragraph::new(Line::from(vec![
            Span::styled(
                " CleanX ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("App Uninstaller"),
            Span::raw("   "),
            Span::styled(
                format!("{} apps found", self.apps_mode.apps.len()),
                Style::default().fg(Color::Green),
            ),
        ]))
        .block(Block::default().borders(Borders::BOTTOM));
        f.render_widget(title, chunks[0]);

        let mut items = Vec::new();
        for (i, app) in self.apps_mode.apps.iter().enumerate() {
            let name = app.name();
            let padded_name = format!("{:<30}", name);

            let size_str = if let Some(&size) = self.apps_mode.app_sizes.get(&i) {
                format_size(size)
            } else {
                "...".to_string()
            };

            items.push(ListItem::new(Line::from(vec![
                Span::raw(padded_name),
                Span::styled(size_str, Style::default().fg(Color::DarkGray)),
            ])));
        }

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::NONE)
                    .title(" Applications "),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
        f.render_stateful_widget(list, chunks[1], &mut self.list_state);

        let footer = Paragraph::new(Line::from(vec![
            Span::styled("↑↓", Style::default().fg(Color::Cyan)),
            Span::raw(" Navigate  "),
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::raw(" Select  "),
            Span::styled("?", Style::default().fg(Color::Cyan)),
            Span::raw(" Help  "),
            Span::styled("q", Style::default().fg(Color::Cyan)),
            Span::raw(" Quit"),
        ]))
        .block(Block::default().borders(Borders::TOP));
        f.render_widget(footer, chunks[2]);
    }

    fn render_uninstall_review(&mut self, f: &mut Frame) {
        let app_idx = self.apps_mode.selected_app_idx.unwrap_or(0);
        let app = match self.apps_mode.apps.get(app_idx) {
            Some(a) => a,
            None => return,
        };

        let related_files = &self.apps_mode.cached_related_files;

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(f.area());

        let header_text = vec![
            Line::from(vec![
                Span::styled("Uninstall: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    app.name(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Size: ", Style::default().fg(Color::Gray)),
                Span::styled(format_size(app.size()), Style::default().fg(Color::Cyan)),
                Span::raw("   "),
                Span::styled("Related: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{} items", related_files.len()),
                    Style::default().fg(Color::Green),
                ),
            ]),
        ];

        let header = Paragraph::new(header_text)
            .block(Block::default().borders(Borders::BOTTOM))
            .alignment(Alignment::Center);
        f.render_widget(header, chunks[0]);

        let mut items = Vec::new();

        let app_selected = self.apps_mode.selected_related.contains(&0);
        let app_name = format!("{}.app", app.name());
        let padded_app_name = format!("{:<35}", app_name);
        let app_size_str = format!("{:>10}", format_size(app.size()));

        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                if app_selected { "[x] " } else { "[ ] " },
                Style::default().fg(Color::Green),
            ),
            Span::styled(
                padded_app_name,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(app_size_str, Style::default().fg(Color::Cyan)),
            Span::raw("  "),
            Span::styled("[App Bundle]", Style::default().fg(Color::DarkGray)),
        ])));

        for (i, file) in related_files.iter().enumerate() {
            let is_selected = self.apps_mode.selected_related.contains(&(i + 1));
            let is_protected = file.category.is_protected();

            let check_color = if is_protected {
                Color::Red
            } else if is_selected {
                Color::Green
            } else {
                Color::Gray
            };

            let file_name = file
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?");

            let padded_name = format!("{:<35}", file_name);
            let size_str = format!("{:>10}", format_size(file.size));
            let protected_tag = if is_protected { " (Protected)" } else { "" };

            items.push(ListItem::new(Line::from(vec![
                Span::styled(
                    if is_selected { "[x] " } else { "[ ] " },
                    Style::default().fg(check_color),
                ),
                Span::raw(padded_name),
                Span::styled(size_str, Style::default().fg(Color::Cyan)),
                Span::raw("  "),
                Span::styled(
                    format!("[{}]", file.category.display_name()),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(protected_tag, Style::default().fg(Color::Red)),
            ])));
        }

        let selected_size: u64 = if self.apps_mode.selected_related.contains(&0) {
            app.size()
        } else {
            0
        } + related_files
            .iter()
            .enumerate()
            .filter(|(i, _)| self.apps_mode.selected_related.contains(&(*i + 1)))
            .map(|(_, f)| f.size)
            .sum::<u64>();

        let list = List::new(items)
            .block(Block::default().borders(Borders::NONE).title(Span::styled(
                format!("Files to delete ({})", format_size(selected_size)),
                Style::default().fg(Color::Yellow),
            )))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        f.render_stateful_widget(list, chunks[1], &mut self.list_state);

        let footer = Paragraph::new(Line::from(vec![
            Span::styled("↑↓", Style::default().fg(Color::Cyan)),
            Span::raw(" Nav  "),
            Span::styled("Space", Style::default().fg(Color::Cyan)),
            Span::raw(" Toggle  "),
            Span::styled("a", Style::default().fg(Color::Cyan)),
            Span::raw(" All  "),
            Span::styled("n", Style::default().fg(Color::Cyan)),
            Span::raw(" None  "),
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::raw(" Delete  "),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::raw(" Back"),
        ]))
        .block(Block::default().borders(Borders::TOP));
        f.render_widget(footer, chunks[2]);
    }

    fn render_uninstall_result(&self, f: &mut Frame) {
        let area = centered_rect(60, 40, f.area());

        let result = &self.apps_mode.uninstall_result;

        let text = if let Some(r) = result {
            let mut lines = vec![
                Line::from(""),
                Line::from(vec![Span::styled(
                    if r.app_deleted {
                        "Uninstalled!"
                    } else {
                        "Uninstall Complete"
                    },
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
            ];

            if r.app_deleted {
                lines.push(Line::from(vec![
                    Span::styled("App: ", Style::default().fg(Color::Gray)),
                    Span::styled("Deleted", Style::default().fg(Color::Green)),
                ]));
            }

            lines.push(Line::from(vec![
                Span::styled("Related files: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{} deleted", r.related_deleted),
                    Style::default().fg(Color::Green),
                ),
            ]));

            lines.push(Line::from(vec![
                Span::styled("Freed: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format_size(r.total_freed),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));

            if !r.errors.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    format!("Errors: {}", r.errors.len()),
                    Style::default().fg(Color::Red),
                )]));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Press Enter to continue",
                Style::default().fg(Color::DarkGray),
            )));

            lines
        } else {
            vec![Line::from("No result")]
        };

        let paragraph = Paragraph::new(text)
            .block(Block::default().title(" Result ").borders(Borders::ALL))
            .alignment(Alignment::Center);

        f.render_widget(Clear, area);
        f.render_widget(paragraph, area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
