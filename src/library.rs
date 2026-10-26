//! The list of presets, which one was applied last, and the changes to it.

use std::io;

use crate::audio::Shared;
use crate::presets::{self, Preset, Store};

/// Stands in when changes are not persisted, so they only last the session.
struct NullStore;

impl Store for NullStore {
    fn save(&self, _: &Preset) -> io::Result<()> {
        Ok(())
    }
    fn remove(&self, _: &str) -> io::Result<()> {
        Ok(())
    }
    fn rename(&self, _: &str, _: &str) -> io::Result<()> {
        Ok(())
    }
    fn reset(&self) -> io::Result<()> {
        Ok(())
    }
}

/// Where `cycle` steps on from.
#[derive(Clone, Copy)]
enum Cursor {
    Start,
    /// The preset applied last.
    At(usize),
    /// The slot a deleted preset left: forward continues with the preset now
    /// in it, backward with the one before it.
    Gap(usize),
}

pub struct Library {
    presets: Vec<Preset>,
    cursor: Cursor,
    store: Box<dyn Store>,
}

impl Library {
    pub fn new() -> Self {
        Self {
            presets: presets::builtin(),
            cursor: Cursor::Start,
            store: Box::new(NullStore),
        }
    }

    pub fn with_presets(mut self, presets: Vec<Preset>) -> Self {
        self.presets = presets;
        self
    }

    pub fn with_store(mut self, store: impl Store + 'static) -> Self {
        self.store = Box::new(store);
        self
    }

    #[cfg(test)]
    pub fn presets(&self) -> &[Preset] {
        &self.presets
    }

    fn index_of(&self, name: &str) -> Option<usize> {
        self.presets
            .iter()
            .position(|p| p.name.eq_ignore_ascii_case(name))
    }

    /// Name of the last applied preset, unless the levels have been changed since.
    pub fn active(&self, shared: &Shared) -> Option<&str> {
        let Cursor::At(i) = self.cursor else {
            return None;
        };
        let preset = self.presets.get(i)?;
        let unchanged = preset
            .levels
            .iter()
            .enumerate()
            .all(|(i, l)| (shared.level(i) - l).abs() < 0.01);
        unchanged.then_some(preset.name.as_str())
    }

    /// Apply the next (or previous) preset to the mixer.
    pub fn cycle(&mut self, forward: bool, shared: &Shared) {
        let n = self.presets.len();
        if n == 0 {
            return;
        }
        let next = match (self.cursor, forward) {
            (Cursor::Start, true) => 0,
            (Cursor::Start, false) => n - 1,
            (Cursor::At(i), true) => (i + 1) % n,
            (Cursor::Gap(i), true) => i % n,
            (Cursor::At(i) | Cursor::Gap(i), false) => (i + n - 1) % n,
        };
        for (i, l) in self.presets[next].levels.iter().enumerate() {
            shared.set_level(i, *l);
        }
        self.cursor = Cursor::At(next);
    }

    // The changes below hold for the session even when the write fails; each
    // returns the message to show, or `None` when there was nothing to do.

    /// Save the current levels under `name`, replacing a preset of that name.
    pub fn save(&mut self, name: &str, shared: &Shared) -> Option<String> {
        let name = name.trim();
        if name.is_empty() {
            return None;
        }
        let preset = Preset {
            name: name.to_string(),
            levels: std::array::from_fn(|i| shared.level(i)),
        };
        let result = self.store.save(&preset);
        let at = match self.index_of(name) {
            Some(i) => {
                self.presets[i] = preset;
                i
            }
            None => {
                self.presets.push(preset);
                self.presets.len() - 1
            }
        };
        self.cursor = Cursor::At(at);
        Some(outcome(result, format!("saved preset {name}")))
    }

    pub fn rename(&mut self, from: &str, to: &str) -> Option<String> {
        let to = to.trim();
        let at = self.index_of(from)?;
        if to.is_empty() || to == from {
            return None;
        }
        // Renaming onto another preset would silently destroy it.
        if self.index_of(to).is_some_and(|i| i != at) {
            return Some(format!("a preset named {to} already exists"));
        }
        let result = self.store.rename(from, to);
        self.presets[at].name = to.to_string();
        Some(outcome(result, format!("renamed {from} to {to}")))
    }

    pub fn delete(&mut self, name: &str) -> Option<String> {
        let at = self.index_of(name)?;
        let result = self.store.remove(name);
        self.presets.remove(at);
        self.cursor = Cursor::Gap(at);
        Some(outcome(result, format!("deleted preset {name}")))
    }

    /// Throw away every preset, saved ones included, for the built-ins.
    pub fn reset(&mut self) -> String {
        let result = self.store.reset();
        self.presets = presets::builtin();
        self.cursor = Cursor::Start;
        outcome(result, "presets reset to the built-ins".to_string())
    }
}

fn outcome(result: io::Result<()>, done: String) -> String {
    match result {
        Ok(()) => done,
        Err(e) => format!("couldn't write presets.toml: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deleting_steps_back_so_cycling_continues_in_place() {
        let shared = Shared::new();
        let mut lib = Library::new();
        let names: Vec<String> = lib.presets().iter().map(|p| p.name.clone()).collect();
        lib.cycle(true, &shared);
        lib.cycle(true, &shared);
        assert_eq!(lib.active(&shared), Some(names[1].as_str()));

        assert!(lib.delete(&names[1]).is_some());
        assert_eq!(lib.active(&shared), None);
        lib.cycle(true, &shared);
        assert_eq!(lib.active(&shared), Some(names[2].as_str()));
    }

    #[test]
    fn deleting_the_last_preset_still_lets_previous_reach_its_neighbour() {
        let shared = Shared::new();
        let mut lib = Library::new();
        let names: Vec<String> = lib.presets().iter().map(|p| p.name.clone()).collect();
        let last = names.len() - 1;
        lib.cycle(false, &shared);
        assert_eq!(lib.active(&shared), Some(names[last].as_str()));

        lib.delete(&names[last]);
        lib.cycle(false, &shared);
        assert_eq!(lib.active(&shared), Some(names[last - 1].as_str()));
    }

    #[test]
    fn changes_that_do_nothing_say_nothing() {
        let shared = Shared::new();
        let mut lib = Library::new();
        assert_eq!(lib.save("  ", &shared), None);
        assert_eq!(lib.rename("missing", "x"), None);
        assert_eq!(lib.delete("missing"), None);
    }
}
