//! ratatui interface.
//!
//! Header (idea + sources-checked transparency line), verdict panel
//! (🟢/🟡/🔴 + headline + gaps + caveat), and a scrollable/filterable matches
//! table. `↑/↓` scroll, `/` filter, `m` show more, `Enter` open URL, `?` help,
//! `q` quit.

use patent::model::{Match, Saturation, Source, Verdict};
use patent::tui::{App, Mode};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, HighlightSpacing, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table, TableState, Wrap,
    },
    DefaultTerminal, Frame,
};

const ACCENT: Color = Color::Cyan;
const MUTED: Color = Color::DarkGray;

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

/// The plain text of a styled line (span contents concatenated).
fn line_text(line: &Line) -> String {
    line.spans.iter().map(|s| s.content.as_ref()).collect()
}

/// Rows `text` occupies when word-wrapped to `width` columns.
///
/// Mirrors ratatui's `Wrap { trim: false }` (greedy word packing, hard-splitting
/// any word longer than the line) closely enough to never *under*-count for
/// normal text — over-counting is harmless (the table just gets a row fewer),
/// but under-counting would clip the integrity-critical caveat.
fn wrapped_rows(text: &str, width: u16) -> u16 {
    let width = width.max(1) as usize;
    let mut rows: u16 = 1;
    let mut col = 0usize;
    let mut first = true;
    for word in text.split(' ') {
        let wlen = word.chars().count();
        let needed = if first { wlen } else { col + 1 + wlen };
        if !first && needed > width {
            rows = rows.saturating_add(1);
            col = 0;
            first = true;
        }
        if first && wlen > width {
            // A single word longer than the line is hard-split across rows.
            rows = rows.saturating_add(((wlen - 1) / width) as u16);
            col = wlen - ((wlen - 1) / width) * width;
        } else if first {
            col = wlen;
        } else {
            col += 1 + wlen;
        }
        first = false;
    }
    rows.max(1)
}

fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let width = area.width;
    let verdict = app.verdict();

    // Build the verdict lines first so the panel can be sized to fit them. The
    // humble caveat is the last line and MUST always be visible (integrity
    // rule), so the panel is never capped — the table takes whatever remains.
    let color = level_color(verdict.level);
    let mut verdict_lines = vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled(
                format!("{} {}", level_icon(verdict.level), verdict.level),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" — ", Style::default().add_modifier(Modifier::DIM)),
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
        verdict_lines.push(Line::from(vec![
            Span::styled("  • ", Style::default().fg(Color::Yellow)),
            Span::styled(gap.as_str(), Style::default().fg(Color::White)),
        ]));
    }
    verdict_lines.push(Line::raw(""));
    verdict_lines.push(Line::from(Span::styled(
        format!(" ⚠  {}", verdict.caveat),
        Style::default()
            .add_modifier(Modifier::DIM)
            .add_modifier(Modifier::ITALIC),
    )));

    // Header height: idea + sources (+ optional "not reached") + bottom border.
    let header_content = if verdict.sources_failed.is_empty() {
        2
    } else {
        3
    };
    let header_height = header_content + 1;

    // Verdict height: sum of word-wrapped content rows, plus panel chrome.
    // Never capped — the table takes whatever's left — so the last line (the
    // humble caveat) is always allocated space.
    let verdict_rows: u16 = verdict_lines
        .iter()
        .map(|l| wrapped_rows(&line_text(l), width))
        .sum();
    // +2 for the panel chrome (the `.title(" Verdict ")` row + the bottom
    // border), +1 slack so a unicode-width rounding difference (e.g. the ⚠
    // glyph) can never clip the caveat by a row.
    let verdict_height = verdict_rows + 3;

    let [header_area, verdict_area, table_area, footer_area] = Layout::vertical([
        Constraint::Length(header_height),
        Constraint::Length(verdict_height),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);

    // -- header
    let sources: Vec<Span> = verdict
        .sources_checked
        .iter()
        .enumerate()
        .flat_map(|(i, s)| {
            let mut spans = Vec::new();
            if i > 0 {
                spans.push(Span::styled(
                    " · ",
                    Style::default().add_modifier(Modifier::DIM),
                ));
            }
            spans.push(Span::styled(
                s.to_string(),
                Style::default().fg(source_color(*s)),
            ));
            spans
        })
        .collect();

    let mut source_line = vec![Span::styled(
        " Sources: ",
        Style::default().add_modifier(Modifier::DIM),
    )];
    source_line.extend(sources);

    let mut header_lines = vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled(
                app.idea(),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(source_line),
    ];
    // Transparency: selected sources that failed are surfaced, not hidden, so a
    // thin result is never mistaken for "nothing out there."
    if !verdict.sources_failed.is_empty() {
        let mut nr = vec![Span::styled(
            " Not reached: ",
            Style::default().fg(Color::Red).add_modifier(Modifier::DIM),
        )];
        for (i, s) in verdict.sources_failed.iter().enumerate() {
            if i > 0 {
                nr.push(Span::styled(
                    " · ",
                    Style::default().add_modifier(Modifier::DIM),
                ));
            }
            nr.push(Span::styled(s.to_string(), Style::default().fg(MUTED)));
        }
        header_lines.push(Line::from(nr));
    }

    let header = Paragraph::new(header_lines).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(MUTED)),
    );
    frame.render_widget(header, header_area);

    // -- verdict panel
    let verdict_panel = Paragraph::new(verdict_lines)
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(MUTED))
                .title(Span::styled(
                    " Verdict ",
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )),
        );
    frame.render_widget(verdict_panel, verdict_area);

    // -- matches table (stateful so it scrolls to keep the selection visible)
    let displayed = app.displayed_matches();
    let total_visible = app.visible_matches().len();

    let rows: Vec<Row> = displayed
        .iter()
        .map(|m| {
            Row::new(vec![
                Cell::from(format!("{:.2}", m.similarity))
                    .style(Style::default().fg(score_color(m.similarity))),
                Cell::from(m.name.as_str()).style(
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(m.source.to_string()).style(Style::default().fg(source_color(m.source))),
                Cell::from(m.description.as_str())
                    .style(Style::default().add_modifier(Modifier::DIM)),
            ])
        })
        .collect();

    let title = if app.mode() == Mode::Filter {
        format!(
            " Matches [/{}] ({}/{}) ",
            app.filter_text(),
            total_visible,
            app.total_matches()
        )
    } else if !app.filter_text().is_empty() {
        format!(
            " Matches [{}] ({}/{}) ",
            app.filter_text(),
            total_visible,
            app.total_matches()
        )
    } else if app.has_more() {
        format!(
            " Matches ({} of {} — m for all) ",
            displayed.len(),
            total_visible
        )
    } else if app.is_expanded() {
        format!(" Matches (all {}) ", displayed.len())
    } else {
        format!(" Matches ({}) ", displayed.len())
    };

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
            .style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD))
            .bottom_margin(1),
    )
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .highlight_spacing(HighlightSpacing::Never)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(ACCENT))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
    );

    let mut table_state = TableState::default();
    if !displayed.is_empty() {
        table_state.select(Some(app.cursor().min(displayed.len() - 1)));
    }
    frame.render_stateful_widget(table, table_area, &mut table_state);

    // -- scrollbar: only when there's more than fits in the table viewport.
    // Chrome = top border + header row + header bottom_margin + bottom border.
    let viewport = table_area.height.saturating_sub(4);
    if (displayed.len() as u16) > viewport && viewport > 0 {
        let mut sb_state = ScrollbarState::new(displayed.len()).position(app.cursor());
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .thumb_style(Style::default().fg(ACCENT)),
            table_area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut sb_state,
        );
    }

    // -- footer hint bar
    let footer_spans = match app.mode() {
        Mode::Normal => {
            let mut spans = vec![
                key_span(" ↑↓"),
                label_span(" scroll  "),
                key_span("/"),
                label_span(" filter  "),
            ];
            if app.has_more() {
                spans.extend([key_span("m"), label_span(" more  ")]);
            } else if app.is_expanded() {
                spans.extend([key_span("m"), label_span(" less  ")]);
            }
            spans.extend([
                key_span("Enter"),
                label_span(" open  "),
                key_span("?"),
                label_span(" help  "),
                key_span("q"),
                label_span(" quit"),
            ]);
            spans
        }
        Mode::Filter => vec![
            label_span(" type to filter  "),
            key_span("Esc"),
            label_span(" cancel  "),
            key_span("Enter"),
            label_span(" confirm"),
        ],
        Mode::Help => vec![
            label_span(" "),
            key_span("?"),
            label_span(" or "),
            key_span("Esc"),
            label_span(" close help"),
        ],
    };
    let footer = Paragraph::new(Line::from(footer_spans));
    frame.render_widget(footer, footer_area);

    // -- help overlay (drawn last so it floats above everything)
    if app.mode() == Mode::Help {
        draw_help(frame);
    }
}

