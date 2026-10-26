//! Named mixes, read from `presets.toml` in the platform config dir.
//!
//! The built-ins are written there on first run so they can be edited. The
//! file is a TOML subset: `[name]` sections of `sound = level` lines.

use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use crate::state;
use crate::synth::Kind;

#[derive(Clone, Debug, PartialEq)]
pub struct Preset {
    pub name: String,
    pub levels: [f32; Kind::ALL.len()],
}

const BUILTIN: &[(&str, &[(Kind, f32)])] = &[
    (
        "rainy night",
        &[
            (Kind::Rain, 0.7),
            (Kind::Thunder, 0.3),
            (Kind::Wind, 0.2),
            (Kind::Crickets, 0.2),
        ],
    ),
    (
        "campfire",
        &[(Kind::Fire, 0.7), (Kind::Wind, 0.1), (Kind::Crickets, 0.5)],
    ),
    (
        "beach",
        &[(Kind::Waves, 0.7), (Kind::Wind, 0.3), (Kind::Birds, 0.3)],
    ),
    (
        "forest",
        &[
            (Kind::Wind, 0.2),
            (Kind::Stream, 0.4),
            (Kind::Birds, 0.5),
            (Kind::Crickets, 0.1),
        ],
    ),
    (
        "storm",
        &[(Kind::Rain, 0.8), (Kind::Thunder, 0.7), (Kind::Wind, 0.6)],
    ),
];

pub fn builtin() -> Vec<Preset> {
    BUILTIN
        .iter()
        .map(|(name, sounds)| {
            let mut levels = [0.0; Kind::ALL.len()];
            for (kind, level) in *sounds {
                let i = Kind::ALL.iter().position(|k| k == kind).unwrap();
                levels[i] = *level;
            }
            Preset {
                name: (*name).to_string(),
                levels,
            }
        })
        .collect()
}

/// The user's presets; a missing file is seeded with the built-ins.
pub fn load() -> Vec<Preset> {
    let Some(path) = state::path("presets.toml") else {
        return builtin();
    };
    match fs::read_to_string(&path) {
        // An existing file is the user's, even with every section removed.
        Ok(text) => parse(&text),
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                // Best effort: running without a writable config dir is fine.
                if let Some(dir) = path.parent() {
                    let _ = fs::create_dir_all(dir);
                }
                let _ = fs::write(&path, render(&builtin()));
            }
            builtin()
        }
    }
}

fn parse(text: &str) -> Vec<Preset> {
    let mut presets: Vec<Preset> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            presets.push(Preset {
                name: name.trim().to_string(),
                levels: [0.0; Kind::ALL.len()],
            });
        } else if let (Some(preset), Some((key, val))) = (presets.last_mut(), line.split_once('='))
            && let Ok(val) = val.trim().parse::<f32>()
            && val.is_finite()
            && let Some(i) = Kind::ALL
                .iter()
                .position(|k| k.name().eq_ignore_ascii_case(key.trim()))
        {
            preset.levels[i] = val.clamp(0.0, 1.0);
        }
    }
    presets.retain(|p| !p.name.is_empty());
    presets
}

fn render_section(p: &Preset) -> String {
    let mut out = format!("[{}]\n", p.name);
    for (i, k) in Kind::ALL.iter().enumerate() {
        if p.levels[i] > 0.0 {
            out.push_str(&format!(
                "{} = {:.2}\n",
                k.name().to_lowercase(),
                p.levels[i]
            ));
        }
    }
    out
}

fn render(presets: &[Preset]) -> String {
    let mut out = String::from(
        "# amb presets: press p / P in the mixer to cycle through them, s / r / d to save, rename or delete one,\n# R to reset all of them to the built-ins.\n\
         # Each [section] is a preset; sounds left out are set to 0.\n",
    );
    for p in presets {
        out.push('\n');
        out.push_str(&render_section(p));
    }
    out
}

