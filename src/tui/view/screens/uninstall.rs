use crate::tui::state::AppsModeState;
use crate::tui::view::components::centered_rect;
use crate::tui::view::components::footer::render_uninstall_review_footer;
use crate::utils::format_size;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::ListState;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

pub fn render_uninstall_review(
    f: &mut Frame,
    list_state: &mut ListState,
    apps_mode: &AppsModeState,
) {
    let app_idx = apps_mode.selected_app_idx.unwrap_or(0);
    let app = match apps_mode.apps.get(app_idx) {
        Some(a) => a,
        None => return,
    };

    let related_files = &apps_mode.cached_related_files;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.area());

    let header_text = vec![
        Line::from(vec![
            Span::styled("Uninstall: ", Style::default().fg(Color::Gray)),
            Span::styled(
                app.name(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Size: ", Style::default().fg(Color::Gray)),
            Span::styled(format_size(app.size()), Style::default().fg(Color::Cyan)),
            Span::raw("   "),
            Span::styled("Related: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{} items", related_files.len()),
                Style::default().fg(Color::Green),
            ),
        ]),
    ];

    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::BOTTOM))
        .alignment(Alignment::Center);
    f.render_widget(header, chunks[0]);

    let mut items = Vec::new();

    let app_selected = apps_mode.selected_related.contains(&0);
    let app_name = format!("{}.app", app.name());
    let padded_app_name = format!("{:<35}", app_name);
    let app_size_str = format!("{:>10}", format_size(app.size()));

    items.push(ListItem::new(Line::from(vec![
        Span::styled(
            if app_selected { "[x] " } else { "[ ] " },
            Style::default().fg(Color::Green),
        ),
        Span::styled(
            padded_app_name,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(app_size_str, Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled("[App Bundle]", Style::default().fg(Color::DarkGray)),
    ])));

    for (i, file) in related_files.iter().enumerate() {
        let is_selected = apps_mode.selected_related.contains(&(i + 1));
        let is_protected = file.category.is_protected();

        let check_color = if is_protected {
            Color::Red
        } else if is_selected {
            Color::Green
        } else {
            Color::Gray
        };

        let file_name = file
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?");

        let padded_name = format!("{:<35}", file_name);
        let size_str = format!("{:>10}", format_size(file.size));
        let protected_tag = if is_protected { " (Protected)" } else { "" };

        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                if is_selected { "[x] " } else { "[ ] " },
                Style::default().fg(check_color),
            ),
            Span::raw(padded_name),
            Span::styled(size_str, Style::default().fg(Color::Cyan)),
            Span::raw("  "),
            Span::styled(
                format!("[{}]", file.category.display_name()),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(protected_tag, Style::default().fg(Color::Red)),
        ])));
    }

    let selected_size: u64 = if apps_mode.selected_related.contains(&0) {
        app.size()
    } else {
        0
    } + related_files
        .iter()
        .enumerate()
        .filter(|(i, _)| apps_mode.selected_related.contains(&(*i + 1)))
        .map(|(_, f)| f.size)
        .sum::<u64>();

    let list = List::new(items)
        .block(Block::default().borders(Borders::NONE).title(Span::styled(
            format!("Files to delete ({})", format_size(selected_size)),
            Style::default().fg(Color::Yellow),
        )))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, chunks[1], list_state);

    render_uninstall_review_footer(f, chunks[2]);
}

pub fn render_uninstall_result(f: &mut Frame, apps_mode: &AppsModeState) {
    let area = centered_rect(60, 40, f.area());

    let result = &apps_mode.uninstall_result;

    let text = if let Some(r) = result {
        let mut lines = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                if r.app_deleted {
                    "Uninstalled!"
                } else {
                    "Uninstall Complete"
                },
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
        ];

        if r.app_deleted {
            lines.push(Line::from(vec![
                Span::styled("App: ", Style::default().fg(Color::Gray)),
                Span::styled("Deleted", Style::default().fg(Color::Green)),
            ]));
        }

        lines.push(Line::from(vec![
            Span::styled("Related files: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{} deleted", r.related_deleted),
                Style::default().fg(Color::Green),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Freed: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format_size(r.total_freed),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        if !r.errors.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                format!("Errors: {}", r.errors.len()),
                Style::default().fg(Color::Red),
            )]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press Enter to continue",
            Style::default().fg(Color::DarkGray),
        )));

        lines
    } else {
        vec![Line::from("No result")]
    };

    let paragraph = Paragraph::new(text)
        .block(Block::default().title(" Result ").borders(Borders::ALL))
        .alignment(Alignment::Center);

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}
