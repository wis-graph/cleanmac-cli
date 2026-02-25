use crate::tui::state::CleanResultDisplay;
use crate::tui::view::components::centered_rect;
use crate::utils::format_size;
use ratatui::layout::Alignment;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub struct ConfirmModalData {
    pub selected_count: usize,
    pub total_size: u64,
}

pub fn render_confirm_modal(f: &mut Frame, data: &ConfirmModalData) {
    let area = centered_rect(60, 35, f.area());

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Delete ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{} items", data.selected_count),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" (", Style::default().fg(Color::White)),
            Span::styled(
                format_size(data.total_size),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(")?", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "This action cannot be undone.",
            Style::default().fg(Color::Red),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("[y/Enter]", Style::default().fg(Color::Green)),
            Span::raw(" Confirm     "),
            Span::styled("[n/Esc]", Style::default().fg(Color::Red)),
            Span::raw(" Cancel"),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Confirm Clean ")
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Center);

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

pub fn render_result_modal(f: &mut Frame, result: Option<&CleanResultDisplay>) {
    let area = centered_rect(60, 40, f.area());

    let text = if let Some(r) = result {
        vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "Clean Complete!",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Cleaned: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{} items", r.success_count),
                    Style::default().fg(Color::Green),
                ),
            ]),
            Line::from(vec![
                Span::styled("Failed: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{} items", r.failed_count),
                    Style::default().fg(if r.failed_count > 0 {
                        Color::Red
                    } else {
                        Color::Green
                    }),
                ),
            ]),
            Line::from(vec![
                Span::styled("Freed: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format_size(r.total_freed),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Duration: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format!("{:.2}s", r.duration.as_secs_f64()),
                    Style::default().fg(Color::Gray),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Press Enter to continue",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    } else {
        vec![Line::from("No result")]
    };

    let paragraph = Paragraph::new(text)
        .block(Block::default().title(" Result ").borders(Borders::ALL))
        .alignment(Alignment::Center);

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

pub fn render_help_modal(f: &mut Frame) {
    let area = centered_rect(65, 65, f.area());

    let help_text = vec![
        Line::from(vec![Span::styled(
            "CleanX Help",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  ↑/↓    ", Style::default().fg(Color::Cyan)),
            Span::raw("Navigate items"),
        ]),
        Line::from(vec![
            Span::styled("  ←/→    ", Style::default().fg(Color::Cyan)),
            Span::raw("Switch category"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Selection",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  Space  ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle selection"),
        ]),
        Line::from(vec![
            Span::styled("  a      ", Style::default().fg(Color::Cyan)),
            Span::raw("Select all in category"),
        ]),
        Line::from(vec![
            Span::styled("  n      ", Style::default().fg(Color::Cyan)),
            Span::raw("Deselect all"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  Enter  ", Style::default().fg(Color::Cyan)),
            Span::raw("Clean selected"),
        ]),
        Line::from(vec![
            Span::styled("  r      ", Style::default().fg(Color::Cyan)),
            Span::raw("Rescan"),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Safety Levels",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("  ● Safe     ", Style::default().fg(Color::Green)),
            Span::raw("Can be deleted"),
        ]),
        Line::from(vec![
            Span::styled("  ● Caution  ", Style::default().fg(Color::Yellow)),
            Span::raw("May affect apps"),
        ]),
        Line::from(vec![
            Span::styled("  ● Protected", Style::default().fg(Color::Red)),
            Span::raw("Cannot delete"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  ?      ", Style::default().fg(Color::Cyan)),
            Span::raw("Show this help"),
        ]),
        Line::from(vec![
            Span::styled("  q      ", Style::default().fg(Color::Cyan)),
            Span::raw("Quit"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press q, Esc, or ? to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph =
        Paragraph::new(help_text).block(Block::default().title(" Help ").borders(Borders::ALL));

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}
