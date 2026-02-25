use crate::tui::service::disk::start_space_scan;
use crate::tui::state::{AppMode, CachedScan, SpaceLensState};
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::widgets::ListState;

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
        KeyCode::Char('?') => {
            *ctx.prev_mode = Some(*ctx.mode);
            *ctx.mode = AppMode::Help;
        }
        _ => {}
    }
    Ok(())
}
