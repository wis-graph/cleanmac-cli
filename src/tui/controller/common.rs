use crate::cleaner::DefaultCleaner;
use crate::plugin::{CleanConfig, Cleaner, ScanResult};
use crate::tui::state::{AppMode, CleanResultDisplay};
use anyhow::Result;
use crossterm::event::KeyCode;
use std::collections::HashSet;

pub struct ConfirmContext<'a> {
    pub mode: &'a mut AppMode,
    pub selected_items: &'a HashSet<String>,
    pub report_items: Vec<ScanResult>,
    pub clean_result: &'a mut Option<CleanResultDisplay>,
}

pub fn handle_confirm_key(ctx: &mut ConfirmContext, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Char('y') | KeyCode::Enter => {
            let items_to_clean: Vec<ScanResult> = ctx
                .report_items
                .iter()
                .filter(|item| ctx.selected_items.contains(&item.id))
                .cloned()
                .collect();

            let cleaner = DefaultCleaner::new();
            let config = CleanConfig {
                dry_run: false,
                log_history: true,
            };

            let result = cleaner.clean(&items_to_clean, &config)?;

            *ctx.clean_result = Some(CleanResultDisplay {
                success_count: result.success_count,
                failed_count: result.failed_count,
                total_freed: result.total_freed,
                duration: result.duration,
            });

            *ctx.mode = AppMode::ResultDisplay;
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            *ctx.mode = AppMode::Review;
        }
        _ => {}
    }
    Ok(())
}

pub struct ResultContext<'a> {
    pub mode: &'a mut AppMode,
}

pub fn handle_result_key(ctx: &mut ResultContext, code: KeyCode) -> Result<()> {
    if matches!(code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q')) {
        *ctx.mode = AppMode::Review;
    }
    Ok(())
}

pub struct HelpContext<'a> {
    pub mode: &'a mut AppMode,
    pub prev_mode: &'a mut Option<AppMode>,
}

pub fn handle_help_key(ctx: &mut HelpContext, code: KeyCode) -> Result<()> {
    if matches!(code, KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('?')) {
        *ctx.mode = ctx.prev_mode.unwrap_or(AppMode::Review);
        *ctx.prev_mode = None;
    }
    Ok(())
}
