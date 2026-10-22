use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::audio::Shared;
use crate::synth::Kind;

const STEP: f32 = 0.05;

pub struct App {
    shared: Arc<Shared>,
    selected: usize,
    quit: bool,
}

impl App {
    pub fn new(shared: Arc<Shared>) -> Self {
        Self {
            shared,
            selected: 0,
            quit: false,
        }
    }

    pub fn shared(&self) -> &Shared {
        &self.shared
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        let n = Kind::ALL.len();
        let sel = self.selected;
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.quit = true,
            KeyCode::Up | KeyCode::Char('k') => self.selected = (sel + n - 1) % n,
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => self.selected = (sel + 1) % n,
            KeyCode::Left | KeyCode::Char('h') => self.nudge(sel, -STEP),
            KeyCode::Right | KeyCode::Char('l') => self.nudge(sel, STEP),
            KeyCode::Char(c @ '0'..='9') => {
                let v = c.to_digit(10).unwrap() as f32 / 10.0;
                self.shared.set_level(sel, v);
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.shared.set_master(stepped(self.shared.master(), STEP))
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                self.shared.set_master(stepped(self.shared.master(), -STEP))
            }
            KeyCode::Char(' ') => self.shared.set_paused(!self.shared.paused()),
            KeyCode::Char('m') => (0..n).for_each(|i| self.shared.set_level(i, 0.0)),
            _ => {}
        }
    }

    fn nudge(&self, i: usize, delta: f32) {
        self.shared
            .set_level(i, stepped(self.shared.level(i), delta));
    }
}

/// Move `v` by `delta`, snapping to the step grid so repeated presses don't
/// accumulate float drift (and an off-grid saved value returns to the grid).
fn stepped(v: f32, delta: f32) -> f32 {
    ((v + delta) / STEP).round() * STEP
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn master_snaps_to_grid_and_reaches_zero() {
        let mut app = App::new(Arc::new(Shared::new()));
        // An off-grid saved value lands back on the 5% grid after one press.
        app.shared.set_master(0.07);
        app.on_key(key(KeyCode::Char('+')));
        assert!((app.shared.master() - 0.10).abs() < 1e-6);
        // And stepping down reaches exactly 0, not a float-error remainder.
        app.shared.set_master(0.05);
        app.on_key(key(KeyCode::Char('-')));
        assert_eq!(app.shared.master(), 0.0);
    }

    #[test]
    fn selection_wraps() {
        let mut app = App::new(Arc::new(Shared::new()));
        app.on_key(key(KeyCode::Up));
        assert_eq!(app.selected(), Kind::ALL.len() - 1);
        app.on_key(key(KeyCode::Down));
        assert_eq!(app.selected(), 0);
    }
}
