use crate::tui::state::App;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{backend::Backend, Terminal};
use std::time::Duration;

use crate::config::Config;
use crate::tui::controller::app_list;
use crate::tui::controller::category_select;
use crate::tui::controller::common;
use crate::tui::controller::review;
use crate::tui::controller::space_lens;
use crate::tui::controller::uninstall;
use crate::tui::controller::{
    handle_app_list_key, handle_category_select_key, handle_confirm_key, handle_help_key,
    handle_result_key, handle_review_key, handle_space_lens_key, handle_uninstall_result_key,
    handle_uninstall_review_key,
};
use crate::tui::service::disk::{poll_space_sizes, start_space_scan};
use crate::tui::service::scanner::{poll_scan_messages, PollContext};
use crate::tui::state::{AppMode, AppsModeState};
use crate::tui::view::components::modal::{
    render_confirm_modal, render_help_modal, render_result_modal,
};
use crate::tui::view::{
    render_app_list, render_category_select, render_loading, render_review, render_space_lens,
    render_uninstall_result, render_uninstall_review, CategorySelectData,
};
use crate::uninstaller::{AppDetector, RelatedFileDetector};

impl App {
    pub fn new_apps_mode() -> Self {
        use rayon::prelude::*;
        use std::sync::mpsc::channel;
        use walkdir::WalkDir;

        let detector = AppDetector::new();
        let apps = detector.list_all();

        let app_paths: Vec<(usize, std::path::PathBuf)> = apps
            .iter()
            .enumerate()
            .map(|(i, app)| (i, app.path.clone()))
            .collect();

        let (tx, rx) = channel();

        rayon::spawn(move || {
            app_paths.par_iter().for_each(|(idx, path)| {
                let size: u64 = if path.exists() {
                    WalkDir::new(path)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter_map(|e| e.metadata().ok())
                        .filter(|m| m.is_file())
                        .map(|m| m.len())
                        .sum()
                } else {
                    0
                };
                let _ = tx.send((*idx, size));
            });
        });

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(0));