fn section_name(line: &str) -> Option<&str> {
    line.trim()
        .strip_prefix('[')
        .and_then(|l| l.strip_suffix(']'))
        .map(str::trim)
}

/// Line range of the section named `name` (ignoring case): its header up to,
/// but not including, the blank and comment lines that lead into the next one.
fn section_span(lines: &[&str], name: &str) -> Option<(usize, usize)> {
    let start = lines
        .iter()
        .position(|l| section_name(l).is_some_and(|n| n.eq_ignore_ascii_case(name)))?;
    let mut tail = lines[start + 1..]
        .iter()
        .position(|l| section_name(l).is_some())
        .map_or(lines.len(), |p| start + 1 + p);
    let leads_on = |l: &str| l.trim().is_empty() || l.trim().starts_with('#');
    while tail > start + 1 && leads_on(lines[tail - 1]) {
        tail -= 1;
    }
    Some((start, tail))
}

fn join_lines<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    parts.into_iter().fold(String::new(), |mut out, l| {
        out.push_str(l);
        out.push('\n');
        out
    })
}

/// Replace the section named like `preset`, or append one. The rest of the
/// file, comments included, is kept as written.
fn upsert(text: &str, preset: &Preset) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let Some((start, tail)) = section_span(&lines, &preset.name) else {
        let mut out = text.trim_end().to_string();
        out.push_str("\n\n");
        out.push_str(&render_section(preset));
        return out;
    };
    let section = render_section(preset);
    join_lines(
        lines[..start]
            .iter()
            .copied()
            .chain(section.lines())
            .chain(lines[tail..].iter().copied()),
    )
}

/// Drop the section named `name`; the text is unchanged if there is none.
fn remove_section(text: &str, name: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    match section_span(&lines, name) {
        Some((start, tail)) => join_lines(
            lines[..start]
                .iter()
                .copied()
                .chain(lines[tail..].iter().copied()),
        ),
        None => text.to_string(),
    }
}

/// Rename a section by rewriting only its header line.
fn rename_section(text: &str, from: &str, to: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    match section_span(&lines, from) {
        Some((start, _)) => {
            let header = format!("[{to}]");
            join_lines(
                lines
                    .iter()
                    .enumerate()
                    .map(|(i, l)| if i == start { header.as_str() } else { l }),
            )
        }
        None => text.to_string(),
    }
}

/// Where preset changes go. The app only keeps them in memory without one.
pub trait Store {
    fn save(&self, preset: &Preset) -> io::Result<()>;
    fn remove(&self, name: &str) -> io::Result<()>;
    fn rename(&self, from: &str, to: &str) -> io::Result<()>;
    /// Replace everything with the built-in presets.
    fn reset(&self) -> io::Result<()>;
}

/// `presets.toml` in the config dir.
pub struct FileStore;

impl Store for FileStore {
    fn save(&self, preset: &Preset) -> io::Result<()> {
        edit_at(&file_path()?, |text| upsert(text, preset))
    }

    fn remove(&self, name: &str) -> io::Result<()> {
        edit_at(&file_path()?, |text| remove_section(text, name))
    }

    fn rename(&self, from: &str, to: &str) -> io::Result<()> {
        edit_at(&file_path()?, |text| rename_section(text, from, to))
    }

    fn reset(&self) -> io::Result<()> {
        reset_at(&file_path()?)
    }
}

fn file_path() -> io::Result<PathBuf> {
    state::path("presets.toml")
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "no config directory"))
}

/// Read the file (the built-ins if it is gone), transform it and write it back.
fn edit_at(path: &Path, change: impl FnOnce(&str) -> String) -> io::Result<()> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => render(&builtin()),
        Err(e) => return Err(e),
    };
    write_atomic(path, &change(&text))
}

/// Replace the file with the built-ins without reading it, so a file that
/// can't be read as text can still be repaired.
fn reset_at(path: &Path) -> io::Result<()> {
    write_atomic(path, &render(&builtin()))
}

