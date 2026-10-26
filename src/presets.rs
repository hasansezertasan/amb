//! Named mixes, read from `presets.toml` in the platform config dir.
//!
//! The built-ins are written there on first run so they can be edited. The
//! file is a TOML subset: `[name]` sections of `sound = level` lines.

use std::fs;
use std::io::{self, ErrorKind};

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
        Ok(text) => {
            let presets = parse(&text);
            if presets.is_empty() {
                builtin()
            } else {
                presets
            }
        }
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
        "# amb presets: press p / P in the mixer to cycle through them, s to save the mix.\n\
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

/// Replace the section named like `preset` (ignoring case), or append one.
/// The rest of the file, comments included, is kept as written.
fn upsert(text: &str, preset: &Preset) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let Some(start) = lines
        .iter()
        .position(|l| section_name(l).is_some_and(|n| n.eq_ignore_ascii_case(&preset.name)))
    else {
        let mut out = text.trim_end().to_string();
        out.push_str("\n\n");
        out.push_str(&render_section(preset));
        return out;
    };
    let end = lines[start + 1..]
        .iter()
        .position(|l| section_name(l).is_some())
        .map_or(lines.len(), |p| start + 1 + p);
    // Blank and comment lines at the end of the old section lead into the
    // next one, so they stay.
    let mut tail = end;
    let leads_on = |l: &str| l.trim().is_empty() || l.trim().starts_with('#');
    while tail > start + 1 && leads_on(lines[tail - 1]) {
        tail -= 1;
    }
    let mut out = String::new();
    for l in &lines[..start] {
        out.push_str(l);
        out.push('\n');
    }
    out.push_str(&render_section(preset));
    for l in &lines[tail..] {
        out.push_str(l);
        out.push('\n');
    }
    out
}

/// Write `preset` into `presets.toml`, replacing one of the same name.
pub fn save(preset: &Preset) -> io::Result<()> {
    let path = state::path("presets.toml")
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "no config directory"))?;
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => render(&builtin()),
        Err(e) => return Err(e),
    };
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    // Write-then-rename so a kill mid-save can't truncate the user's file.
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, upsert(&text, preset))?;
    fs::rename(tmp, path)
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
    fn parse_skips_junk_and_clamps() {
        let text = "orphan = 0.5\n[ mix ]\nrain = 2\nwind = nan\nbogus = 0.3\nfire = x\n# c\n[]\n";
        let presets = parse(text);
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].name, "mix");
        assert_eq!(presets[0].levels[0], 1.0);
        assert_eq!(presets[0].levels[2], 0.0);
    }
}
