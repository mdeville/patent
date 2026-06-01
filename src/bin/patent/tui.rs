//! ratatui interface (M5).
//!
//! Header (idea + sources-checked transparency line), verdict panel
//! (🟢/🟡/🔴 + headline + gaps + caveat), and a scrollable/filterable matches
//! table. `↑/↓` scroll, `/` filter, `Enter` open URL, `q` quit.

use patent::model::{Match, Saturation, Verdict};
use patent::tui::{App, Mode};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, Wrap},
    DefaultTerminal, Frame,
};

fn level_icon(level: Saturation) -> &'static str {
    match level {
        Saturation::Open => "🟢",
        Saturation::Crowded => "🟡",
        Saturation::Saturated => "🔴",
    }
}

fn level_color(level: Saturation) -> Color {
    match level {
        Saturation::Open => Color::Green,
        Saturation::Crowded => Color::Yellow,
        Saturation::Saturated => Color::Red,
    }
}

fn draw(frame: &mut Frame, app: &App) {
    let verdict = app.verdict();
    let gap_rows = verdict.gaps.len() as u16;
    let verdict_height = (gap_rows + 7).min(frame.area().height / 3);

    let [header_area, verdict_area, table_area, help_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(verdict_height),
        Constraint::Min(5),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    // -- header
    let sources_str: String = verdict
        .sources_checked
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            format!(" 🔍 {}", app.idea()),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::raw(format!(" Sources checked: {sources_str}"))),
    ])
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, header_area);

    // -- verdict panel
    let color = level_color(verdict.level);
    let mut lines = vec![
        Line::from(Span::styled(
            format!(
                " {} {:?} — {}",
                level_icon(verdict.level),
                verdict.level,
                verdict.headline,
            ),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
    ];
    for gap in &verdict.gaps {
        lines.push(Line::from(Span::raw(format!("  • {gap}"))));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        format!(" ⚠ {}", verdict.caveat),
        Style::default().fg(Color::DarkGray),
    )));
    let verdict_panel = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::BOTTOM).title(" Verdict "));
    frame.render_widget(verdict_panel, verdict_area);

    // -- matches table
    let visible = app.visible_matches();
    let selected_style = Style::default()
        .bg(Color::DarkGray)
        .add_modifier(Modifier::BOLD);
    let rows: Vec<Row> = visible
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let row = Row::new(vec![
                format!("{:.2}", m.similarity),
                m.name.clone(),
                m.source.to_string(),
                m.description.clone(),
            ]);
            if i == app.cursor() {
                row.style(selected_style)
            } else {
                row
            }
        })
        .collect();

    let filter_indicator = if app.mode() == Mode::Filter {
        format!(" Matches [/{}] ", app.filter_text())
    } else if !app.filter_text().is_empty() {
        format!(" Matches [{}] ", app.filter_text())
    } else {
        " Matches ".to_string()
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(6),
            Constraint::Length(20),
            Constraint::Length(12),
            Constraint::Min(20),
        ],
    )
    .header(
        Row::new(vec!["Score", "Name", "Source", "Description"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(filter_indicator),
    );
    frame.render_widget(table, table_area);

    // -- help bar
    let help_text = match app.mode() {
        Mode::Normal => " ↑/↓ scroll  /  filter  Enter open  q quit",
        Mode::Filter => " type to filter  Esc cancel  Enter confirm",
    };
    let help = Paragraph::new(Span::styled(
        help_text,
        Style::default().fg(Color::DarkGray),
    ));
    frame.render_widget(help, help_area);
}

fn handle_event(app: &mut App) -> std::io::Result<()> {
    use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

    let event = event::read()?;

    if let Event::Key(key) = event {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            app.quit();
            return Ok(());
        }

        match app.mode() {
            Mode::Normal => match key.code {
                KeyCode::Char('q') => app.quit(),
                KeyCode::Down | KeyCode::Char('j') => app.scroll_down(),
                KeyCode::Up | KeyCode::Char('k') => app.scroll_up(),
                KeyCode::Char('/') => app.enter_filter(),
                KeyCode::Enter => {
                    if let Some(url) = app.selected_url() {
                        let _ = open::that(url);
                    }
                }
                _ => {}
            },
            Mode::Filter => match key.code {
                KeyCode::Esc => app.exit_filter(),
                KeyCode::Backspace => app.filter_pop(),
                KeyCode::Enter => app.confirm_filter(),
                KeyCode::Char(c) => app.filter_push(c),
                _ => {}
            },
        }
    }

    Ok(())
}

pub fn run(idea: &str, verdict: &Verdict, matches: &[Match]) -> anyhow::Result<()> {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        original_hook(info);
    }));

    let mut terminal = ratatui::init();
    let result = run_loop(&mut terminal, idea, verdict, matches);
    ratatui::restore();
    result
}

fn run_loop(
    terminal: &mut DefaultTerminal,
    idea: &str,
    verdict: &Verdict,
    matches: &[Match],
) -> anyhow::Result<()> {
    let mut app = App::new(idea, verdict, matches);

    loop {
        terminal.draw(|frame| draw(frame, &app))?;
        handle_event(&mut app)?;
        if app.should_quit() {
            return Ok(());
        }
    }
}
