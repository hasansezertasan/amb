mod app;
mod audio;
mod presets;
mod state;
mod synth;
mod ui;

use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event};

fn main() -> Result<()> {
    // Open audio before touching the terminal so failures print normally.
    let engine = audio::Engine::start()?;
    state::load(&engine.shared);

    let mut app = app::App::new(engine.shared.clone())
        .with_presets(presets::load())
        .with_saver(presets::save);
    let mut terminal = ratatui::try_init().context("amb needs an interactive terminal")?;
    let result = run(&mut terminal, &mut app);
    ratatui::restore();

    state::save(&engine.shared);
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut app::App) -> Result<()> {
    while !app.should_quit() {
        terminal.draw(|f| ui::draw(f, app))?;
        if event::poll(Duration::from_millis(250))?
            && let Event::Key(key) = event::read()?
        {
            app.on_key(key);
        }
    }
    Ok(())
}
