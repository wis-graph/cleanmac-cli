use crate::tui::state::{AppMode, AppsModeState};
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::widgets::ListState;

pub struct AppListContext<'a> {
    pub list_state: &'a mut ListState,
    pub apps_mode: &'a mut AppsModeState,
    pub mode: &'a mut AppMode,
    pub prev_mode: &'a mut Option<AppMode>,
    pub should_quit: &'a mut bool,
}

pub fn handle_app_list_key(ctx: &mut AppListContext, code: KeyCode) -> Result<()> {
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
            let max = ctx.apps_mode.apps.len().saturating_sub(1);
            if let Some(current) = ctx.list_state.selected() {
                if current < max {
                    ctx.list_state.select(Some(current + 1));
                }
            }
        }
        KeyCode::Enter => {
            if let Some(idx) = ctx.list_state.selected() {
                ctx.apps_mode.selected_app_idx = Some(idx);
                *ctx.mode = AppMode::LoadingRelatedFiles;
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
