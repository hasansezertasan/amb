use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::audio::Shared;
use crate::library::Library;
use crate::presets::{Preset, Store};
use crate::synth::Kind;

const STEP: f32 = 0.05;
const NAME_MAX: usize = 24;

/// What the footer is asking the user while a preset action is under way.
pub enum Prompt {
    Save(String),
    Rename { from: String, name: String },
    ConfirmDelete(String),
    ConfirmReset,
}

pub struct App {
    shared: Arc<Shared>,
    selected: usize,
    quit: bool,
    library: Library,
    prompt: Option<Prompt>,
    /// One-off message for the footer, cleared by the next key.
    notice: Option<String>,
}

enum Edit {
    Typing,
    Submit,
    Cancel,
}

/// Apply a key to a name being typed in a prompt.
fn edit_name(key: KeyEvent, name: &mut String) -> Edit {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Esc => Edit::Cancel,
        KeyCode::Char('c') if ctrl => Edit::Cancel,
        KeyCode::Enter => Edit::Submit,
        KeyCode::Backspace => {
            name.pop();
            Edit::Typing
        }
        // Brackets would corrupt the section header in the file.
        KeyCode::Char(c)
            if !c.is_control()
                && !ctrl
                && c != '['
                && c != ']'
                && name.chars().count() < NAME_MAX =>
        {
            name.push(c);
            Edit::Typing
        }
        _ => Edit::Typing,
    }
}

impl App {
    pub fn new(shared: Arc<Shared>) -> Self {
        Self {
            shared,
            selected: 0,
            quit: false,
            library: Library::new(),
            prompt: None,
            notice: None,
        }
    }

    pub fn with_presets(mut self, presets: Vec<Preset>) -> Self {
        self.library = self.library.with_presets(presets);
        self
    }

    pub fn with_store(mut self, store: impl Store + 'static) -> Self {
        self.library = self.library.with_store(store);
        self
    }

    pub fn prompt(&self) -> Option<&Prompt> {
        self.prompt.as_ref()
    }

    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    /// Name of the last applied preset, unless the levels have been changed since.
    pub fn active_preset(&self) -> Option<&str> {
        self.library.active(&self.shared)
    }

    fn on_prompt_key(&mut self, key: KeyEvent, prompt: Prompt) {
        match prompt {
            Prompt::Save(mut name) => match edit_name(key, &mut name) {
                Edit::Typing => self.prompt = Some(Prompt::Save(name)),
                Edit::Submit => self.notice = self.library.save(&name, &self.shared),
                Edit::Cancel => {}
            },
            Prompt::Rename { from, mut name } => match edit_name(key, &mut name) {
                Edit::Typing => self.prompt = Some(Prompt::Rename { from, name }),
                Edit::Submit => self.notice = self.library.rename(&from, &name),
                Edit::Cancel => {}
            },
            Prompt::ConfirmDelete(name) => {
                if key.code == KeyCode::Char('y') {
                    self.notice = self.library.delete(&name);
                }
            }
            Prompt::ConfirmReset => {
                if key.code == KeyCode::Char('y') {
                    self.notice = Some(self.library.reset());
                }
            }
        }
    }

