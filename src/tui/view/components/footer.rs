use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn render_review_footer(f: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Cyan)),
        Span::raw(" Nav  "),
        Span::styled("←→", Style::default().fg(Color::Cyan)),
        Span::raw(" Cat  "),
        Span::styled("s", Style::default().fg(Color::Cyan)),
        Span::raw(" Sort  "),
        Span::styled("v", Style::default().fg(Color::Cyan)),
        Span::raw(" Space  "),
        Span::styled("Tab", Style::default().fg(Color::Cyan)),
        Span::raw(" Cats  "),
        Span::styled("Space", Style::default().fg(Color::Cyan)),
        Span::raw(" Select  "),
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(" Clean  "),
        Span::styled("?", Style::default().fg(Color::Cyan)),
        Span::raw(" Help"),
    ]))
    .block(Block::default().borders(Borders::TOP));

    f.render_widget(footer, area);
}

pub fn render_category_select_footer(
    f: &mut Frame,
    area: Rect,
    has_cached: bool,
    cached_size: u64,
) {
    use crate::utils::format_size;

    let mut footer_spans = vec![
        Span::styled("↑↓", Style::default().fg(Color::Cyan)),
        Span::raw(" Nav  "),
        Span::styled("Space", Style::default().fg(Color::Cyan)),
        Span::raw(" Toggle  "),
        Span::styled("r", Style::default().fg(Color::Cyan)),
        Span::raw(" Scan  "),
        Span::styled("a", Style::default().fg(Color::Cyan)),
        Span::raw(" All  "),
        Span::styled("n", Style::default().fg(Color::Cyan)),
        Span::raw(" None  "),
    ];

    if has_cached {
        footer_spans.push(Span::styled("Tab", Style::default().fg(Color::Cyan)));
        footer_spans.push(Span::raw(" View  "));
    }

    footer_spans.push(Span::styled("q", Style::default().fg(Color::Cyan)));
    footer_spans.push(Span::raw(" Quit"));

    if has_cached {
        footer_spans.push(Span::raw("  "));
        footer_spans.push(Span::styled(
            format!("| {} cached", format_size(cached_size)),
            Style::default().fg(Color::Green),
        ));
    }

    let footer =
        Paragraph::new(Line::from(footer_spans)).block(Block::default().borders(Borders::TOP));
    f.render_widget(footer, area);
}

pub fn render_app_list_footer(f: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Cyan)),
        Span::raw(" Navigate  "),
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(" Select  "),
        Span::styled("?", Style::default().fg(Color::Cyan)),
        Span::raw(" Help  "),
        Span::styled("q", Style::default().fg(Color::Cyan)),
        Span::raw(" Quit"),
    ]))
    .block(Block::default().borders(Borders::TOP));
    f.render_widget(footer, area);
}

pub fn render_uninstall_review_footer(f: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Cyan)),
        Span::raw(" Nav  "),
        Span::styled("Space", Style::default().fg(Color::Cyan)),
        Span::raw(" Toggle  "),
        Span::styled("a", Style::default().fg(Color::Cyan)),
        Span::raw(" All  "),
        Span::styled("n", Style::default().fg(Color::Cyan)),
        Span::raw(" None  "),
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(" Delete  "),
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::raw(" Back"),
    ]))
    .block(Block::default().borders(Borders::TOP));
    f.render_widget(footer, area);
}

pub fn render_space_lens_footer(f: &mut Frame, area: Rect, parallel: bool) {
    let mode_indicator = if parallel {
        Span::styled(" [Parallel]", Style::default().fg(Color::Yellow))
    } else {
        Span::styled(" [Single]", Style::default().fg(Color::DarkGray))
    };

    let footer = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Cyan)),
        Span::raw(" Nav  "),
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(" Open  "),
        Span::styled("Esc/⌫", Style::default().fg(Color::Cyan)),
        Span::raw(" Up/Back  "),
        Span::styled("r", Style::default().fg(Color::Cyan)),
        Span::raw(" Refresh  "),
        Span::styled("p", Style::default().fg(Color::Cyan)),
        Span::raw(" Parallel  "),
        Span::styled("q", Style::default().fg(Color::Cyan)),
        Span::raw(" Exit"),
        mode_indicator,
    ]))
    .block(Block::default().borders(Borders::TOP));
    f.render_widget(footer, area);
}
