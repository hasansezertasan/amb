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
| `+` / `-` | master volume |
| `space` | pause / resume |
| `m` | silence every sound |
| `q` / `Esc` | quit |

Levels are remembered in `~/.config/amb/levels` (honouring `$XDG_CONFIG_HOME`), or under `%APPDATA%` on Windows.
