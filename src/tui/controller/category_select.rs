use crate::config::Config;
use crate::plugin::registry::ScanReport;
use crate::tui::service::scanner::{start_scan, ScanStartParams};
use crate::tui::state::{AppMode, ScanMessage, ScanProgress, ScannerInfo};
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::widgets::ListState;
use std::sync::mpsc::Receiver;

pub struct CategorySelectContext<'a> {
    pub list_state: &'a mut ListState,
    pub available_scanners: &'a mut [ScannerInfo],
    pub mode: &'a mut AppMode,
    pub should_quit: &'a mut bool,
    pub config: &'a Config,
    pub report: &'a mut Option<ScanReport>,
    pub scan_progress: &'a mut ScanProgress,
    pub scan_receiver: &'a mut Option<Receiver<ScanMessage>>,
}

pub fn handle_category_select_key(ctx: &mut CategorySelectContext, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Char('q') => *ctx.should_quit = true,
        KeyCode::Up => {
            if let Some(current) = ctx.list_state.selected() {
                if current > 0 {
                    ctx.list_state.select(Some(current - 1));
                }
            }
        }
        KeyCode::Down => {
            let max = ctx.available_scanners.len().saturating_sub(1);
            if let Some(current) = ctx.list_state.selected() {
                if current < max {
                    ctx.list_state.select(Some(current + 1));
                }
            }
        }
        KeyCode::Char(' ') => {
            if let Some(idx) = ctx.list_state.selected() {
                if let Some(scanner) = ctx.available_scanners.get_mut(idx) {
                    scanner.enabled = !scanner.enabled;
                }
            }
        }
        KeyCode::Char('a') => {
            for scanner in ctx.available_scanners.iter_mut() {
                scanner.enabled = true;
            }
        }
        KeyCode::Char('n') => {
            for scanner in ctx.available_scanners.iter_mut() {
                scanner.enabled = false;
            }
        }
        KeyCode::Enter | KeyCode::Tab => {
            if ctx.report.is_some() && !ctx.report.as_ref().unwrap().categories.is_empty() {
                *ctx.mode = AppMode::Review;
            }
        }
        KeyCode::Char('r') => {
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
        _ => {}
    }
    Ok(())
}
