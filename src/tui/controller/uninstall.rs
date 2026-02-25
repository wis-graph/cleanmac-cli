use crate::tui::state::{AppMode, AppsModeState, UninstallResultDisplay};
use crate::uninstaller::Uninstaller;
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::widgets::ListState;

pub struct UninstallReviewContext<'a> {
    pub list_state: &'a mut ListState,
    pub apps_mode: &'a mut AppsModeState,
    pub mode: &'a mut AppMode,
    pub prev_mode: &'a mut Option<AppMode>,
}

pub fn handle_uninstall_review_key(ctx: &mut UninstallReviewContext, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Char('q') | KeyCode::Esc => {
            *ctx.mode = AppMode::AppList;
            ctx.apps_mode.selected_related.clear();
            ctx.apps_mode.selected_app_idx = None;
        }
        KeyCode::Up => {
            if let Some(current) = ctx.list_state.selected() {
                if current > 0 {
                    ctx.list_state.select(Some(current - 1));
                }
            }
        }
        KeyCode::Down => {
            let max = ctx.apps_mode.cached_related_files.len();
            if let Some(current) = ctx.list_state.selected() {
                if current < max {
                    ctx.list_state.select(Some(current + 1));
                }
            }
        }
        KeyCode::Char(' ') => {
            if let Some(idx) = ctx.list_state.selected() {
                if ctx.apps_mode.selected_related.contains(&idx) {
                    ctx.apps_mode.selected_related.remove(&idx);
                } else {
                    ctx.apps_mode.selected_related.insert(idx);
                }
            }
        }
        KeyCode::Char('a') => {
            ctx.apps_mode.selected_related.insert(0);
            for (i, file) in ctx.apps_mode.cached_related_files.iter().enumerate() {
                if !file.category.is_protected() {
                    ctx.apps_mode.selected_related.insert(i + 1);
                }
            }
        }
        KeyCode::Char('n') => {
            ctx.apps_mode.selected_related.clear();
        }
        KeyCode::Enter => execute_uninstall(ctx)?,
        KeyCode::Char('?') => {
            *ctx.prev_mode = Some(*ctx.mode);
            *ctx.mode = AppMode::Help;
        }
        _ => {}
    }
    Ok(())
}

fn execute_uninstall(ctx: &mut UninstallReviewContext) -> Result<()> {
    let app_idx = match ctx.apps_mode.selected_app_idx {
        Some(idx) => idx,
        None => return Ok(()),
    };

    let app = match ctx.apps_mode.apps.get(app_idx) {
        Some(a) => a.clone(),
        None => return Ok(()),
    };

    let selected_related: Vec<_> = ctx
        .apps_mode
        .cached_related_files
        .iter()
        .enumerate()
        .filter(|(i, _)| ctx.apps_mode.selected_related.contains(&(*i + 1)))
        .map(|(_, f)| f.clone())
        .collect();

    let uninstaller = Uninstaller::new(false);
    let result = uninstaller.uninstall(&app, &selected_related)?;

    ctx.apps_mode.uninstall_result = Some(UninstallResultDisplay {
        app_deleted: result.deleted_app,
        related_deleted: result.deleted_related.len(),
        total_freed: result.total_freed,
        errors: result.errors,
    });

    if result.deleted_app {
        ctx.apps_mode.apps.remove(app_idx);
    }

    *ctx.mode = AppMode::UninstallResult;
    ctx.apps_mode.selected_related.clear();
    ctx.apps_mode.selected_app_idx = None;
    ctx.apps_mode.cached_related_files.clear();

    Ok(())
}

pub struct UninstallResultContext<'a> {
    pub apps_mode: &'a mut AppsModeState,
    pub mode: &'a mut AppMode,
}

pub fn handle_uninstall_result_key(ctx: &mut UninstallResultContext, code: KeyCode) -> Result<()> {
    if matches!(code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q')) {
        *ctx.mode = AppMode::AppList;
        ctx.apps_mode.selected_related.clear();
    }
    Ok(())
}