fn write_atomic(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    // Write-then-rename so a kill mid-save can't truncate the user's file.
    // The pid keeps two running instances from sharing a temp file.
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    let result = fs::write(&tmp, text).and_then(|()| fs::rename(&tmp, path));
    if result.is_err() {
        // Don't leave a stray temp file behind for every failed save.
        let _ = fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_survive_a_write_and_read() {
        let presets = builtin();
        assert_eq!(parse(&render(&presets)), presets);
    }

    fn preset(name: &str, rain: f32) -> Preset {
        let mut levels = [0.0; Kind::ALL.len()];
        levels[0] = rain;
        Preset {
            name: name.to_string(),
            levels,
        }
    }

    #[test]
    fn upsert_appends_a_new_preset() {
        let out = upsert("# mine\n[a]\nrain = 0.50\n", &preset("b", 0.3));
        assert_eq!(out, "# mine\n[a]\nrain = 0.50\n\n[b]\nrain = 0.30\n");
    }

    #[test]
    fn upsert_overwrites_a_duplicate_and_keeps_the_rest() {
        let text = "# top\n[a]\nrain = 0.50\nfire = 0.10\n\n# about b\n[b]\nwind = 0.2\n";
        let out = upsert(text, &preset("A", 0.9));
        assert_eq!(
            out,
            "# top\n[A]\nrain = 0.90\n\n# about b\n[b]\nwind = 0.2\n"
        );
        let parsed = parse(&out);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].levels[0], 0.9);
        assert_eq!(parsed[0].levels[4], 0.0);
    }

    #[test]
    fn remove_drops_one_section_and_keeps_the_rest() {
        let text = "# top\n[a]\nrain = 0.5\n\n# about b\n[b]\nwind = 0.2\n";
        assert_eq!(
            remove_section(text, "A"),
            "# top\n\n# about b\n[b]\nwind = 0.2\n"
        );
        assert_eq!(remove_section(text, "missing"), text);
    }

    #[test]
    fn reset_repairs_a_file_that_cannot_be_read_as_text() {
        let dir = std::env::temp_dir().join(format!("amb-presets-{}", std::process::id()));
        let path = dir.join("presets.toml");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, [0xff, 0xfe, b'[', b'a', b']']).unwrap();

        // Editing refuses to guess at a file it can't read; reset replaces it.
        assert!(edit_at(&path, |t| t.to_string()).is_err());
        reset_at(&path).unwrap();
        assert_eq!(parse(&fs::read_to_string(&path).unwrap()), builtin());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_failed_write_leaves_no_temp_file() {
        let dir = std::env::temp_dir().join(format!("amb-tmpfile-{}", std::process::id()));
        // A non-empty directory where the file should go makes the final rename fail.
        let path = dir.join("presets.toml");
        fs::create_dir_all(path.join("inner")).unwrap();

        assert!(write_atomic(&path, "x").is_err());
        let leftovers: Vec<_> = fs::read_dir(&dir).unwrap().flatten().collect();
        assert_eq!(leftovers.len(), 1, "only the blocking directory remains");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rename_rewrites_only_the_header() {
        let text = "[a]\nrain = 0.5 \n\n[b]\nwind = 0.2\n";
        assert_eq!(
            rename_section(text, "a", "new name"),
            "[new name]\nrain = 0.5 \n\n[b]\nwind = 0.2\n"
        );
    }

    #[test]
    fn parse_skips_junk_and_clamps() {
        let text = "orphan = 0.5\n[ mix ]\nrain = 2\nwind = nan\nbogus = 0.3\nfire = x\n# c\n[]\n";
        let presets = parse(text);
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].name, "mix");
        assert_eq!(presets[0].levels[0], 1.0);
        assert_eq!(presets[0].levels[2], 0.0);
    }
}
