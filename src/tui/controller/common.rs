use crate::tui::state::AppMode;
use anyhow::Result;
use crossterm::event::KeyCode;

pub struct ConfirmContext<'a> {
    pub mode: &'a mut AppMode,
}

pub fn handle_confirm_key(ctx: &mut ConfirmContext, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Char('y') | KeyCode::Enter => {
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
