//! ratatui interface.
//!
//! Header (idea + sources-checked transparency line), verdict panel
//! (🟢/🟡/🔴 + headline + gaps + caveat), and a scrollable/filterable matches
//! table. `↑/↓` scroll, `/` filter, `m` show more, `Enter` open URL, `q` quit.

use patent::model::{Match, Saturation, Source, Verdict};
use patent::tui::{App, Mode};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, Wrap},
    DefaultTerminal, Frame,
};

const CYAN: Color = Color::Cyan;
const DIM: Color = Color::Rgb(100, 100, 100);
const STRIPE: Color = Color::Rgb(30, 30, 40);
const SELECT_BG: Color = Color::Rgb(50, 50, 80);
const SELECT_FG: Color = Color::White;
const BORDER: Color = Color::Rgb(80, 80, 100);

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

fn score_color(sim: f32) -> Color {
    if sim >= 0.7 {
        Color::Green
    } else if sim >= 0.4 {
        Color::Yellow
    } else {
        Color::Red
    }
}

fn source_color(source: Source) -> Color {
    match source {
        Source::CratesIo => Color::Yellow,
        Source::GitHub => Color::White,
        Source::Npm => Color::Red,
        Source::PyPI => Color::Blue,
        Source::HackerNews => Color::Rgb(255, 102, 0),
        Source::Go => Color::Cyan,
        Source::Maven => Color::Rgb(200, 50, 50),
        Source::RubyGems => Color::Magenta,
        Source::DockerHub => Color::Rgb(30, 144, 255),
        Source::VsCodeMarketplace => Color::Rgb(0, 122, 204),
        Source::NuGet => Color::Rgb(100, 45, 170),
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

    let border_style = Style::default().fg(BORDER);

    // -- header
    let sources: Vec<Span> = verdict
        .sources_checked
        .iter()
        .enumerate()
        .flat_map(|(i, s)| {
            let mut spans = Vec::new();
            if i > 0 {
                spans.push(Span::styled(" · ", Style::default().fg(DIM)));
            }
            spans.push(Span::styled(
                s.to_string(),
                Style::default().fg(source_color(*s)),
            ));
            spans
        })
        .collect();

    let mut source_line = vec![Span::styled(" Sources: ", Style::default().fg(DIM))];
    source_line.extend(sources);

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" 🔍 ", Style::default()),
            Span::styled(
                app.idea(),
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(source_line),
    ])
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(border_style),
    );
    frame.render_widget(header, header_area);

    // -- verdict panel
    let color = level_color(verdict.level);
    let mut lines = vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled(
                format!("{} {}", level_icon(verdict.level), verdict.level),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" — ", Style::default().fg(DIM)),
            Span::styled(
                &verdict.headline,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::raw(""),
    ];
    for gap in &verdict.gaps {
        lines.push(Line::from(vec![
            Span::styled("  • ", Style::default().fg(Color::Yellow)),
            Span::styled(gap.as_str(), Style::default().fg(Color::White)),
        ]));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        format!(" ⚠  {}", verdict.caveat),
        Style::default().fg(DIM).add_modifier(Modifier::ITALIC),
    )));
    let verdict_panel = Paragraph::new(lines).wrap(Wrap { trim: false }).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(border_style)
            .title(Span::styled(
                " Verdict ",
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )),
    );
    frame.render_widget(verdict_panel, verdict_area);

    // -- matches table
    let displayed = app.displayed_matches();
    let total_visible = app.visible_matches().len();

    let rows: Vec<Row> = displayed
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let is_selected = i == app.cursor();
            let base = if is_selected {
                Style::default().bg(SELECT_BG).fg(SELECT_FG)
            } else if i % 2 == 1 {
                Style::default().bg(STRIPE)
            } else {
                Style::default()
            };

            let score_style = if is_selected {
                base.fg(score_color(m.similarity))
                    .add_modifier(Modifier::BOLD)
            } else {
                base.fg(score_color(m.similarity))
            };

            let src_style = if is_selected {
                base.add_modifier(Modifier::BOLD)
            } else {
                base.fg(source_color(m.source))
            };

            let name_style = if is_selected {
                base.add_modifier(Modifier::BOLD)
            } else {
                base.fg(Color::White)
            };

            let desc_style = if is_selected {
                base
            } else {
                base.fg(Color::Gray)
            };

            Row::new(vec![
                ratatui::widgets::Cell::from(format!("{:.2}", m.similarity)).style(score_style),
                ratatui::widgets::Cell::from(m.name.as_str()).style(name_style),
                ratatui::widgets::Cell::from(m.source.to_string()).style(src_style),
                ratatui::widgets::Cell::from(m.description.as_str()).style(desc_style),
            ])
            .style(base)
        })
        .collect();

    let title = if app.mode() == Mode::Filter {
        format!(" Matches [/{}] ", app.filter_text())
    } else if !app.filter_text().is_empty() {
        format!(" Matches [{}] ", app.filter_text())
    } else if app.has_more() {
        format!(
            " Matches ({}/{} — press m for more) ",
            displayed.len(),
            total_visible
        )
    } else if app.is_expanded() {
        format!(" Matches (all {}) ", displayed.len())
    } else {
        format!(" Matches ({}) ", displayed.len())
    };

    let header_style = Style::default().fg(CYAN).add_modifier(Modifier::BOLD);

    let table = Table::new(
        rows,
        [
            Constraint::Length(6),
            Constraint::Length(24),
            Constraint::Length(14),
            Constraint::Min(20),
        ],
    )
    .header(
        Row::new(vec!["Score", "Name", "Source", "Description"])
            .style(header_style)
            .bottom_margin(1),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(
                title,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    frame.render_widget(table, table_area);

    // -- help bar
    let help_spans = match app.mode() {
        Mode::Normal => {
            let mut spans = vec![
                key_span("↑↓"),
                label_span(" scroll  "),
                key_span("/"),
                label_span(" filter  "),
            ];
            if app.has_more() {
                spans.extend([key_span("m"), label_span(" show more  ")]);
            } else if app.is_expanded() {
                spans.extend([key_span("m"), label_span(" show less  ")]);
            }
            spans.extend([
                key_span("Enter"),
                label_span(" open  "),
                key_span("q"),
                label_span(" quit"),
            ]);
            spans
        }
        Mode::Filter => vec![
            label_span("type to filter  "),
            key_span("Esc"),
            label_span(" cancel  "),
            key_span("Enter"),
            label_span(" confirm"),
        ],
    };
    let help = Paragraph::new(Line::from(help_spans));
    frame.render_widget(help, help_area);
}

fn key_span(text: &str) -> Span<'_> {
    Span::styled(text, Style::default().fg(CYAN).add_modifier(Modifier::BOLD))
}

fn label_span(text: &str) -> Span<'_> {
    Span::styled(text, Style::default().fg(DIM))
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
                KeyCode::Char('m') => app.toggle_expand(),
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
