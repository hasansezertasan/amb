# amb

[![Keep the Why · live](https://hasansezertasan.github.io/amb/dashboard/live/badge-entries.svg)](https://hasansezertasan.github.io/amb/dashboard/live/)

A terminal ambient-sound mixer, in the spirit of [A Soft Murmur](https://asoftmurmur.com).
Mix rain, thunder, wind, waves, fire, a stream, birds and crickets right from your terminal.

Every sound is synthesised in real time (noise, filters and oscillators),
so there are no audio files to download and nothing to loop.

## Install

With [mise](https://mise.jdx.dev) (prebuilt binary, no Rust needed):

```sh
mise use -g github:hasansezertasan/amb
```

With Cargo:

```sh
cargo install amb --locked
```

On Linux, building from source needs the ALSA headers (`libasound2-dev` on Debian/Ubuntu).

## Run

```sh
amb
```

## Keys

| Key | Action |
| --- | --- |
| `↑` `↓` / `j` `k` | select a sound |
| `←` `→` / `h` `l` | change its level by 5% |
| `0`-`9` | set its level to 0-90% |
| `p` / `P` | next / previous preset |
| `s` | save the current mix as a preset (type a name, `enter`; an existing name is overwritten) |
| `r` | rename the active preset (the one named in the header) |
| `d` | delete the active preset, after a `y` to confirm |
| `R` | reset every preset to the built-ins, after a `y` to confirm (this deletes presets you saved) |
| `+` / `-` | master volume |
| `space` | pause / resume |
| `m` | silence every sound |
| `q` / `Esc` | quit |

Levels are remembered in `~/.config/amb/levels` (honouring `$XDG_CONFIG_HOME`), or under `%APPDATA%` on Windows.

Presets live next to it in `presets.toml`. The built-in ones (rainy night, campfire, beach, forest, storm) are written there on first run; edit them, save the current mix with `s`, or add your own `[name]` section of `sound = level` lines (levels 0 to 1, sounds left out are silent). Saving, renaming and deleting only touch the section concerned; the rest of the file, comments included, is left alone. Renaming onto a name that is already taken is refused.
