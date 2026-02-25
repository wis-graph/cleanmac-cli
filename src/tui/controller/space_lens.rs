use crate::tui::service::disk::start_space_scan;
use crate::tui::state::{
    AppMode, CachedScan, DeleteResult, FolderEntry, SpaceLensMode, SpaceLensState,
};
use crate::utils::format_size;
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::widgets::ListState;
use std::fs;

pub struct SpaceLensContext<'a> {
    pub list_state: &'a mut ListState,
    pub space_lens: &'a mut SpaceLensState,
    pub mode: &'a mut AppMode,
    pub prev_mode: &'a mut Option<AppMode>,
    pub should_quit: &'a mut bool,
}

fn cache_current_if_needed(state: &mut SpaceLensState) {
    if !state.entries.is_empty() {
        let path = state.current_path.clone();
        if !state.cache.contains_key(&path) {
            state.cache.insert(
                path,
                CachedScan {
                    entries: state.entries.clone(),
                    total_size: state.total_size,
                    was_loading: state.loading,
                },
            );
        }
    }
}

pub fn handle_space_lens_key(ctx: &mut SpaceLensContext, code: KeyCode) -> Result<()> {
    match ctx.space_lens.delete_mode {
        SpaceLensMode::ConfirmDelete => handle_confirm_key(ctx, code),
        SpaceLensMode::ShowResult => handle_result_key(ctx, code),
        SpaceLensMode::Browse => handle_browse_key(ctx, code),
    }
}

fn handle_browse_key(ctx: &mut SpaceLensContext, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Char('q') => {
            if let Some(prev) = *ctx.prev_mode {
                *ctx.mode = prev;
                *ctx.prev_mode = None;
            } else {
                *ctx.should_quit = true;
            }
        }
        KeyCode::Up => {
            if let Some(current) = ctx.list_state.selected() {
                if current > 0 {
                    ctx.list_state.select(Some(current - 1));
                }
            }
        }
        KeyCode::Down => {
            let max = ctx.space_lens.entries.len().saturating_sub(1);
            if let Some(current) = ctx.list_state.selected() {
                if current < max {
                    ctx.list_state.select(Some(current + 1));
                }
            }
        }
        KeyCode::Enter => {
            if let Some(idx) = ctx.list_state.selected() {
                if let Some(entry) = ctx.space_lens.entries.get(idx) {
                    if entry.is_dir {
                        let new_path = entry.path.clone();
                        cache_current_if_needed(ctx.space_lens);
                        ctx.space_lens.current_path = new_path;
                        ctx.list_state.select(Some(0));
                        start_space_scan(ctx.space_lens);
                    }
                }
            }
        }
        KeyCode::Esc | KeyCode::Backspace => {
            if let Some(parent) = ctx.space_lens.current_path.parent() {
                if parent != ctx.space_lens.current_path {
                    let new_path = parent.to_path_buf();
                    cache_current_if_needed(ctx.space_lens);
                    ctx.space_lens.current_path = new_path;
                    ctx.list_state.select(Some(0));
                    start_space_scan(ctx.space_lens);
                } else if let Some(prev) = *ctx.prev_mode {
                    *ctx.mode = prev;
                    *ctx.prev_mode = None;
                }
            } else if let Some(prev) = *ctx.prev_mode {
                *ctx.mode = prev;
                *ctx.prev_mode = None;
            }
        }
        KeyCode::Char('r') => {
            ctx.space_lens.cache.remove(&ctx.space_lens.current_path);
            ctx.space_lens.entries.clear();
            ctx.list_state.select(Some(0));
            start_space_scan(ctx.space_lens);
        }
        KeyCode::Char('p') => {
            ctx.space_lens.parallel_scan = !ctx.space_lens.parallel_scan;
            if !ctx.space_lens.parallel_scan {
                ctx.space_lens.pending_scans.clear();
            }
        }
        KeyCode::Char('d') => {
            if let Some(idx) = ctx.list_state.selected() {
                if let Some(entry) = ctx.space_lens.entries.get(idx).cloned() {
                    ctx.space_lens.pending_delete = Some(entry);
                    ctx.space_lens.delete_mode = SpaceLensMode::ConfirmDelete;
                }
            }
        }
        KeyCode::Char('?') => {
            *ctx.prev_mode = Some(*ctx.mode);
            *ctx.mode = AppMode::Help;
        }
        _ => {}
    }
    Ok(())
}

fn handle_confirm_key(ctx: &mut SpaceLensContext, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Char('y') | KeyCode::Enter => {
            if let Some(entry) = ctx.space_lens.pending_delete.take() {
                let result = delete_entry(&entry);
                ctx.space_lens.entries.retain(|e| e.path != entry.path);
                ctx.space_lens.total_size = ctx.space_lens.entries.iter().map(|e| e.size).sum();
                ctx.space_lens.cache.remove(&ctx.space_lens.current_path);

                if ctx.list_state.selected().unwrap_or(0) >= ctx.space_lens.entries.len() {
                    ctx.list_state
                        .select(Some(ctx.space_lens.entries.len().saturating_sub(1)));
                }

                ctx.space_lens.delete_result = Some(result);
                ctx.space_lens.delete_mode = SpaceLensMode::ShowResult;
            } else {
                ctx.space_lens.delete_mode = SpaceLensMode::Browse;
            }
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            ctx.space_lens.pending_delete = None;
            ctx.space_lens.delete_mode = SpaceLensMode::Browse;
        }
        _ => {}
    }
    Ok(())
}

fn handle_result_key(ctx: &mut SpaceLensContext, code: KeyCode) -> Result<()> {
    if matches!(code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q')) {
        ctx.space_lens.delete_result = None;
        ctx.space_lens.delete_mode = SpaceLensMode::Browse;
    }
    Ok(())
}

fn delete_entry(entry: &FolderEntry) -> DeleteResult {
    let path = &entry.path;
    let size = entry.size;

    let result = if path.is_dir() {
        fs::remove_dir_all(path)
    } else if path.exists() {
        fs::remove_file(path)
    } else {
        Ok(())
    };

    match result {
        Ok(()) => DeleteResult {
            path: path.clone(),
            success: true,
            size,
            error: None,
        },
        Err(e) => DeleteResult {
            path: path.clone(),
            success: false,
            size: 0,
            error: Some(e.to_string()),
        },
    }
}
