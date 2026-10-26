use std::io;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::audio::Shared;
use crate::presets::{self, Preset};
use crate::synth::Kind;

const STEP: f32 = 0.05;

pub struct App {
    shared: Arc<Shared>,
    selected: usize,
    quit: bool,
    presets: Vec<Preset>,
    /// The preset applied last, which `p` / `P` step on from.
    applied: Option<usize>,
    /// The name being typed while saving a preset.
    naming: Option<String>,
    /// Persists a saved preset; `None` keeps it in memory only.
    saver: Option<Saver>,
    /// One-off message for the footer, cleared by the next key.
    notice: Option<String>,
}

type Saver = Box<dyn Fn(&Preset) -> io::Result<()>>;

const NAME_MAX: usize = 24;

impl App {
    pub fn new(shared: Arc<Shared>) -> Self {
        Self {
            shared,
            selected: 0,
            quit: false,
            presets: presets::builtin(),
            applied: None,
            naming: None,
            saver: None,
            notice: None,
        }
    }

    pub fn with_presets(mut self, presets: Vec<Preset>) -> Self {
        self.presets = presets;
        self
    }

    pub fn with_saver(mut self, saver: impl Fn(&Preset) -> io::Result<()> + 'static) -> Self {
        self.saver = Some(Box::new(saver));
        self
    }

    /// The preset name being typed, if the save prompt is open.
    pub fn naming(&self) -> Option<&str> {
        self.naming.as_deref()
    }

    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    /// Save the current levels under `name`, replacing a preset of that name.
    fn save_preset(&mut self, name: &str) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        let preset = Preset {
            name: name.to_string(),
            levels: std::array::from_fn(|i| self.shared.level(i)),
        };
        self.notice = match self.saver.as_ref().map(|save| save(&preset)) {
            Some(Err(e)) => Some(format!("couldn't write presets.toml: {e}")),
            _ => Some(format!("saved preset {name}")),
        };
        let at = match self
            .presets
            .iter()
            .position(|p| p.name.eq_ignore_ascii_case(name))
        {
            Some(i) => {
                self.presets[i] = preset;
                i
            }
            None => {
                self.presets.push(preset);
                self.presets.len() - 1
            }
        };
        self.applied = Some(at);
    }

    fn on_naming_key(&mut self, key: KeyEvent, mut name: String) {
        match key.code {
            KeyCode::Esc => return,
            KeyCode::Enter => return self.save_preset(&name),
            KeyCode::Backspace => {
                name.pop();
            }
            // Brackets would corrupt the section header in the file.
            KeyCode::Char(c)
                if !c.is_control() && c != '[' && c != ']' && name.chars().count() < NAME_MAX =>
            {
                name.push(c)
            }
            _ => {}
        }
        self.naming = Some(name);
    }

    /// Name of the last applied preset, unless the levels have been changed since.
    pub fn active_preset(&self) -> Option<&str> {
        let preset = self.presets.get(self.applied?)?;
        let unchanged = preset
            .levels
            .iter()
            .enumerate()
            .all(|(i, l)| (self.shared.level(i) - l).abs() < 0.01);
        unchanged.then_some(preset.name.as_str())
    }

    fn cycle_preset(&mut self, forward: bool) {
        let n = self.presets.len();
        if n == 0 {
            return;
        }
        let next = match (self.applied, forward) {
            (None, true) => 0,
            (None, false) => n - 1,
            (Some(i), true) => (i + 1) % n,
            (Some(i), false) => (i + n - 1) % n,
        };
        for (i, l) in self.presets[next].levels.iter().enumerate() {
            self.shared.set_level(i, *l);
        }
        self.applied = Some(next);
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
        self.notice = None;
        if let Some(name) = self.naming.take() {
            return self.on_naming_key(key, name);
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
            KeyCode::Char('p') => self.cycle_preset(true),
            KeyCode::Char('P') => self.cycle_preset(false),
            KeyCode::Char('s') => self.naming = Some(String::new()),
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
    fn presets_cycle_apply_and_go_stale_on_edit() {
        let mut app = App::new(Arc::new(Shared::new()));
        let first = app.presets[0].clone();
        app.on_key(key(KeyCode::Char('p')));
        assert_eq!(app.active_preset(), Some(first.name.as_str()));
        assert!((app.shared.level(0) - first.levels[0]).abs() < 1e-6);
        // Editing a level means the mix is no longer that preset.
        app.on_key(key(KeyCode::Char('9')));
        assert_eq!(app.active_preset(), None);
        // `P` steps back from the last applied preset, wrapping to the end.
        app.on_key(key(KeyCode::Char('P')));
        assert_eq!(
            app.active_preset(),
            Some(app.presets.last().unwrap().name.as_str())
        );
    }

    #[test]
    fn saving_names_the_mix_and_overwrites_duplicates() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let saved = Rc::new(RefCell::new(Vec::new()));
        let sink = saved.clone();
        let mut app = App::new(Arc::new(Shared::new())).with_saver(move |p| {
            sink.borrow_mut().push((p.name.clone(), p.levels[0]));
            Ok(())
        });
        let before = app.presets.len();
        let type_name = |app: &mut App, name: &str| {
            app.on_key(key(KeyCode::Char('s')));
            for c in name.chars() {
                app.on_key(key(KeyCode::Char(c)));
            }
        };

        app.on_key(key(KeyCode::Char('5')));
        // Typing in the prompt must not trigger shortcuts, and brackets are dropped.
        type_name(&mut app, "qm Mine[]");
        assert_eq!(app.naming(), Some("qm Mine"));
        assert!(!app.should_quit());
        app.on_key(key(KeyCode::Backspace));
        app.on_key(key(KeyCode::Backspace));
        app.on_key(key(KeyCode::Backspace));
        app.on_key(key(KeyCode::Backspace));
        app.on_key(key(KeyCode::Backspace));
        app.on_key(key(KeyCode::Backspace));
        app.on_key(key(KeyCode::Backspace));
        assert_eq!(app.naming(), Some(""));
        for c in "Mine".chars() {
            app.on_key(key(KeyCode::Char(c)));
        }
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.naming(), None);
        assert_eq!(app.presets.len(), before + 1);
        assert_eq!(app.active_preset(), Some("Mine"));

        // The same name in another case replaces it instead of adding a second.
        app.on_key(key(KeyCode::Char('7')));
        type_name(&mut app, "mine");
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.presets.len(), before + 1);
        assert_eq!(app.presets.last().unwrap().name, "mine");
        assert_eq!(saved.borrow().len(), 2);
        assert!((saved.borrow()[1].1 - 0.7).abs() < 1e-6);

        // Esc cancels without saving.
        type_name(&mut app, "nope");
        app.on_key(key(KeyCode::Esc));
        assert_eq!(app.naming(), None);
        assert_eq!(saved.borrow().len(), 2);
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
