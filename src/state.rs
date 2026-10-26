//! Remembers mixer levels between runs in the platform config dir (`~/.config/amb/levels`).

use std::fs;
use std::path::PathBuf;

use etcetera::{AppStrategy, AppStrategyArgs, choose_app_strategy};

use crate::audio::Shared;
use crate::synth::Kind;

/// A file in amb's config dir.
pub fn path(file: &str) -> Option<PathBuf> {
    // XDG on macOS/Linux (ignoring empty or relative values), %APPDATA% on Windows.
    let strategy = choose_app_strategy(AppStrategyArgs {
        top_level_domain: String::new(),
        author: String::new(),
        app_name: "amb".to_string(),
    })
    .ok()?;
    Some(strategy.config_dir().join(file))
}

pub fn load(shared: &Shared) {
    let Some(text) = path("levels").and_then(|p| fs::read_to_string(p).ok()) else {
        return;
    };
    for line in text.lines() {
        let Some((key, val)) = line.split_once('=') else {
            continue;
        };
        let Ok(val) = val.trim().parse::<f32>() else {
            continue;
        };
        let key = key.trim();
        if key == "master" {
            shared.set_master(val);
        } else if let Some(i) = Kind::ALL
            .iter()
            .position(|k| k.name().eq_ignore_ascii_case(key))
        {
            shared.set_level(i, val);
        }
    }
}

pub fn save(shared: &Shared) {
    let Some(path) = path("levels") else { return };
    let mut out = format!("master={:.2}\n", shared.master());
    for (i, k) in Kind::ALL.iter().enumerate() {
        out.push_str(&format!(
            "{}={:.2}\n",
            k.name().to_lowercase(),
            shared.level(i)
        ));
    }
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    // Best effort: failing to remember levels should never crash the app.
    // Write-then-rename so a kill mid-save can't leave a truncated file.
    let tmp = path.with_extension("tmp");
    if fs::write(&tmp, out).is_ok() {
        let _ = fs::rename(tmp, path);
    }
}
