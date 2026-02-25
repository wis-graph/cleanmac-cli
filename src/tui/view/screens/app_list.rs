use crate::tui::state::AppsModeState;
use crate::tui::view::components::footer::render_app_list_footer;
use crate::utils::format_size;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::ListState;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

pub fn render_app_list(f: &mut Frame, list_state: &mut ListState, apps_mode: &AppsModeState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.area());

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " CleanX ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("App Uninstaller"),
        Span::raw("   "),
        Span::styled(
            format!("{} apps found", apps_mode.apps.len()),
            Style::default().fg(Color::Green),
        ),
    ]))
    .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, chunks[0]);

    let mut items = Vec::new();
    for (i, app) in apps_mode.apps.iter().enumerate() {
        let name = app.name();
        let padded_name = format!("{:<30}", name);

        let size_str = if let Some(&size) = apps_mode.app_sizes.get(&i) {
            format_size(size)
        } else {
            "...".to_string()
        };

        items.push(ListItem::new(Line::from(vec![
            Span::raw(padded_name),
            Span::styled(size_str, Style::default().fg(Color::DarkGray)),
        ])));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::NONE)
                .title(" Applications "),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    f.render_stateful_widget(list, chunks[1], list_state);

    render_app_list_footer(f, chunks[2]);
}
