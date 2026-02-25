use crate::config::Config;
use crate::plugin::registry::ScanReport;
use crate::tui::state::{
    AppMode, AppsModeState, CleanResultDisplay, ScanMessage, ScanProgress, ScannerInfo, SortMode,
    SpaceLensState,
};
use ratatui::widgets::ListState;
use std::collections::HashSet;
use std::sync::mpsc::Receiver;

pub struct App {
    pub config: Config,
    pub report: Option<ScanReport>,
    pub selected_category: usize,
    pub selected_items: HashSet<String>,
    pub list_state: ListState,
    pub mode: AppMode,
    pub prev_mode: Option<AppMode>,
    pub should_quit: bool,
    pub scan_progress: ScanProgress,
    pub clean_result: Option<CleanResultDisplay>,
    pub apps_mode: AppsModeState,
    pub scan_receiver: Option<Receiver<ScanMessage>>,
    pub available_scanners: Vec<ScannerInfo>,
    pub sort_mode: SortMode,
    pub space_lens: SpaceLensState,
}

impl App {
    pub fn new(config: Config) -> Self {
        let available_scanners = vec![
            ScannerInfo {
                id: "system_caches".into(),
                name: "System Caches".into(),
                enabled: true,
            },
            ScannerInfo {
                id: "system_logs".into(),
                name: "System Logs".into(),
                enabled: true,
            },
            ScannerInfo {
                id: "trash".into(),
                name: "Trash".into(),
                enabled: true,
            },
            ScannerInfo {
                id: "browser_caches".into(),
                name: "Browser Caches".into(),
                enabled: true,
            },
            ScannerInfo {
                id: "dev_junk".into(),
                name: "Development Junk".into(),
                enabled: true,
            },
            ScannerInfo {
                id: "large_old_files".into(),
                name: "Large & Old Files".into(),
                enabled: true,
            },
            ScannerInfo {
                id: "mail_attachments".into(),
                name: "Mail Attachments".into(),
                enabled: true,
            },
            ScannerInfo {
                id: "photo_junk".into(),
                name: "Photo Junk".into(),
                enabled: true,
            },
            ScannerInfo {
                id: "music_junk".into(),
                name: "Music & Podcasts".into(),
                enabled: true,
            },
            ScannerInfo {
                id: "duplicates".into(),
                name: "Duplicates".into(),
                enabled: false,
            },
            ScannerInfo {
                id: "privacy".into(),
                name: "Privacy".into(),
                enabled: false,
            },
            ScannerInfo {
                id: "maintenance".into(),
                name: "Maintenance".into(),
                enabled: false,
            },
            ScannerInfo {
                id: "startup_items".into(),
                name: "Startup Items".into(),
                enabled: false,
            },
        ];

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            config,
            report: None,
            selected_category: 0,
            selected_items: HashSet::new(),
            list_state,
            mode: AppMode::CategorySelect,
            prev_mode: None,
            should_quit: false,
            scan_progress: ScanProgress::default(),
            clean_result: None,
            apps_mode: AppsModeState::default(),
            scan_receiver: None,
            available_scanners,
            sort_mode: SortMode::default(),
            space_lens: SpaceLensState::default(),
        }
    }
}
