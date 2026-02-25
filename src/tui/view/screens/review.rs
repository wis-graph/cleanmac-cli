use crate::plugin::registry::{CategoryScanResult, ScanReport};
use crate::plugin::{SafetyLevel, ScanResult};
use crate::tui::state::{ScanProgress, SortMode};
use crate::tui::view::components::footer::render_review_footer;
use crate::utils::{format_number, format_size};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::ListState;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;
use std::collections::HashSet;

pub fn render_review(
    f: &mut Frame,
    list_state: &mut ListState,
    report: &mut Option<ScanReport>,
    selected_items: &HashSet<String>,
    selected_category: &mut usize,
    sort_mode: SortMode,
    scan_progress: &ScanProgress,
    is_scanning: bool,
) {
    let header_height = if is_scanning { 4 } else { 3 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_height),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.area());

    render_header(
        f,
        chunks[0],
        report,
        selected_items,
        scan_progress,
        is_scanning,
        sort_mode,
    );
    render_main(
        f,
        chunks[1],
        list_state,
        report,
        selected_items,
        selected_category,
    );
    render_review_footer(f, chunks[2]);
}

pub fn render_header(
    f: &mut Frame,
    area: Rect,
    report: &Option<ScanReport>,
    selected_items: &HashSet<String>,
    scan_progress: &ScanProgress,
    is_scanning: bool,
    sort_mode: SortMode,
) {
    let total_size: u64 = report.as_ref().map(|r| r.total_size).unwrap_or(0);
    let selected_size: u64 = report
        .as_ref()
        .iter()
        .flat_map(|r| r.categories.iter())
        .flat_map(|c| c.items.iter())
        .filter(|item| selected_items.contains(&item.id))
        .map(|i| i.size)
        .sum();

    let scan_indicator = if is_scanning {
        let done = scan_progress.scanners_done;
        let total = scan_progress.total_scanners;
        let active = scan_progress.active_scanners;
        let current = &scan_progress.current_scanner;

        format!(" [{}/{}c {}/4t|{}]", done, total, active, current)
    } else {
        String::new()
    };

    if is_scanning && area.height >= 4 {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Length(2)])
            .split(area);

        let header = Paragraph::new(Line::from(vec![
            Span::styled(
                " CleanX ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("System Cleaner"),
            Span::raw("   "),
            Span::styled(
                format!(
                    "Total: {} | Selected: {}",
                    format_size(total_size),
                    format_size(selected_size)
                ),
                Style::default().fg(Color::Green),
            ),
            Span::styled(scan_indicator, Style::default().fg(Color::Yellow)),
        ]));
        f.render_widget(header, chunks[0]);

        let current_path = scan_progress.current_path.as_deref().unwrap_or("");
        let truncated = truncate_path_middle(current_path, 80);
        let scan_line = Paragraph::new(Line::from(vec![
            Span::styled(" Scanning: ", Style::default().fg(Color::DarkGray)),
            Span::styled(truncated, Style::default().fg(Color::Gray)),
        ]))
        .block(Block::default().borders(Borders::BOTTOM));
        f.render_widget(scan_line, chunks[1]);
    } else {
        let header = Paragraph::new(Line::from(vec![
            Span::styled(
                " CleanX ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("System Cleaner"),
            Span::raw("   "),
            Span::styled(
                format!(
                    "Total: {} | Selected: {}",
                    format_size(total_size),
                    format_size(selected_size)
                ),
                Style::default().fg(Color::Green),
            ),
            Span::raw("   "),
            Span::styled(
                format!("[{}]", sort_mode.label()),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(scan_indicator, Style::default().fg(Color::Yellow)),
        ]))
        .block(Block::default().borders(Borders::BOTTOM));
        f.render_widget(header, area);
    }
}

fn render_main(
    f: &mut Frame,
    area: Rect,
    list_state: &mut ListState,
    report: &mut Option<ScanReport>,
    selected_items: &HashSet<String>,
    selected_category: &mut usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_sidebar(
        f,
        chunks[0],
        list_state,
        report,
        selected_items,
        selected_category,
    );
    render_detail(
        f,
        chunks[1],
        list_state,
        report,
        selected_items,
        selected_category,
    );
}

fn render_sidebar(
    f: &mut Frame,
    area: Rect,
    list_state: &mut ListState,
    report: &Option<ScanReport>,
    selected_items: &HashSet<String>,
    selected_category: &usize,
) {
    let mut items = Vec::new();

    if let Some(ref report) = report {
        for (i, category) in report.categories.iter().enumerate() {
            let is_selected = i == *selected_category;
            let selected_count = category
                .items
                .iter()
                .filter(|item| selected_items.contains(&item.id))
                .count();

            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let prefix = if is_selected { "> " } else { "  " };
            let count_indicator = if selected_count > 0 {
                format!(" [{}]", selected_count)
            } else {
                String::new()
            };

            items.push(ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(&category.name, style),
                Span::raw(count_indicator),
                Span::raw(" "),
                Span::styled(
                    format!("({})", format_size(category.total_size())),
                    Style::default().fg(Color::DarkGray),
                ),
            ])));

            if is_selected {
                for (idx, item) in category.items.iter().enumerate() {
                    let is_item_selected = selected_items.contains(&item.id);
                    let is_focused = list_state.selected() == Some(idx);
                    let check = if is_item_selected { "[x]" } else { "[ ]" };

                    let safety_color = match item.safety_level {
                        SafetyLevel::Safe => Color::Green,
                        SafetyLevel::Caution => Color::Yellow,
                        SafetyLevel::Protected => Color::Red,
                    };

                    let name_style = if is_focused {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else if is_item_selected {
                        Style::default().fg(Color::White)
                    } else {
                        Style::default()
                    };

                    items.push(ListItem::new(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(check, Style::default().fg(safety_color)),
                        Span::raw(" "),
                        Span::styled(
                            item.path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("?"),
                            name_style,
                        ),
                        Span::raw(" "),
                        Span::styled(
                            format!("({})", format_size(item.size)),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ])));
                }
            }
        }
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::RIGHT)
            .title(" Categories "),
    );

    let mut temp_state = list_state.clone();
    if let Some(item_idx) = list_state.selected() {
        let actual_idx = selected_category + 1 + item_idx;
        temp_state.select(Some(actual_idx));
    }

    f.render_stateful_widget(list, area, &mut temp_state);
}

