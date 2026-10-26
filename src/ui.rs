use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph, Wrap};

use crate::app::{App, Prompt};
use crate::synth::Kind;

const ACCENT: Color = Color::Rgb(137, 180, 250);
const DIM: Color = Color::Rgb(108, 112, 134);

pub fn draw(f: &mut Frame, app: &App) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(3),
        Constraint::Length(3),
    ])
    .areas(f.area());

    draw_header(f, app, header);
    draw_mixer(f, app, body);
    draw_footer(f, app, footer);
}

fn bar(level: f32, width: usize, active: bool) -> Vec<Span<'static>> {
    let filled = (level * width as f32).round() as usize;
    let on = if active {
        ACCENT
    } else {
        Color::Rgb(116, 140, 190)
    };
    vec![
        Span::styled("█".repeat(filled), Style::new().fg(on)),
        Span::styled(
            "░".repeat(width - filled),
            Style::new().fg(Color::Rgb(69, 71, 90)),
        ),
    ]
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let status = if app.shared().paused() {
        "paused"
    } else {
        "playing"
    };
    let mut spans = vec![
        Span::styled(" amb ", Style::new().bg(ACCENT).fg(Color::Black).bold()),
        Span::raw("  master "),
    ];
    spans.extend(bar(app.shared().master(), 16, false));
    spans.push(Span::raw(format!(
        " {:>3}%  ",
        (app.shared().master() * 100.0).round()
    )));
    spans.push(Span::styled(status, Style::new().fg(DIM).italic()));
    if let Some(name) = app.active_preset() {
        spans.push(Span::styled("  ·  ", Style::new().fg(DIM)));
        spans.push(Span::styled(name.to_string(), Style::new().fg(ACCENT)));
    }
    f.render_widget(
        Paragraph::new(Line::from(spans)).block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(DIM)),
        ),
        area,
    );
}

fn draw_mixer(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(DIM))
        .title(" mixer ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    // marker + space + name(9) + space + percentage(5) take 17 columns; the
    // bar takes the rest so the mixer spans the window.
    let bar_width = (inner.width as usize).saturating_sub(17).max(4);
    // On short terminals keep the selected row in view by scrolling.
    let rows = (inner.height as usize).max(1);
    let offset = (app.selected() + 1).saturating_sub(rows);
    let lines: Vec<Line> = Kind::ALL
        .iter()
        .enumerate()
        .skip(offset)
        .take(rows)
        .map(|(i, kind)| {
            let level = app.shared().level(i);
            let selected = i == app.selected();
            let marker = if selected { "▌" } else { " " };
            let name_style = if selected {
                Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)
            } else if level > 0.0 {
                Style::new()
            } else {
                Style::new().fg(DIM)
            };
            let mut spans = vec![
                Span::styled(marker, Style::new().fg(ACCENT)),
                Span::raw(" "),
                Span::styled(format!("{:<9}", kind.name()), name_style),
            ];
            spans.push(Span::raw(" "));
            spans.extend(bar(level, bar_width, selected));
            spans.push(Span::styled(
                format!(" {:>3}%", (level * 100.0).round()),
                name_style,
            ));
            Line::from(spans)
        })
        .collect();
    f.render_widget(Paragraph::new(lines), inner);
}

/// Footer text, centred and wrapped so an 80-column terminal never clips it.
fn centered(spans: Vec<Span<'_>>) -> Paragraph<'_> {
    Paragraph::new(Line::from(spans))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    if let Some(prompt) = app.prompt() {
        let dim = Style::new().fg(DIM);
        let accent = Style::new().fg(ACCENT);
        let spans = match prompt {
            Prompt::Save(name) => vec![
                Span::styled("save preset as ", dim),
                Span::styled(format!("{name}█"), accent),
                Span::styled("  ·  enter save  ·  esc cancel", dim),
            ],
            Prompt::Rename { from, name } => vec![
                Span::styled(format!("rename {from} to "), dim),
                Span::styled(format!("{name}█"), accent),
                Span::styled("  ·  enter rename  ·  esc cancel", dim),
            ],
            Prompt::ConfirmReset => vec![
                Span::styled(
                    "reset every preset to the built-ins, deleting your own? ",
                    dim,
                ),
                Span::styled("y", accent),
                Span::styled(" to confirm  ·  any other key cancels", dim),
            ],
            Prompt::ConfirmDelete(name) => vec![
                Span::styled(format!("delete preset {name}? "), dim),
                Span::styled("y", accent),
                Span::styled(" to confirm  ·  any other key cancels", dim),
            ],
        };
        f.render_widget(centered(spans), area);
        return;
    }
    if let Some(notice) = app.notice() {
        let span = Span::styled(notice.to_string(), Style::new().fg(ACCENT));
        f.render_widget(centered(vec![span]), area);
        return;
    }
    let keys = [
        ("↑↓/jk", "select"),
        ("←→/hl", "level"),
        ("0-9", "set"),
        ("p/P", "preset"),
        ("s/r/d", "save/rename/delete preset"),
        ("R", "reset presets"),
        ("+/-", "master"),
        ("space", "pause"),
        ("m", "mute all"),
        ("q", "quit"),
    ];
    let mut spans = Vec::new();
    for (i, (k, d)) in keys.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ·  ", Style::new().fg(DIM)));
        }
        spans.push(Span::styled(*k, Style::new().fg(ACCENT)));
        spans.push(Span::styled(format!(" {d}"), Style::new().fg(DIM)));
    }
    // The hints are ~150 columns wide; they wrap over the footer's three rows.
    f.render_widget(centered(spans), area);
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ratatui::{Terminal, backend::TestBackend};

    use super::*;
    use crate::audio::Shared;

    #[test]
    fn renders_every_sound() {
        let shared = Arc::new(Shared::new());
        shared.set_level(0, 0.5);
        let app = App::new(shared);
        let mut term = Terminal::new(TestBackend::new(80, 16)).unwrap();
        term.draw(|f| draw(f, &app)).unwrap();
        let text: String = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        for k in Kind::ALL {
            assert!(text.contains(k.name()), "missing {}", k.name());
        }
    }

    #[test]
    fn short_terminal_keeps_selection_visible() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut app = App::new(Arc::new(Shared::new()));
        // Up wraps to the last sound.
        app.on_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        let mut term = Terminal::new(TestBackend::new(40, 9)).unwrap();
        term.draw(|f| draw(f, &app)).unwrap();
        let text: String = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(
            text.contains("Crickets"),
            "selected row scrolled out of view"
        );
    }

    #[test]
    fn confirm_prompts_are_not_clipped_on_an_80_column_terminal() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut app = App::new(Arc::new(Shared::new()));
        app.on_key(KeyEvent::new(KeyCode::Char('R'), KeyModifiers::NONE));
        let mut term = Terminal::new(TestBackend::new(80, 16)).unwrap();
        term.draw(|f| draw(f, &app)).unwrap();
        let text: String = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("cancels"), "reset prompt is clipped");
    }

    #[test]
    fn footer_hints_fit_an_80_column_terminal() {
        let app = App::new(Arc::new(Shared::new()));
        let mut term = Terminal::new(TestBackend::new(80, 16)).unwrap();
        term.draw(|f| draw(f, &app)).unwrap();
        let text: String = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        for hint in [
            "select",
            "level",
            "set",
            "preset",
            "save/rename/delete",
            "reset presets",
            "master",
            "pause",
            "mute all",
            "quit",
        ] {
            assert!(text.contains(hint), "footer hint {hint:?} is clipped");
        }
    }
}
