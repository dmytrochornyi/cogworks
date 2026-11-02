# Cogworks

A cozy 2D automation game written in Rust with [macroquad](https://macroquad.rs).
Mine ore by hand, lay belts and robotic arms, smelt and assemble your way up a
research tree, keep the smoke-sensitive hives at bay, and finally wake the
**Grand Engine**.

## Features

- **Logistics:** two-lane conveyors, tunnel belts, splitters, arms and crates
- **Production:** drills, smelters, assemblers and labs with a multi-tier research tree
- **Power:** coal generators, solar panels and power poles
- **Fluids:** wellheads, pipes, refineries and mixers (crude tar, naphtha)
- **Defence:** pollution drifts toward hives at the map's edge; build walls and sentries, or switch on peaceful mode
- **Atmosphere:** day/dusk cycle with lantern lighting, lo-fi soundtrack, ambience and foley
- Save/load plus autosave, minimap and pollution overlay

## Running

Requires a recent stable Rust toolchain (edition 2024, Rust 1.85+).

```sh
cargo run --release
```

On Linux you may need the usual macroquad system packages (X11/Wayland, OpenGL
and ALSA development headers), e.g. on Debian/Ubuntu:

```sh
sudo apt install pkg-config libx11-dev libxi-dev libgl1-mesa-dev libasound2-dev
```

Sprites and audio are embedded in the binary, so the built executable runs on
its own. Saves are written to `cogworks_save.json` / `cogworks_autosave.json`
in the working directory.

## Controls

| Key | Action |
| --- | --- |
| WASD / arrows | Walk |
| Mouse wheel | Zoom |
| Left click | Build (drag for belts, pipes, walls) or open a building |
| Right hold | Mine ore by hand, or pick a building back up |
| 1–0 | Quickbar (hover an inventory item and press a number to bind it) |
| R / Shift+R | Rotate clockwise / counter-clockwise |
| Q | Copy the hovered building, or empty your hands |
| E / Tab | Backpack and workbench |
| T | Research |
| Esc | Close panel / drop tool |
| Space | Pause |
| O | Pollution overlay |
| P | Peaceful mode |
| M | Toggle music |
| F5 / F9 | Save / load |
| H / F1 | Help note |

## Development

```sh
cargo test                                   # unit tests
cargo run --release --bin gen_assets         # regenerate every sprite in assets/
```

All sprites are procedurally painted by `src/bin/gen_assets`. The audio is
built offline from sample packs by `tools/music/build_music.py`; see
[`tools/music/README.md`](tools/music/README.md).

## Assets and licences

- **Code and sprites:** MIT, see [`LICENSE`](LICENSE).
- **Font:** [M PLUS Rounded 1c](https://fonts.google.com/specimen/M+PLUS+Rounded+1c),
  SIL Open Font License 1.1, see [`assets/fonts/OFL.txt`](assets/fonts/OFL.txt).
- **Music, ambience and sound effects** (`assets/audio/`): original arrangements
  and edits built from the free sample packs by
  [Mondo Loops](https://mondoloops.com) (*Dreamy Lofi Hip Hop Samples Vol. 1* and
  *Organic Lofi Hip Hop Vol. 1*). These files are **not** covered by the MIT
  licence. They may not be extracted and reused as samples, and must not be
  registered with Content ID. The raw packs are not included in this repository.
