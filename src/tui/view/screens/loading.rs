use ratatui::layout::Alignment;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::tui::view::components::centered_rect;

pub fn render_loading(f: &mut Frame) {
    let area = centered_rect(40, 20, f.area());

    let loading = Paragraph::new(Line::from(vec![Span::styled(
        "Scanning related files...",
        Style::default().fg(Color::Cyan),
    )]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));

    f.render_widget(Clear, area);
    f.render_widget(loading, area);
}
