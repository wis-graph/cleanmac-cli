use crate::plugin::registry::ScanReport;
use crate::tui::state::ScannerInfo;
use crate::tui::view::components::footer::render_category_select_footer;
use crate::utils::format_size;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::ListState;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;
use std::collections::HashSet;

pub struct CategorySelectData<'a> {
    pub list_state: &'a mut ListState,
    pub available_scanners: &'a [ScannerInfo],
    pub report: Option<&'a ScanReport>,
}

pub fn render_category_select(f: &mut Frame, data: &mut CategorySelectData) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Percentage(100),
            Constraint::Length(2),
        ])
        .split(area);

    let has_cached = data.report.is_some();
    let cached_size = data.report.map(|r| r.total_size).unwrap_or(0);
    let cached_items = data.report.map(|r| r.total_items).unwrap_or(0);

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " CleanX ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Select Categories to Scan"),
        if has_cached {
            Span::styled(
                format!(
                    " (Cached: {} / {} items)",
                    format_size(cached_size),
                    cached_items
                ),
                Style::default().fg(Color::DarkGray),
            )
        } else {
            Span::raw("")
        },
    ]))
    .block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(title, chunks[0]);

    let scanned_ids: HashSet<String> = data
        .report
        .map(|r| r.categories.iter().map(|c| c.scanner_id.clone()).collect())
        .unwrap_or_default();

    let enabled_count = data.available_scanners.iter().filter(|s| s.enabled).count();
    let items: Vec<ListItem> = data
        .available_scanners
        .iter()
        .map(|scanner| {
            let check = if scanner.enabled { "[x]" } else { "[ ]" };
            let is_scanned = scanned_ids.contains(&scanner.id);
            let scanned_cat = data
                .report
                .and_then(|r| r.categories.iter().find(|c| c.scanner_id == scanner.id));

            let style = if scanner.enabled {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let scan_indicator = if let Some(cat) = scanned_cat {
                Span::styled(
                    format!(" ({})", format_size(cat.total_size())),
                    Style::default().fg(Color::Yellow),
                )
            } else {
                Span::raw("")
            };

            let cached_mark = if is_scanned {
                Span::styled(" ✓", Style::default().fg(Color::Cyan))
            } else {
                Span::raw("")
            };

            ListItem::new(Line::from(vec![
                Span::styled(check, Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::styled(&scanner.name, style),
                scan_indicator,
                cached_mark,
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(
                    " Categories ({}/{} enabled) ",
                    enabled_count,
                    data.available_scanners.len()
                ))
                .borders(Borders::NONE),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    f.render_stateful_widget(list, chunks[1], data.list_state);

    let has_viewable = data
        .report
        .map(|r| !r.categories.is_empty())
        .unwrap_or(false);
    render_category_select_footer(f, chunks[2], has_cached && has_viewable, cached_size);
}