fn draw_help(frame: &mut Frame) {
    let area = centered_rect(50, 70, frame.area());
    frame.render_widget(Clear, area);

    let lines = vec![
        Line::raw(""),
        help_section("Navigation"),
        help_row("↑ / k", "Scroll up"),
        help_row("↓ / j", "Scroll down"),
        help_row("g / Home", "Jump to top"),
        help_row("G / End", "Jump to bottom"),
        Line::raw(""),
        help_section("Actions"),
        help_row("Enter", "Open in browser"),
        help_row("/", "Filter matches"),
        help_row("m", "Show more / less"),
        help_row("?", "Toggle this help"),
        help_row("q", "Quit"),
        Line::raw(""),
        help_section("Filter mode"),
        help_row("Esc", "Cancel filter"),
        help_row("Enter", "Confirm filter"),
        help_row("Backspace", "Delete character"),
        Line::raw(""),
    ];

    let help = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(ACCENT))
            .title(Span::styled(
                " Keybindings ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
    );
    frame.render_widget(help, area);
}

fn help_section(title: &str) -> Line<'_> {
    Line::from(Span::styled(
        format!("  {title}"),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    ))
}

fn help_row<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("  {key:>12}  "),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(desc, Style::default().fg(Color::White)),
    ])
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let [_, vert, _] = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .areas(area);
    let [_, horiz, _] = Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .areas(vert);
    horiz
}

fn key_span(text: &str) -> Span<'_> {
    Span::styled(
        text,
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
    )
}