    /// Open a prompt that acts on the active preset, or say why it can't.
    fn open_for_active(&mut self, make: impl FnOnce(String) -> Prompt) {
        match self.active_preset().map(str::to_string) {
            Some(name) => self.prompt = Some(make(name)),
            None => self.notice = Some("no active preset: pick one with p first".to_string()),
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
        self.notice = None;
        if let Some(prompt) = self.prompt.take() {
            return self.on_prompt_key(key, prompt);
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
            KeyCode::Char('p') => self.library.cycle(true, &self.shared),
            KeyCode::Char('P') => self.library.cycle(false, &self.shared),
            KeyCode::Char('s') => self.prompt = Some(Prompt::Save(String::new())),
            KeyCode::Char('r') => self.open_for_active(|name| Prompt::Rename {
                from: name.clone(),
                name,
            }),
            KeyCode::Char('d') => self.open_for_active(Prompt::ConfirmDelete),
            KeyCode::Char('R') => self.prompt = Some(Prompt::ConfirmReset),
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
    use std::cell::RefCell;
    use std::io;
    use std::rc::Rc;

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
        let first = app.library.presets()[0].clone();
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
            Some(app.library.presets().last().unwrap().name.as_str())
        );
    }

    /// Records every store call so tests can see what would hit the file.
    #[derive(Clone, Default)]
    struct Log(Rc<RefCell<Vec<String>>>);

    impl Store for Log {
        fn save(&self, p: &Preset) -> io::Result<()> {
            self.0.borrow_mut().push(format!("save {}", p.name));
            Ok(())
        }
        fn remove(&self, name: &str) -> io::Result<()> {
            self.0.borrow_mut().push(format!("remove {name}"));
            Ok(())
        }
        fn rename(&self, from: &str, to: &str) -> io::Result<()> {
            self.0.borrow_mut().push(format!("rename {from} {to}"));
            Ok(())
        }
        fn reset(&self) -> io::Result<()> {
            self.0.borrow_mut().push("reset".to_string());
            Ok(())
        }
    }

    fn app_with_log() -> (App, Log) {
        let log = Log::default();
        let app = App::new(Arc::new(Shared::new())).with_store(log.clone());
        (app, log)
    }

    fn type_str(app: &mut App, text: &str) {
        for c in text.chars() {
            app.on_key(key(KeyCode::Char(c)));
        }
    }

    /// The text typed so far in a save or rename prompt.
    fn typed(app: &App) -> Option<&str> {
        match app.prompt()? {
            Prompt::Save(t) => Some(t),
            Prompt::Rename { name, .. } => Some(name),
            Prompt::ConfirmDelete(_) | Prompt::ConfirmReset => None,
        }
    }

    #[test]
    fn saving_names_the_mix_and_overwrites_duplicates() {
        let (mut app, log) = app_with_log();
        let before = app.library.presets().len();

        app.on_key(key(KeyCode::Char('5')));
        // Typing in the prompt must not trigger shortcuts, and brackets are dropped.
        app.on_key(key(KeyCode::Char('s')));
        type_str(&mut app, "qm Mine[]");
        assert_eq!(typed(&app), Some("qm Mine"));
        assert!(!app.should_quit());
        for _ in 0..4 {
            app.on_key(key(KeyCode::Backspace));
        }
        assert_eq!(typed(&app), Some("qm "));
        for _ in 0..3 {
            app.on_key(key(KeyCode::Backspace));
        }
        type_str(&mut app, "Mine");
        app.on_key(key(KeyCode::Enter));
        assert!(app.prompt().is_none());
        assert_eq!(app.library.presets().len(), before + 1);
        assert_eq!(app.active_preset(), Some("Mine"));

        // The same name in another case replaces it instead of adding a second.
        app.on_key(key(KeyCode::Char('7')));
        app.on_key(key(KeyCode::Char('s')));
        type_str(&mut app, "mine");
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.library.presets().len(), before + 1);
        assert_eq!(app.library.presets().last().unwrap().name, "mine");
        assert!((app.library.presets().last().unwrap().levels[0] - 0.7).abs() < 1e-6);
        assert_eq!(*log.0.borrow(), ["save Mine", "save mine"]);

        // Esc cancels without saving.
        app.on_key(key(KeyCode::Char('s')));
        type_str(&mut app, "nope");
        app.on_key(key(KeyCode::Esc));
        assert!(app.prompt().is_none());
        assert_eq!(log.0.borrow().len(), 2);

        // Ctrl+C cancels too rather than typing a `c`, and other Ctrl chords are ignored.
        app.on_key(key(KeyCode::Char('s')));
        type_str(&mut app, "no");
        app.on_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL));
        assert_eq!(typed(&app), Some("no"));
        app.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(app.prompt().is_none());
        assert_eq!(log.0.borrow().len(), 2);
    }

    #[test]
    fn rename_and_delete_act_on_the_active_preset() {
        let (mut app, log) = app_with_log();
        let first = app.library.presets()[0].name.clone();
        let second = app.library.presets()[1].name.clone();
        let count = app.library.presets().len();

        // With no active preset there is nothing to act on.
        app.on_key(key(KeyCode::Char('r')));
        assert!(app.prompt().is_none());
        assert!(app.notice().is_some());

        app.on_key(key(KeyCode::Char('p')));
        app.on_key(key(KeyCode::Char('r')));
        assert_eq!(typed(&app), Some(first.as_str()));
        for _ in 0..first.chars().count() {
            app.on_key(key(KeyCode::Backspace));
        }
        type_str(&mut app, "cosy");
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.active_preset(), Some("cosy"));

        // Renaming onto another preset is refused rather than destroying it.
        app.on_key(key(KeyCode::Char('r')));
        for _ in 0..4 {
            app.on_key(key(KeyCode::Backspace));
        }
        type_str(&mut app, &second.to_uppercase());
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.active_preset(), Some("cosy"));
        assert_eq!(app.library.presets().len(), count);

        // Delete asks first; anything but `y` keeps the preset.
        app.on_key(key(KeyCode::Char('d')));
        app.on_key(key(KeyCode::Char('n')));
        assert_eq!(app.library.presets().len(), count);
        app.on_key(key(KeyCode::Char('d')));
        app.on_key(key(KeyCode::Char('y')));
        assert_eq!(app.library.presets().len(), count - 1);
        assert_eq!(app.active_preset(), None);
        // `p` continues with whichever preset took its place.
        app.on_key(key(KeyCode::Char('p')));
        assert_eq!(app.active_preset(), Some(second.as_str()));
        assert_eq!(
            *log.0.borrow(),
            [format!("rename {first} cosy"), "remove cosy".to_string()]
        );
    }

    #[test]
    fn reset_replaces_every_preset_with_the_built_ins_after_confirming() {
        let (mut app, log) = app_with_log();
        let builtins = crate::presets::builtin();
        app.on_key(key(KeyCode::Char('5')));
        app.on_key(key(KeyCode::Char('s')));
        type_str(&mut app, "mine");
        app.on_key(key(KeyCode::Enter));
        app.on_key(key(KeyCode::Char('p')));
        app.on_key(key(KeyCode::Char('d')));
        app.on_key(key(KeyCode::Char('y')));
        assert_ne!(app.library.presets(), builtins);

        // Anything but `y` leaves the presets alone.
        app.on_key(key(KeyCode::Char('R')));
        app.on_key(key(KeyCode::Esc));
        assert_ne!(app.library.presets(), builtins);
        assert!(!app.should_quit());

        app.on_key(key(KeyCode::Char('R')));
        app.on_key(key(KeyCode::Char('y')));
        assert_eq!(app.library.presets(), builtins);
        assert_eq!(app.active_preset(), None);
        assert_eq!(log.0.borrow().last().map(String::as_str), Some("reset"));
        // Cycling starts again from the first built-in.
        app.on_key(key(KeyCode::Char('p')));
        assert_eq!(app.active_preset(), Some(builtins[0].name.as_str()));
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