fn render_detail(
    f: &mut Frame,
    area: Rect,
    list_state: &mut ListState,
    report: &Option<ScanReport>,
    selected_items: &HashSet<String>,
    selected_category: &usize,
) {
    let detail_text = if let Some(ref report) = report {
        if let Some(category) = report.categories.get(*selected_category) {
            if let Some(idx) = list_state.selected() {
                if let Some(item) = category.items.get(idx) {
                    format_item_detail(item)
                } else {
                    format_category_detail(category, selected_items)
                }
            } else {
                format_category_detail(category, selected_items)
            }
        } else {
            "No category selected".to_string()
        }
    } else {
        "No data".to_string()
    };

    let detail = Paragraph::new(detail_text)
        .block(Block::default().borders(Borders::NONE).title(" Details "))
        .wrap(Wrap { trim: true });

    f.render_widget(detail, area);
}

fn format_item_detail(item: &ScanResult) -> String {
    let (safety_str, safety_desc) = match item.safety_level {
        SafetyLevel::Safe => ("Safe", "Can be safely executed"),
        SafetyLevel::Caution => ("Caution", "May affect system behavior"),
        SafetyLevel::Protected => ("Protected", "Cannot be executed"),
    };

    if item.metadata.get("scanner_id").map(|s| s.as_str()) == Some("maintenance") {
        let description = item
            .metadata
            .get("description")
            .cloned()
            .unwrap_or_default();
        let command = item.metadata.get("command").cloned().unwrap_or_default();
        let requires_sudo = item
            .metadata
            .get("requires_sudo")
            .map(|s| s == "true")
            .unwrap_or(false);

        return format!(
            "Task:\n  {}\n\nDescription:\n  {}\n\nCommand:\n  {}\n\nRequires Sudo:\n  {}\n\nSafety Level:\n  {}\n  ({})",
            item.name,
            description,
            command,
            if requires_sudo { "Yes" } else { "No" },
            safety_str,
            safety_desc
        );
    }

    format!(
        "Path:\n  {}\n\nSize:\n  {}\n\nFiles:\n  {}\n\nLast Accessed:\n  {}\n\nLast Modified:\n  {}\n\nSafety Level:\n  {}\n  ({})",
        item.path.display(),
        format_size(item.size),
        format_number(item.file_count),
        item.last_accessed
            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "Unknown".to_string()),
        item.last_modified
            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "Unknown".to_string()),
        safety_str,
        safety_desc
    )
}

fn format_category_detail(
    category: &CategoryScanResult,
    selected_items: &HashSet<String>,
) -> String {
    let selected_count = category
        .items
        .iter()
        .filter(|item| selected_items.contains(&item.id))
        .count();

    let selected_size: u64 = category
        .items
        .iter()
        .filter(|item| selected_items.contains(&item.id))
        .map(|i| i.size)
        .sum();

    format!(
        "Category:\n  {}\n\nTotal Size:\n  {}\n\nItems:\n  {}\n\nSelected:\n  {} items ({})",
        category.name,
        format_size(category.total_size()),
        category.items.len(),
        selected_count,
        format_size(selected_size)
    )
}

fn truncate_path_middle(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        return path.to_string();
    }

    let segments: Vec<&str> = path.split(std::path::MAIN_SEPARATOR).collect();

    if segments.len() <= 4 {
        return path.to_string();
    }

    let head_count = 2;
    let tail_count = 2;

    let head: String = segments[..head_count].join(&std::path::MAIN_SEPARATOR.to_string());
    let tail: String =
        segments[segments.len() - tail_count..].join(&std::path::MAIN_SEPARATOR.to_string());

    format!(
        "{}{}...{}{}",
        head,
        std::path::MAIN_SEPARATOR,
        std::path::MAIN_SEPARATOR,
        tail
    )
}
