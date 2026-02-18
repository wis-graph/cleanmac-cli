# TUI 상세 스펙

## 개요

ratatui 기반 대화형 터미널 UI 구현

## 레이아웃 구조

```
┌──────────────────────────────────────────────────────────────────┐
│  Header (3 lines)                                                │
├────────────────────────────┬─────────────────────────────────────┤
│                            │                                     │
│  Sidebar (Categories)      │  Detail Panel                      │
│  ~40%                      │  ~60%                               │
│                            │                                     │
│                            │                                     │
│                            │                                     │
├────────────────────────────┴─────────────────────────────────────┤
│  Footer (3 lines)                                                │
│  - Keybindings                                                   │
│  - Status                                                        │
└──────────────────────────────────────────────────────────────────┘
```

## 컴포넌트 구조

```rust
pub struct App {
    // 상태
    mode: AppMode,
    config: Config,
    
    // 데이터
    categories: Vec<CategoryData>,
    selected_category: usize,
    selected_items: HashSet<String>,  // 다중 선택
    
    // UI 상태
    focus: Focus,
    list_state: ListState,
    detail_scroll: u16,
    
    // 백그라운드
    scan_progress: Option<ScanProgress>,
    
    // 서비스
    plugin_registry: PluginRegistry,
    safety_checker: SafetyChecker,
    history_logger: HistoryLogger,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Scanning,
    Review,
    Cleaning,
    Help,
    ConfirmClean,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Categories,
    Items,
    Details,
}
```

## 1. Header 컴포넌트

```rust
pub fn render_header(&self, f: &mut Frame, area: Rect) {
    let total_size: u64 = self.categories.iter().map(|c| c.total_size).sum();
    let selected_size: u64 = self.get_selected_items()
        .iter()
        .map(|i| i.size)
        .sum();
    
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    
    // 왼쪽: 로고 + 타이틀
    let title = Paragraph::new(Line::from(vec![
        Span::styled(" CleanX ", Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)),
        Span::raw("System Cleaner"),
    ]));
    f.render_widget(title, chunks[0]);
    
    // 오른쪽: 상태
    let status = match self.mode {
        AppMode::Scanning => format!("Scanning... {}", self.scan_progress.as_ref().map(|p| p.current()).unwrap_or("")),
        _ => format!("Total: {} | Selected: {}", 
            format_size(total_size),
            format_size(selected_size)),
    };
    
    let status_widget = Paragraph::new(status)
        .alignment(Alignment::Right)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(status_widget, chunks[1]);
}
```

## 2. Sidebar 컴포넌트

```rust
pub fn render_sidebar(&mut self, f: &mut Frame, area: Rect) {
    let items: Vec<ListItem> = self.categories
        .iter()
        .enumerate()
        .map(|(i, category)| {
            let is_selected = i == self.selected_category;
            let selected_count = category.items.iter()
                .filter(|item| self.selected_items.contains(&item.id))
                .count();
            
            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            
            let prefix = if is_selected { "▶ " } else { "  " };
            let count_indicator = if selected_count > 0 {
                format!(" [{}]", selected_count)
            } else {
                String::new()
            };
            
            ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(&category.name, style),
                Span::raw(count_indicator),
                Span::raw(" "),
                Span::styled(
                    format!("({})", format_size(category.total_size)),
                    Style::default().fg(Color::DarkGray)
                ),
            ]))
        })
        .collect();
    
    // 하위 아이템 표시
    if let Some(category) = self.categories.get(self.selected_category) {
        for item in &category.items {
            let is_item_selected = self.selected_items.contains(&item.id);
            
            items.push(ListItem::new(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    if is_item_selected { "[x] " } else { "[ ] " },
                    Style::default().fg(Color::Green)
                ),
                Span::raw(item.name.as_str()),
                Span::raw(" "),
                Span::styled(
                    format!("({})", format_size(item.size)),
                    Style::default().fg(Color::DarkGray)
                ),
            ])));
        }
    }
    
    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::RIGHT)
            .title(" Categories "));
    
    f.render_stateful_widget(list, area, &mut self.list_state);
}
```

## 3. Detail Panel 컴포넌트