        let mut app = Self::new(Config::default());
        app.mode = AppMode::AppList;
        app.apps_mode = AppsModeState {
            apps,
            size_receiver: Some(rx),
            ..Default::default()
        };
        app.list_state = list_state;
        app.available_scanners.clear();
        app
    }

    pub fn new_space_lens_mode(start_path: Option<&str>) -> Self {
        let mut app = Self::new(Config::default());
        app.mode = AppMode::SpaceLens;
        app.space_lens.current_path = start_path
            .map(|p| std::path::PathBuf::from(p))
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/")));
        app.list_state.select(Some(0));
        // 스캔은 run() 루프에서 첫 프레임 후에 시작
        app.available_scanners.clear();
        app
    }

    pub fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
        // SpaceLens 모드면 첫 프레임 전에 스캔 시작하고 잠시 대기
        if self.mode == AppMode::SpaceLens && self.space_lens.entries.is_empty() {
            start_space_scan(&mut self.space_lens);
            std::thread::sleep(Duration::from_millis(50));
        }

        while !self.should_quit {
            if self.mode == AppMode::LoadingRelatedFiles {
                terminal.draw(|f| render_loading(f))?;
                self.load_related_files();
            }

            self.poll_app_sizes();
            self.poll_scan();
            poll_space_sizes(&mut self.space_lens);

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

    fn poll_scan(&mut self) {
        let mut ctx = PollContext {
            scan_receiver: &mut self.scan_receiver,
            report: &mut self.report,
            scan_progress: &mut self.scan_progress,
            list_state: &mut self.list_state,
        };
        poll_scan_messages(&mut ctx);
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

    fn select_all_related(&mut self) {
        self.apps_mode.selected_related.insert(0);
        for (i, file) in self.apps_mode.cached_related_files.iter().enumerate() {
            if !file.category.is_protected() {
                self.apps_mode.selected_related.insert(i + 1);
            }
        }
    }

    fn handle_key(&mut self, code: KeyCode, _modifiers: KeyModifiers) -> Result<()> {
        match self.mode {
            AppMode::CategorySelect => {
                let mut ctx = category_select::CategorySelectContext {
                    list_state: &mut self.list_state,
                    available_scanners: &mut self.available_scanners,
                    mode: &mut self.mode,
                    should_quit: &mut self.should_quit,
                    config: &self.config,
                    report: &mut self.report,
                    scan_progress: &mut self.scan_progress,
                    scan_receiver: &mut self.scan_receiver,
                };
                handle_category_select_key(&mut ctx, code)
            }
            AppMode::Review => {
                let mut ctx = review::ReviewContext {
                    list_state: &mut self.list_state,
                    selected_category: &mut self.selected_category,
                    selected_items: &mut self.selected_items,
                    report: &mut self.report,
                    mode: &mut self.mode,
                    prev_mode: &mut self.prev_mode,
                    should_quit: &mut self.should_quit,
                    sort_mode: &mut self.sort_mode,
                    space_lens: &mut self.space_lens,
                    config: &self.config,
                    available_scanners: &self.available_scanners,
                    scan_progress: &mut self.scan_progress,
                    scan_receiver: &mut self.scan_receiver,
                };
                handle_review_key(&mut ctx, code)
            }
            AppMode::ConfirmClean => {
                let mut ctx = common::ConfirmContext {
                    mode: &mut self.mode,
                };
                handle_confirm_key(&mut ctx, code)
            }
            AppMode::ResultDisplay => {
                let mut ctx = common::ResultContext {
                    mode: &mut self.mode,
                };
                handle_result_key(&mut ctx, code)
            }
            AppMode::Help => {
                let mut ctx = common::HelpContext {
                    mode: &mut self.mode,
                    prev_mode: &mut self.prev_mode,
                };
                handle_help_key(&mut ctx, code)
            }
            AppMode::LoadingRelatedFiles => Ok(()),
            AppMode::AppList => {
                let mut ctx = app_list::AppListContext {
                    list_state: &mut self.list_state,
                    apps_mode: &mut self.apps_mode,
                    mode: &mut self.mode,
                    prev_mode: &mut self.prev_mode,
                    should_quit: &mut self.should_quit,
                };
                handle_app_list_key(&mut ctx, code)
            }
            AppMode::UninstallReview => {
                let mut ctx = uninstall::UninstallReviewContext {
                    list_state: &mut self.list_state,
                    apps_mode: &mut self.apps_mode,
                    mode: &mut self.mode,
                    prev_mode: &mut self.prev_mode,
                };
                handle_uninstall_review_key(&mut ctx, code)
            }
            AppMode::UninstallResult => {
                let mut ctx = uninstall::UninstallResultContext {
                    apps_mode: &mut self.apps_mode,
                    mode: &mut self.mode,
                };
                handle_uninstall_result_key(&mut ctx, code)
            }
            AppMode::SpaceLens => {
                let mut ctx = space_lens::SpaceLensContext {
                    list_state: &mut self.list_state,
                    space_lens: &mut self.space_lens,
                    mode: &mut self.mode,
                    prev_mode: &mut self.prev_mode,
                    should_quit: &mut self.should_quit,
                };
                handle_space_lens_key(&mut ctx, code)
            }
        }
    }

    fn render(&mut self, f: &mut ratatui::Frame) {
        match self.mode {
            AppMode::CategorySelect => {
                let mut data = CategorySelectData {
                    list_state: &mut self.list_state,
                    available_scanners: &self.available_scanners,
                    report: self.report.as_ref(),
                };
                render_category_select(f, &mut data);
            }
            AppMode::AppList => {
                render_app_list(f, &mut self.list_state, &self.apps_mode);
            }
            AppMode::UninstallReview => {
                render_uninstall_review(f, &mut self.list_state, &self.apps_mode);
            }
            AppMode::UninstallResult => {
                render_uninstall_result(f, &self.apps_mode);
            }
            AppMode::SpaceLens => {
                render_space_lens(f, &mut self.list_state, &mut self.space_lens);
            }
            AppMode::LoadingRelatedFiles => {
                render_loading(f);
            }
            _ => {
                render_review(
                    f,
                    &mut self.list_state,
                    &mut self.report,
                    &self.selected_items,
                    &mut self.selected_category,
                    self.sort_mode,
                    &self.scan_progress,
                    self.scan_receiver.is_some(),
                );
            }
        }

        match self.mode {
            AppMode::ConfirmClean => {
                let selected: Vec<_> = self
                    .report
                    .iter()
                    .flat_map(|r| r.categories.iter())
                    .flat_map(|c| c.items.iter())
                    .filter(|item| self.selected_items.contains(&item.id))
                    .collect();
                let total_size: u64 = selected.iter().map(|i| i.size).sum();
                render_confirm_modal(
                    f,
                    &crate::tui::view::components::modal::ConfirmModalData {
                        selected_count: selected.len(),
                        total_size,
                    },
                );
            }
            AppMode::ResultDisplay => {
                render_result_modal(f, self.clean_result.as_ref());
            }
            AppMode::Help => {
                render_help_modal(f);
            }
            _ => {}
        }
    }
}
