use crate::tui::service::disk::get_active_threads;
use crate::tui::state::{DeleteResult, SpaceLensMode, SpaceLensState};
use crate::tui::view::components::footer::render_space_lens_footer;
use crate::tui::view::components::utils::centered_rect;
use crate::utils::format_size;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::ListState;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

pub fn render_space_lens(
    f: &mut Frame,
    list_state: &mut ListState,
    space_lens: &mut SpaceLensState,
) {
    if !space_lens.entries.is_empty() && list_state.selected().is_none() {
        list_state.select(Some(0));
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.area());

    let path_str = space_lens.current_path.display().to_string();

    let thread_info = if space_lens.parallel_scan {
        let actual_threads = match space_lens.thread_count {
            1..=4 => 4,
            5..=8 => 8,
            _ => 16,
        };
        let active = get_active_threads(space_lens.thread_count);
        format!(" [{}t | {}/{}]", actual_threads, active, actual_threads)
    } else {
        " [single]".to_string()
    };

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " CleanX ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Space Lens"),
        Span::styled(&thread_info, Style::default().fg(Color::Yellow)),
        Span::raw("  "),
        Span::styled(&path_str, Style::default().fg(Color::Green)),
    ]))
    .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, chunks[0]);

    let max_size = space_lens.entries.iter().map(|e| e.size).max().unwrap_or(1);

    let bar_width = 20u16;
    let selected_idx = list_state.selected();

    let items: Vec<ListItem> = space_lens
        .entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let is_focused = selected_idx == Some(idx);

            let filled = if max_size > 0 {
                ((entry.size as f64 / max_size as f64) * bar_width as f64) as usize
            } else {
                0
            };
            let bar: String = "█".repeat(filled) + &"░".repeat(bar_width as usize - filled);

            let name_style = if is_focused {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let dir_indicator = if entry.is_dir { "/" } else { "" };
            let size_percent = if space_lens.total_size > 0 && entry.size > 0 {
                (entry.size as f64 / space_lens.total_size as f64 * 100.0) as u8
            } else {
                0
            };

            let (size_text, size_style) = if entry.is_dir && entry.size == 0 {
                ("...".to_string(), Style::default().fg(Color::DarkGray))
            } else {
                (format_size(entry.size), Style::default().fg(Color::Green))
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<30}", format!("{}{}", entry.name, dir_indicator)),
                    name_style,
                ),
                Span::styled(bar, Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::styled(format!("{:>12}", size_text), size_style),
                Span::styled(
                    format!(" {:>3}%", size_percent),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let list_title = if space_lens.loading {
        "Scanning..."
    } else if space_lens.cache.contains_key(&space_lens.current_path) {
        "Contents (cached)"
    } else {
        "Contents"
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::NONE).title(Span::styled(
            format!("{} ({})", list_title, format_size(space_lens.total_size)),
            Style::default().fg(Color::Yellow),
        )))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    f.render_stateful_widget(list, chunks[1], list_state);

    render_space_lens_footer(f, chunks[2], space_lens.parallel_scan);

    match space_lens.delete_mode {
        SpaceLensMode::ConfirmDelete => {
            if let Some(ref entry) = space_lens.pending_delete {
                render_delete_confirm_modal(f, entry);
            }
        }
        SpaceLensMode::ShowResult => {
            if let Some(ref result) = space_lens.delete_result {
                render_delete_result_modal(f, result);
            }
        }
        SpaceLensMode::Browse => {}
    }
}

fn render_delete_confirm_modal(f: &mut Frame, entry: &crate::tui::state::FolderEntry) {
    let area = centered_rect(60, 35, f.area());

    let dir_text = if entry.is_dir { "folder" } else { "file" };
    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Delete ", Style::default().fg(Color::White)),
            Span::styled(dir_text, Style::default().fg(Color::Yellow)),
            Span::styled("?", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            &entry.name,
            Style::default().fg(Color::Cyan),
        )]),
        Line::from(vec![Span::styled(
            format_size(entry.size),
            Style::default().fg(Color::Green),
        )]),
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
                .title(" Confirm Delete ")
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Center);

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

fn render_delete_result_modal(f: &mut Frame, result: &DeleteResult) {
    let area = centered_rect(60, 30, f.area());

    let text = if result.success {
        vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "Deleted Successfully!",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Freed: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    format_size(result.size),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Press Enter to continue",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "Delete Failed!",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Error: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    result.error.as_deref().unwrap_or("Unknown error"),
                    Style::default().fg(Color::Red),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Press Enter to continue",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    };

    let paragraph = Paragraph::new(text)
        .block(Block::default().title(" Result ").borders(Borders::ALL))
        .alignment(Alignment::Center);

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}