```rust
pub fn render_detail(&self, f: &mut Frame, area: Rect) {
    let detail = if let Some(item) = self.get_focused_item() {
        self.render_item_detail(item)
    } else if let Some(category) = self.categories.get(self.selected_category) {
        self.render_category_detail(category)
    } else {
        Paragraph::new("Select an item to view details")
    };
    
    f.render_widget(detail, area);
}

fn render_item_detail(&self, item: &ScanResult) -> Paragraph<'static> {
    let safety_color = match item.safety_level {
        SafetyLevel::Safe => Color::Green,
        SafetyLevel::Caution => Color::Yellow,
        SafetyLevel::Protected => Color::Red,
    };
    
    let lines = vec![
        Line::from(vec![
            Span::styled("Path: ", Style::default().fg(Color::Gray)),
            Span::raw(item.path.display().to_string()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Size: ", Style::default().fg(Color::Gray)),
            Span::styled(format_size(item.size), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Files: ", Style::default().fg(Color::Gray)),
            Span::raw(format_number(item.file_count)),
        ]),
        Line::from(vec![
            Span::styled("Folders: ", Style::default().fg(Color::Gray)),
            Span::raw(format_number(item.dir_count)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Last Accessed: ", Style::default().fg(Color::Gray)),
            Span::raw(item.last_accessed
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Unknown".to_string())),
        ]),
        Line::from(vec![
            Span::styled("Last Modified: ", Style::default().fg(Color::Gray)),
            Span::raw(item.last_modified
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Unknown".to_string())),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Safety: ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:?}", item.safety_level),
                Style::default().fg(safety_color)
            ),
        ]),
    ];
    
    Paragraph::new(lines)
        .block(Block::default()
            .borders(Borders::NONE)
            .title(" Details "))
        .wrap(Wrap { trim: true })
}
```

## 4. Footer 컴포넌트

```rust
pub fn render_footer(&self, f: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(2)])
        .split(area);
    
    // 상태 바
    let selected_count = self.selected_items.len();
    let selected_size = self.get_selected_items()
        .iter()
        .map(|i| i.size)
        .sum();
    
    let status = if selected_count > 0 {
        format!("Selected: {} items ({})", selected_count, format_size(selected_size))
    } else {
        "No items selected".to_string()
    };
    
    let status_bar = Paragraph::new(status)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(status_bar, chunks[0]);
    
    // 키 바인딩
    let keybindings = match self.mode {
        AppMode::Review => vec![
            ("↑↓", "Navigate"),
            ("Space", "Select"),
            ("Enter", "Clean"),
            ("Tab", "Focus"),
            ("?", "Help"),
            ("q", "Quit"),
        ],
        AppMode::ConfirmClean => vec![
            ("y", "Confirm"),
            ("n", "Cancel"),
        ],
        AppMode::Help => vec![
            ("q/Esc", "Close"),
        ],
        _ => vec![],
    };
    
    let key_widgets: Vec<Span> = keybindings
        .iter()
        .flat_map(|(key, desc)| {
            vec![
                Span::styled(*key, Style::default().fg(Color::Cyan)),
                Span::raw(format!(" {}  ", desc)),
            ]
        })
        .collect();
    
    let key_bar = Paragraph::new(Line::from(key_widgets));
    f.render_widget(key_bar, chunks[1]);
}
```

## 5. 키 이벤트 처리

```rust
pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
    match self.mode {
        AppMode::Review => self.handle_review_key(key),
        AppMode::ConfirmClean => self.handle_confirm_key(key),
        AppMode::Help => self.handle_help_key(key),
        AppMode::Cleaning => Ok(()), // 블로킹
        AppMode::Scanning => Ok(()), // 블로킹
    }
}

fn handle_review_key(&mut self, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('q') => {
            self.should_quit = true;
        }
        KeyCode::Up => self.navigate_up(),
        KeyCode::Down => self.navigate_down(),
        KeyCode::Left => self.navigate_category_prev(),
        KeyCode::Right => self.navigate_category_next(),
        KeyCode::Char(' ') => self.toggle_selection(),
        KeyCode::Enter => self.start_clean_confirm(),
        KeyCode::Tab => self.cycle_focus(),
        KeyCode::Char('?') => self.mode = AppMode::Help,
        _ => {}
    }
    Ok(())
}

fn toggle_selection(&mut self) {
    if let Some(item) = self.get_focused_item() {
        let id = item.id.clone();
        if self.selected_items.contains(&id) {
            self.selected_items.remove(&id);
        } else {
            self.selected_items.insert(id);
        }
    }
}

fn start_clean_confirm(&mut self) {
    if !self.selected_items.is_empty() {
        self.mode = AppMode::ConfirmClean;
    }
}

fn handle_confirm_key(&mut self, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('y') => {
            self.execute_clean()?;
            self.mode = AppMode::Review;
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            self.mode = AppMode::Review;
        }
        _ => {}
    }
    Ok(())
}
```

