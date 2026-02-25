use crate::config::Config;
use crate::plugin::registry::ScanReport;
use crate::tui::logic::{
    apply_sort, deselect_all, navigate_category_next, navigate_category_prev, navigate_down,
    navigate_up, select_all_in_category, toggle_selection,
};
use crate::tui::service::disk::start_space_scan;
use crate::tui::service::scanner::{start_scan, ScanStartParams};
use crate::tui::state::{
    AppMode, ScanMessage, ScanProgress, ScannerInfo, SortMode, SpaceLensState,
};
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::widgets::ListState;
use std::collections::HashSet;
use std::sync::mpsc::Receiver;

pub struct ReviewContext<'a> {
    pub list_state: &'a mut ListState,
    pub selected_category: &'a mut usize,
    pub selected_items: &'a mut HashSet<String>,
    pub report: &'a mut Option<ScanReport>,
    pub mode: &'a mut AppMode,
    pub prev_mode: &'a mut Option<AppMode>,
    pub should_quit: &'a mut bool,
    pub sort_mode: &'a mut SortMode,
    pub space_lens: &'a mut SpaceLensState,
    pub config: &'a Config,
    pub available_scanners: &'a [ScannerInfo],
    pub scan_progress: &'a mut ScanProgress,
    pub scan_receiver: &'a mut Option<Receiver<ScanMessage>>,
}

pub fn handle_review_key(ctx: &mut ReviewContext, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Char('q') => *ctx.should_quit = true,
        KeyCode::Up => navigate_up(ctx.list_state),
        KeyCode::Down => {
            if let Some(report) = ctx.report.as_ref() {
                if let Some(category) = report.categories.get(*ctx.selected_category) {
                    navigate_down(ctx.list_state, category.items.len());
                }
            }
        }
        KeyCode::Left => navigate_category_prev(ctx.selected_category, ctx.list_state),
        KeyCode::Right => {
            navigate_category_next(ctx.selected_category, ctx.list_state, ctx.report.as_ref())
        }
        KeyCode::Char(' ') => {
            let focused = get_focused_item(
                ctx.report.as_ref(),
                *ctx.selected_category,
                ctx.list_state.selected(),
            );
            toggle_selection(ctx.selected_items, focused.as_ref());
        }
        KeyCode::Char('a') => {
            if let Some(report) = ctx.report.as_ref() {
                if let Some(category) = report.categories.get(*ctx.selected_category) {
                    select_all_in_category(ctx.selected_items, &category.items);
                }
            }
        }
        KeyCode::Char('n') => deselect_all(ctx.selected_items),
        KeyCode::Enter => {
            if !ctx.selected_items.is_empty() {
                *ctx.mode = AppMode::ConfirmClean;
            }
        }
        KeyCode::Char('?') => {
            *ctx.prev_mode = Some(*ctx.mode);
            *ctx.mode = AppMode::Help;
        }
        KeyCode::Esc | KeyCode::Tab => {
            *ctx.mode = AppMode::CategorySelect;
        }
        KeyCode::Char('r') => {
            ctx.selected_items.clear();
            *ctx.report = None;
            let enabled_ids: Vec<String> = ctx
                .available_scanners
                .iter()
                .filter(|s| s.enabled)
                .map(|s| s.id.clone())
                .collect();
            let mut params = ScanStartParams {
                config: ctx.config,
                enabled_scanner_ids: enabled_ids,
                report: ctx.report,
                scan_progress: ctx.scan_progress,
                scan_receiver: ctx.scan_receiver,
                mode: ctx.mode,
            };
            start_scan(&mut params);
        }
        KeyCode::Char('s') => {
            *ctx.sort_mode = ctx.sort_mode.next();
            if let Some(ref mut report) = ctx.report {
                apply_sort(report, *ctx.sort_mode);
            }
        }
        KeyCode::Char('v') => {
            *ctx.prev_mode = Some(*ctx.mode);
            ctx.space_lens.current_path =
                dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/"));
            ctx.list_state.select(Some(0));
            start_space_scan(ctx.space_lens);
            *ctx.mode = AppMode::SpaceLens;
        }
        _ => {}
    }
    Ok(())
}

fn get_focused_item(
    report: Option<&ScanReport>,
    selected_category: usize,
    selected: Option<usize>,
) -> Option<crate::plugin::ScanResult> {
    let report = report?;
    let category = report.categories.get(selected_category)?;
    let idx = selected?;
    category.items.get(idx).cloned()
}