fn label_span(text: &str) -> Span<'_> {
    Span::styled(text, Style::default().add_modifier(Modifier::DIM))
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
                KeyCode::Home | KeyCode::Char('g') => app.scroll_to_top(),
                KeyCode::End | KeyCode::Char('G') => app.scroll_to_bottom(),
                KeyCode::Char('/') => app.enter_filter(),
                KeyCode::Char('m') => app.toggle_expand(),
                KeyCode::Char('?') => app.toggle_help(),
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
            Mode::Help => match key.code {
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => app.toggle_help(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use patent::verdict::CAVEAT;
    use ratatui::{backend::TestBackend, Terminal};

    fn verdict_with(gaps: usize, failed: Vec<Source>) -> Verdict {
        Verdict {
            level: Saturation::Crowded,
            headline: "Several closely-related tools turned up in the sources checked.".into(),
            gaps: (0..gaps)
                .map(|i| format!("a differentiator number {i} the user could pursue"))
                .collect(),
            sources_checked: vec![
                Source::Npm,
                Source::CratesIo,
                Source::GitHub,
                Source::HackerNews,
            ],
            sources_failed: failed,
            caveat: CAVEAT.to_string(),
        }
    }

    fn many_matches(n: usize) -> Vec<Match> {
        (0..n)
            .map(|i| Match {
                name: format!("tool-{i}"),
                source: Source::Npm,
                url: format!("https://example.com/{i}"),
                description: "a tool that does something useful".into(),
                popularity: Some(100),
                similarity: 0.9 - (i as f32 * 0.01),
            })
            .collect()
    }

    fn rendered(width: u16, height: u16, verdict: &Verdict, matches: &[Match]) -> String {
        let app = App::new(
            "an interactive cli to manage processes on a port",
            verdict,
            matches,
        );
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| draw(f, &app)).unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect()
    }

    #[test]
    fn caveat_is_never_clipped_at_common_sizes() {
        // The humble caveat ends with "before committing." and must ALWAYS be
        // visible — this is the non-negotiable integrity guarantee. Guards the
        // regression where the panel title row / word-wrap was under-budgeted.
        for (w, h) in [(80u16, 24u16), (100, 30), (120, 40), (80, 28)] {
            for gaps in [0usize, 2, 4] {
                let v = verdict_with(gaps, vec![]);
                let text = rendered(w, h, &v, &many_matches(40));
                assert!(
                    text.contains("committing"),
                    "caveat clipped at {w}x{h} with {gaps} gaps"
                );
            }
        }
    }

    #[test]
    fn not_reached_sources_are_surfaced() {
        let v = verdict_with(2, vec![Source::PyPI, Source::Go]);
        let text = rendered(100, 30, &v, &many_matches(5));
        assert!(text.contains("Not reached"), "failed sources must be shown");
        assert!(
            text.contains("committing"),
            "caveat still shown with a not-reached line"
        );
    }

    #[test]
    fn renders_without_panic_at_tiny_sizes() {
        // Layout/scrollbar must not panic on degenerate terminal sizes.
        let v = verdict_with(3, vec![Source::PyPI]);
        for (w, h) in [(1u16, 1u16), (10, 3), (40, 5), (80, 2)] {
            let _ = rendered(w, h, &v, &many_matches(50));
        }
    }

    // TEMP adversarial oracle: compare wrapped_rows against ratatui's real reflow.
    fn build_verdict_lines(verdict: &Verdict) -> Vec<Line<'_>> {
        let color = level_color(verdict.level);
        let mut verdict_lines = vec![
            Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    format!("{} {}", level_icon(verdict.level), verdict.level),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" — ", Style::default().add_modifier(Modifier::DIM)),
                Span::styled(&verdict.headline, Style::default().fg(Color::White)),
            ]),
            Line::raw(""),
        ];
        for gap in &verdict.gaps {
            verdict_lines.push(Line::from(vec![
                Span::styled("  • ", Style::default().fg(Color::Yellow)),
                Span::styled(gap.as_str(), Style::default().fg(Color::White)),
            ]));
        }
        verdict_lines.push(Line::raw(""));
        verdict_lines.push(Line::from(Span::styled(
            format!(" ⚠  {}", verdict.caveat),
            Style::default(),
        )));
        verdict_lines
    }

    // Render a single line at `width` into a tall buffer and count how many rows
    // ratatui's real Wrap{trim:false} reflow actually used (rows containing any
    // non-space cell). The empty Line::raw("") legitimately uses 1 row.
    fn real_rows(line: &Line, width: u16) -> usize {
        if width == 0 {
            return 1;
        }
        let height = 40u16;
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|f| {
                let p = Paragraph::new(line.clone()).wrap(Wrap { trim: false });
                f.render_widget(p, f.area());
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        let mut used = 0usize;
        for y in 0..height {
            let mut nonblank = false;
            for x in 0..width {
                if buf[(x, y)].symbol() != " " {
                    nonblank = true;
                    break;
                }
            }
            if nonblank {
                used = (y as usize) + 1;
            }
        }
        used.max(1)
    }

    #[test]
    fn temp_wrapped_rows_never_undercounts_real_reflow() {
        let adversarial: Vec<Verdict> = vec![
            verdict_with(0, vec![]),
            verdict_with(5, vec![]),
            Verdict {
                level: Saturation::Saturated,
                headline: "🔴 a — multi—dash headline with ⚠ glyphs • and • bullets repeated \
                    several times to span multiple wrapped rows at narrow widths indeed"
                    .into(),
                gaps: vec![
                    "no — async — support — with — many — em — dashes — that — are — wide".into(),
                    "supercalifragilisticexpialidocious-pneumonoultramicroscopicsilicovolcanoconiosis-longword".into(),
                    "• • • • • • • • • • • • • • • • • • • • • • • • • • • • • •".into(),
                    "double  spaces   and    runs     of      whitespace       inside".into(),
                    "嗨 你好 世界 这是 一个 测试 的 间隙 文本 用于 检查 换行 计数 是否 正确".into(),
                ],
                sources_checked: vec![Source::Npm, Source::GitHub],
                sources_failed: vec![],
                caveat: CAVEAT.to_string(),
            },
        ];
        let mut undercounts_lt80 = 0u32;
        let mut undercounts_ge80 = 0u32;
        for v in &adversarial {
            let lines = build_verdict_lines(v);
            for w in 1u16..=200 {
                for line in &lines {
                    let mine = wrapped_rows(&line_text(line), w) as usize;
                    let real = real_rows(line, w);
                    if mine < real {
                        if w >= 80 {
                            undercounts_ge80 += 1;
                            eprintln!(
                                "UNDERCOUNT>=80 at width {w}: mine={mine} real={real} {:?}",
                                line_text(line)
                            );
                        } else {
                            undercounts_lt80 += 1;
                        }
                    }
                }
            }
        }
        eprintln!("undercounts <80: {undercounts_lt80}, >=80: {undercounts_ge80}");
        assert_eq!(undercounts_ge80, 0, "under-count at realistic width >=80");
    }

    #[test]
    fn temp_total_undercount_vs_slack() {
        // The clip happens only if SUM(real_rows) - SUM(wrapped_rows) >= 2,
        // because verdict_height = SUM(wrapped_rows) + 3 and chrome eats 2.
        // Try realistic verdicts (headline + <=5 short gaps + caveat) and find
        // the worst total deficit across normal widths.
        let realistic: Vec<Verdict> = vec![
            verdict_with(0, vec![]),
            verdict_with(1, vec![]),
            verdict_with(3, vec![]),
            verdict_with(5, vec![]),
            verdict_with(5, vec![Source::PyPI]),
            // gaps engineered with many wide em-dash glyphs to maximize the
            // display-width vs char-count deficit per line.
            Verdict {
                level: Saturation::Saturated,
                headline:
                    "🔴 — — — — — — — — — — — — — — — — — — — — — — — — — — — — — — — — — — — —"
                        .into(),
                gaps: (0..5)
                    .map(|_| "— — — — — — — — — — — — — — — — — — — — — — — — — — — — — —".into())
                    .collect(),
                sources_checked: vec![Source::Npm],
                sources_failed: vec![],
                caveat: CAVEAT.to_string(),
            },
        ];
        // Programmatically generate many em-dash-heavy shapes so several lines
        // can hit their wrap boundary at the SAME width simultaneously.
        let dashes = |n: usize| -> String {
            std::iter::repeat("— ")
                .take(n)
                .collect::<String>()
                .trim_end()
                .to_string()
        };
        let mut generated: Vec<Verdict> = Vec::new();
        for hl in 30..120 {
            for gl in 30..120 {
                if (hl as i32 - gl as i32).abs() > 6 {
                    continue;
                }
                generated.push(Verdict {
                    level: Saturation::Saturated,
                    headline: format!("🔴 {}", dashes(hl)),
                    gaps: (0..5).map(|_| dashes(gl)).collect(),
                    sources_checked: vec![Source::Npm],
                    sources_failed: vec![],
                    caveat: CAVEAT.to_string(),
                });
            }
        }
        let mut worst = 0i32;
        for v in realistic.iter().chain(generated.iter()) {
            let lines = build_verdict_lines(v);
            for w in 80u16..=160 {
                let mine: usize = lines
                    .iter()
                    .map(|l| wrapped_rows(&line_text(l), w) as usize)
                    .sum();
                let real: usize = lines.iter().map(|l| real_rows(l, w)).sum();
                let deficit = real as i32 - mine as i32;
                if deficit > worst {
                    worst = deficit;
                    eprintln!("new worst total deficit {deficit} at width {w}");
                }
            }
        }
        eprintln!("WORST total deficit at width>=80: {worst} (slack is 1)");
        assert!(
            worst < 2,
            "total deficit {worst} >= 2 would clip the caveat"
        );
    }

    #[test]
    fn temp_caveat_visible_adversarial() {
        let v = Verdict {
            level: Saturation::Saturated,
            headline: "🔴 lots of — closely related ⚠ tooling turned up in the sources \
                checked across many ecosystems and registries we queried just now"
                .into(),
            gaps: (0..5)
                .map(|i| {
                    format!(
                        "differentiator {i} — a — wide — em — dash — laden — gap — entry — text"
                    )
                })
                .collect(),
            sources_checked: vec![Source::Npm, Source::CratesIo, Source::GitHub],
            sources_failed: vec![Source::PyPI],
            caveat: CAVEAT.to_string(),
        };
        for (w, h) in [(80u16, 24u16), (100, 30), (120, 40), (80, 28), (90, 25)] {
            let text = rendered(w, h, &v, &many_matches(40));
            assert!(text.contains("committing"), "caveat clipped at {w}x{h}");
        }
    }
}