## 6. 클린 실행 모달

```rust
pub fn render_clean_confirm(&self, f: &mut Frame) {
    let selected = self.get_selected_items();
    let total_size: u64 = selected.iter().map(|i| i.size).sum();
    
    let area = centered_rect(60, 40, f.size());
    
    let block = Block::default()
        .title(" Confirm Clean ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black));
    
    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("You are about to delete "),
            Span::styled(format!("{} items", selected.len()), Style::default().fg(Color::Yellow)),
            Span::raw(" totaling "),
            Span::styled(format_size(total_size), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from("This action cannot be undone."),
        Line::from(""),
        Line::from(vec![
            Span::styled("[y]", Style::default().fg(Color::Green)),
            Span::raw(" Confirm  "),
            Span::styled("[n]", Style::default().fg(Color::Red)),
            Span::raw(" Cancel"),
        ]),
    ];
    
    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);
    
    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
```

## 7. 도움말 모달

```rust
pub fn render_help(&self, f: &mut Frame) {
    let area = centered_rect(70, 70, f.size());
    
    let help_text = vec![
        Line::from(vec![
            Span::styled("CleanX Help", Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD))
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Navigation", Style::default().add_modifier(Modifier::BOLD))]),
        Line::from("  ↑/↓     Navigate items"),
        Line::from("  ←/→     Switch category"),
        Line::from("  Tab     Change focus"),
        Line::from(""),
        Line::from(vec![Span::styled("Selection", Style::default().add_modifier(Modifier::BOLD))]),
        Line::from("  Space   Toggle selection"),
        Line::from("  a       Select all"),
        Line::from("  n       Deselect all"),
        Line::from(""),
        Line::from(vec![Span::styled("Actions", Style::default().add_modifier(Modifier::BOLD))]),
        Line::from("  Enter   Clean selected"),
        Line::from("  r       Rescan"),
        Line::from("  ?       Show this help"),
        Line::from("  q       Quit"),
        Line::from(""),
        Line::from(vec![Span::styled("Safety Levels", Style::default().add_modifier(Modifier::BOLD))]),
        Line::from(vec![
            Span::styled("  ● Safe      ", Style::default().fg(Color::Green)),
            Span::raw("Can be safely deleted"),
        ]),
        Line::from(vec![
            Span::styled("  ● Caution   ", Style::default().fg(Color::Yellow)),
            Span::raw("May affect some apps"),
        ]),
        Line::from(vec![
            Span::styled("  ● Protected ", Style::default().fg(Color::Red)),
            Span::raw("Cannot be deleted"),
        ]),
    ];
    
    let paragraph = Paragraph::new(help_text)
        .block(Block::default()
            .title(" Help (press q or Esc to close) ")
            .borders(Borders::ALL));
    
    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}
```

## 8. 메인 루프

```rust
pub fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
    // 초기 스캔
    self.start_scan();
    
    loop {
        terminal.draw(|f| self.render(f))?;
        
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                self.handle_key(key)?;
            }
        }
        
        // 백그라운드 스캔 완료 확인
        if let Some(progress) = &self.scan_progress {
            if progress.is_complete() {
                self.finish_scan();
            }
        }
        
        if self.should_quit {
            break;
        }
    }
    
    Ok(())
}

pub fn render(&mut self, f: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.size());
    
    self.render_header(f, chunks[0]);
    
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[1]);
    
    self.render_sidebar(f, main_chunks[0]);
    self.render_detail(f, main_chunks[1]);
    
    self.render_footer(f, chunks[2]);
    
    // 모달 오버레이
    match self.mode {
        AppMode::ConfirmClean => self.render_clean_confirm(f),
        AppMode::Help => self.render_help(f),
        _ => {}
    }
}
```

---

**버전**: 1.0
**작성일**: 2026-02-18
