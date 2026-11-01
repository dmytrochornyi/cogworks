# Lantern-Dusk Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. In this project, tasks are delegated to Reasonix one at a time (delegate-reasonix skill), and each one is audited before the next starts.

**Goal:** Restyle Cogworks into the "lantern market at dusk" look:
- sunny lush days
- lavender dusk with red paper lanterns
- cream-paper and red-lacquer UI
- every building restyled
- M PLUS Rounded 1c font, rendered sharp

No gameplay changes.

**Architecture:**
- All colour lives in `src/theme.rs` tokens, shared by the game and the procedural sprite generator (`src/bin/gen_assets/`).
- Sprites are regenerated with `cargo run --release --bin gen_assets`, and the output under `assets/` is committed.
- The day/night look comes from `sky()` in `src/render.rs` plus the `COMPOSITE` shader in `src/assets.rs`, mirrored by `theme::grade()`.
- UI chrome is drawn by `surface_alpha`, `Panel`, `bar`, `button_kind` and `slot` in `src/ui.rs`.

**Tech Stack:** Rust 2024, macroquad 0.4.16, glam 0.27. No new crates.

**Spec:** `docs/superpowers/specs/2026-11-01-lantern-dusk-redesign-design.md`. Read it before every task. Mood references are in `docs/superpowers/specs/assets/`.

## Global Constraints

- No gameplay, balance, controls, layout, or mechanics changes. Panel positions and sizes stay as they are.
- No new crate dependencies. Font files are assets.
- `src/theme.rs` is the single source of truth for colours. No raw hex literals outside it, except in `paint.rs`'s `hex()` helper.
- Sprite canvas sizes and anchor points (`LANTERN`, the light positions in `draw_lights`, arm and beam pivots) stay unchanged.
- Keep the UI camera Y-flip in `src/ui.rs` `draw()`. It's needed by macroquad 0.4.16 for cameras without a render target.
- No film grain or any per-frame screen noise.
- Copy no characters, creatures, logos or specific designs from the reference films. Mood only.
- After every task:
  - `cargo test --release` passes
  - `cargo build --release` gives no new warnings
  - if any sprite code or token changed, run `cargo run --release --bin gen_assets` and commit the regenerated `assets/` and `src/assets_list.rs` in the same commit
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **UI upside down or misaligned clicks after restyling.** Expected: the UI renders upright and `ui_mouse()` hit-tests match what's drawn. Pinned by `ui_camera_is_y_down` (Task 1).
2. **A sprite's canvas size silently changes** during the restyle and breaks placement, lights or pivots. Expected: identical dimensions. Pinned by `sprite_sizes_match_baseline` (Task 1).
3. **Pale icons or dim text vanish on cream paper.** Expected: readable contrast everywhere. Pinned by `ui_text_contrast_is_readable` and `items_stand_out_in_slot_well` (Task 2).
4. **Night too dark to play, or lanterns only at midnight.** Expected: a lavender night that's always playable, and lanterns from golden hour. Pinned by `night_is_never_pitch_dark`, `lanterns_glow_from_golden_hour` and `dusk_phase_is_long` (Task 5).
5. **Text blurry or layouts shifting at 1440p and 4K after the font change.** Expected: sharp text and the same measured widths. Pinned by `raster_params_render_at_physical_size` (Task 3).

---

### Task 1: Regression guards (camera orientation and sprite sizes)

**Files:**
- Modify: `src/ui.rs`: extract the camera built in `draw()` (around line 539) into a function, and add a test.
- Create: `src/sprite_sizes.txt`: the baseline, one line per sprite: `name width height`.
- Modify: `src/assets.rs`: add a `#[cfg(test)] mod tests`.

**Interfaces:**
- Produces: `pub fn ui_camera(w: f32, h: f32) -> Camera2D` in `src/ui.rs`, used by `draw()`.

- [ ] **Step 1:** Write `src/sprite_sizes.txt` from the current PNGs, sorted by name. For example, use Python reading bytes 16..24 of each PNG (the IHDR width and height, big-endian u32).
- [ ] **Step 2:** Write the failing tests.
  - In `src/ui.rs`, add `ui_camera_is_y_down`: build `ui_camera(1000.0, 500.0)` and use `Camera::matrix()`.
    - `project_point3(vec3(0,0,0))` must be ≈ `(-1, 1)`.
    - `project_point3(vec3(1000,500,0))` must be ≈ `(1, -1)`.
    - Tolerance: 1e-4.
  - In `src/assets.rs`, add `sprite_sizes_match_baseline`. For every `(name, bytes)` in `crate::assets_list::SPRITES`:
    - read the width and height from the PNG header
    - assert they equal the line for that name in `include_str!("sprite_sizes.txt")`
    - assert both lists have the same set of names
- [ ] **Step 3:** Run `cargo test --release ui_camera_is_y_down`. Expected: FAIL to compile (`ui_camera` doesn't exist yet).
- [ ] **Step 4:** Implement `ui_camera`. It's `from_display_rect` plus the existing `zoom.y = -zoom.y` flip, with its comment moved along. Make `draw()` call it.
- [ ] **Step 5:** Run `cargo test --release`. Expected: all pass, including both new tests.
- [ ] **Step 6:** Commit: `test: guard UI orientation and sprite sizes`.

### Task 2: New palette in theme.rs

**Files:**
- Modify: `src/theme.rs`: the palette ramps, semantic tokens and tests.
- Modify: `src/ui.rs` constants block (lines 19–31) only if token names change.
- Regenerate: `assets/`, `src/assets_list.rs`.

**Interfaces:**
- Produces these tokens. Later tasks use them by name.
  - Add new ramps:
    - `LACQUER_700/600/500/400/300` (lacquer reds; 500 is roughly `0xc8453a`)
    - `VERDIGRIS_600/500/400` (teal trim)
    - `PAPER_100/200/300` (creams; 100 is roughly `0xf6ead2`)
    - `INKBROWN_900/700/500` (text; 900 is roughly `0x3b2a2e`)
    - `MEADOW_700..200` (lush soft greens, replacing the olive SAGE grass values)
    - `DUSK_700/500/300` (lavender-blues)
  - Re-point the semantic UI tokens:
    - `BG` → `PAPER_100`, `BG2` → `PAPER_200`, `EDGE` → `LACQUER_500`
    - `HONEY` stays gold. It's used for keys and accents.
    - `SHADE` stays a dark shadow colour.
  - **Do not change `CREAM` or `DIM`.** `CREAM` is also used as a paint colour by sprites
    (`sprites.rs`) and by world overlays (`render.rs:910,923`, `main.rs:462`). Instead:
    - add `TEXT` (= `INKBROWN_900`) and `TEXT_DIM` (= `INKBROWN_500`)
    - in `src/ui.rs`, rename the local constants `CREAM` → `TEXT` and `DIM` → `TEXT_DIM`, pointing
      them at the new tokens, and update every use in `ui.rs`
    - `ui::CREAM` is `pub`; fix any outside users to `ui::TEXT`
    - leave `theme::CREAM` as a light paint colour
  - New tokens:
    - `TRIM_SHADOW` (darker lacquer, for the panel's under-edge)
    - `SLOT_WELL` (warm darker paper, e.g. `PAPER_300`)
    - `TITLE` (= `LACQUER_600`)
  - Keep every existing token name compiling, so sprites and render code still build. Re-pointing values is fine.
- Produces in `theme.rs`:
  - `pub fn luminance(hex: u32) -> f32` (WCAG relative luminance)
  - `pub fn contrast(a: u32, b: u32) -> f32` (WCAG contrast ratio)

- [ ] **Step 1:** Write the failing tests in `src/theme.rs` `#[cfg(test)] mod tests`.
  - `ui_text_contrast_is_readable`:
    - `contrast(TEXT, BG) >= 7.0`
    - `contrast(TEXT_DIM, BG) >= 4.5`
    - `contrast(TITLE, BG) >= 4.5`
    - `contrast(TEXT, BG2) >= 4.5`
  - `items_stand_out_in_slot_well`: list every `ITEM_*` token in an array in the test, and
    assert `contrast(item, SLOT_WELL) >= 1.6` for each. Make it pass by re-tuning the near-cream
    ones (`ITEM_POLYMER`, `ITEM_LENS`, `ITEM_GLASS`, `ITEM_IRON_PLATE`, `ITEM_QUARTZ`,
    `ITEM_STONE`) a step darker or more saturated, or by making `SLOT_WELL` darker. In both
    cases the `TEXT`-on-`SLOT_WELL` contrast must stay ≥ 4.5, because slot counts are drawn on
    it. Add that assertion too.
- [ ] **Step 2:** Run `cargo test --release theme`. Expected: FAIL (`contrast` isn't defined).
- [ ] **Step 3:** Implement the ramps, the re-pointing, and `luminance`/`contrast`. Terrain tokens (`GRASS_*`, `BLADE_*`, `MOSS*`, `MAP_GRASS`, `WORLD_CLEAR`) move to `MEADOW_*` values. Other sprite tokens may keep their values for now. Later tasks restyle them.
- [ ] **Step 4:** Run `cargo test --release`. Expected: PASS.
- [ ] **Step 5:** Run `cargo run --release --bin gen_assets`, then `cargo test --release` (the size baseline must still pass).
- [ ] **Step 6:** Commit: `feat: lantern-dusk palette tokens` (include the regenerated assets).

### Task 3: M PLUS Rounded 1c, sharp text

**Files:**
- Create: `assets/fonts/MPLUSRounded1c-Medium.ttf`, `assets/fonts/MPLUSRounded1c-Bold.ttf`, `assets/fonts/OFL.txt` (replace Nunito's).
  - Source: `https://github.com/google/fonts/tree/main/ofl/mplusrounded1c`. Use raw downloads of the static files and their `OFL.txt`.
  - If `pyftsubset` is available, subset both files to Basic Latin, Latin-1 and the symbols `· × → ← ↑ ↓ … – —`. If not, ship the full files.
- Delete: `assets/fonts/Nunito.ttf`.
- Modify:
  - `src/assets.rs`: add `font_bold: Font`, and load both fonts.
  - `src/theme.rs`: add `TextSize::bold(self) -> bool` (true for `Xl` and `Xxl`).
  - `src/ui.rs`: `text`, `text_w`, `text_in`, and every other `measure_text`/`draw_text_ex` call.

**Interfaces:**
- Produces:
  - `pub fn raster_params(px: f32, scale: f32) -> (u16, f32)` in `src/ui.rs`. It returns `((px * scale).round() as u16, 1.0 / scale)`, i.e. the physical font size and the `font_scale`.
  - `fn font_for(a: &Assets, size: TextSize) -> &Font`.
- All text drawing and measuring goes through `raster_params(size.px(), ui_scale())` and `font_for`, so drawn and measured widths agree.

- [ ] **Step 1:** Write the failing test `raster_params_render_at_physical_size`:
  - `raster_params(17.0, 1.6) == (27, 0.625)`
  - `raster_params(17.0, 1.0) == (17, 1.0)`
  - `raster_params(13.0, 0.8).0 == 10`
- [ ] **Step 2:** Run `cargo test --release raster_params`. Expected: FAIL (not defined).
- [ ] **Step 3:** Download the fonts (and subset them if possible). Wire up `font_bold`, `raster_params` and `font_for`. Route every text draw and measure through them. Remove Nunito.
- [ ] **Step 4:** Run `cargo test --release`. Expected: PASS. Also check with `fc-query --format '%{charset}\n' assets/fonts/MPLUSRounded1c-Medium.ttf` that it covers U+00B7 (·), U+00D7 (×) and U+2192 (→).
- [ ] **Step 5:** Run the game with `COGWORKS_WIDTH=1440 COGWORKS_HEIGHT=900` and take a screenshot. The help panel text must sit fully inside its panel. If M PLUS reads small, nudge `TextSize::px` values up by at most 1 px.
- [ ] **Step 6:** Commit: `feat: M PLUS Rounded 1c font rendered at physical size`.

### Task 4: Cream-paper UI chrome

**Files:**
- Modify: `src/ui.rs`:
  - `surface_alpha` (around line 286), `Panel::show` title colour, `bar`, `button_kind`, `slot`, `recipe_icon`
  - `hud`, `quickbar`, `craft_queue`, `minimap` frame, `radio`, `toasts`, `help`, `overlays`, `draw_tooltip`

**Interfaces:**
- Consumes the Task 2 tokens: `BG`, `BG2`, `EDGE`, `TRIM_SHADOW`, `SLOT_WELL`, `TITLE`, `TEXT`, `TEXT_DIM`, `HONEY`, `LACQUER_500`.
- Also check the off-UI chrome in `src/main.rs:476` (cursor badge drawn with `theme::BG`). It will turn paper-coloured, which is intended. Its contents must stay readable.

- [ ] **Step 1:** Restyle `surface_alpha`:
  - paper fill (`BG`; Card uses `BG2`)
  - a lacquer-red border, `line::STRONG` wide, for Panel, Tooltip, Toast and Banner
  - a 3 px `TRIM_SHADOW` under-edge offset straight down, plus the existing soft drop shadow at lower alpha
  - remove the 8%-alpha bevel line
- [ ] **Step 2:** Panel titles use `TITLE`. Slots draw a `SLOT_WELL` fill under the icon. The selected slot gets a `LACQUER_500` ring. Bars draw their track in `SLOT_WELL` instead of near-black. Primary buttons are lacquer red with paper text; secondary buttons are paper with a red outline.
- [ ] **Step 3:** Sweep the remaining UI functions listed above for any colour that assumed a dark background (white highlights, `SHADE` fills at high alpha) and re-point it to the paper tokens.
- [ ] **Step 4:** Run `cargo test --release`. Expected: PASS.
- [ ] **Step 5:** Run the game and take screenshots with:
  - the help panel open
  - the inventory open (E)
  - the research panel open (T)
  - the cursor over an item (tooltip)
  
  Every panel must be cream with red trim, upright, and readable.
- [ ] **Step 6:** Commit: `feat: cream paper and lacquer-red UI`.

### Task 5: Sunny days, long lantern dusk

**Files:**
- Modify:
  - `src/render.rs`: `Sky` (add a `glow` field), `sky()`, `draw_lights`, `update_particles`, `frame`
  - `src/assets.rs`: the `COMPOSITE` grade numbers
  - `src/theme.rs`: `grade()`, mirroring the shader
  - `src/main.rs`: read the env var `COGWORKS_DAY_PHASE` (0..1) at startup to set the starting time of day, for screenshots. Unset means today's behaviour.

**Interfaces:**
- Produces:
  - `pub struct Sky { ambient: Color, night: f32, glow: f32, label: &'static str }`, where `glow` (0..1) is how strongly lanterns, windows and lamps shine.
  - `pub fn sky_at_phase(p: f32) -> Sky`: the pure core of `sky()`, where `p` is the day fraction with 0 = midnight. `sky(time)` calls it with `(time / DAY_LENGTH + 0.3).fract()`.

- [ ] **Step 1:** Write the failing tests in `src/render.rs` tests, sampling `sky_at_phase` at 1000 points over `[0,1)`:
  - `night_is_never_pitch_dark`: the minimum ambient average of `(r+g+b)/3` is ≥ 0.45, and at the darkest point blue > red.
  - `days_are_bright_and_sunny`: at `p = 0.5`, the ambient average is ≥ 0.95 and `glow < 0.1`.
  - `lanterns_glow_from_golden_hour`: `sky_at_phase(0.68).glow >= 0.3` and `sky_at_phase(0.85).glow >= 0.8`.
  - `dusk_phase_is_long`: the fraction of samples with `glow >= 0.5` is ≥ 0.40.
- [ ] **Step 2:** Run `cargo test --release sky`. Expected: FAIL (not defined).
- [ ] **Step 3:** Implement the new key table:
  - a short dawn
  - a bright, slightly green-warm midday
  - golden hour from about 0.62
  - a long lavender dusk (`DUSK_*` colours)
  - a lavender night
  
  Compute `glow` from its own smooth curve, not from luminance. Use `glow` in `draw_lights` and the firefly light in place of `night`, where lamps are concerned. Grow the lamp light radius from 3.6 to about 4.2 and tint it warm orange (`LAMP`). By day, golden-dust motes become slightly more visible.
- [ ] **Step 4:** Add the test `composite_has_no_grain` in `src/assets.rs` tests: `COMPOSITE` contains neither `"fract(sin("` nor `"Params.x"`. Retune `COMPOSITE` toward pastel softness: a slightly lower contrast lift and a lavender shadow tint instead of plum. Update `theme::grade()` to the same numbers.
- [ ] **Step 5:** Run `cargo test --release`. Expected: PASS.
- [ ] **Step 6:** Take screenshots at `COGWORKS_DAY_PHASE` = 0.5, 0.68, 0.8 and 0.95, with at least one lamp pole placed near the player. The look must progress sunny → golden → lantern dusk → lavender night.
- [ ] **Step 7:** Commit: `feat: sunny days and a long lantern-lit dusk`.

### Task 6: Ground, ores, decorations, belts

**Files:**
- Modify:
  - `src/bin/gen_assets/sprites.rs`: `grass`, `decor`, `rock`/`ore`, `tar`, `belt`, `belt_curve`, `tunnel`, `splitter`
  - `src/theme.rs`: the terrain, ore and belt tokens
- Regenerate the assets.

- [ ] **Step 1:** Grass and decorations: lush pastel meadow greens (`MEADOW_*`), softer tufts, and a few small pastel flowers (pink, butter-yellow, lilac) in the `decor` variants.
- [ ] **Step 2:** Ores stay distinct: grey-blue iron, warm copper, near-black coal, cream stone, lilac quartz, each with soft highlights. Belts become a dark wood/lacquer band with brass rails (spec "My calls").
- [ ] **Step 3:** Run `cargo run --release --bin gen_assets && cargo test --release`. Expected: PASS (sizes unchanged).
- [ ] **Step 4:** Screenshot at `COGWORKS_DAY_PHASE=0.5` with all ore types and a belt in view. Each ore must be identifiable at a glance.
- [ ] **Step 5:** Commit: `art: meadow ground, ores and belts`.

### Task 7: Red paper lanterns and roofed machines

**Files:**
- Modify:
  - `src/bin/gen_assets/sprites.rs`: `pole`, `smelter`, `assembler`, `lab`, `generator`, `refinery`, `mixer`, `drill`, `wellhead`, `engine`
  - `src/theme.rs`: the machine tokens
- Regenerate the assets.

**Interfaces:**
- Consumes: `LACQUER_*`, `VERDIGRIS_*`, `PAPER_*`, `LAMP`, `LAMP_INNER`.
- Must not move: the `pole` lantern position (it must stay where `LANTERN` in `render.rs` points), or the window and furnace spots at the `draw_lights` coordinates for smelter, generator, lab and assembler.

- [ ] **Step 1:** Pole: the wooden post stays. The hanging lantern becomes an oval red paper lantern with dark top and bottom caps, faint vertical ribs and a warm inner gradient. It sits on the same spot.
- [ ] **Step 2:** For each of the nine machines, paint a curved red-lacquer roof with a teal (verdigris) ridge or trim inside the existing canvas, over cream or wood walls. Windows sit at the existing light coordinates in a warm `LAMP_INNER` tone. Keep the drill bit, assembler gear, generator wheel, mixer paddle, wellhead beam and engine ring overlays aligned with their current pivots.
- [ ] **Step 3:** Run `cargo run --release --bin gen_assets && cargo test --release`. Expected: PASS.
- [ ] **Step 4:** Build one of each machine plus a pole (using save/load or play), then screenshot at day (0.5) and dusk (0.8). Lights must sit on windows and lanterns.
- [ ] **Step 5:** Commit: `art: paper lanterns and roofed market-stall machines`.

### Task 8: Everything else (repainted, no roofs)

**Files:**
- Modify:
  - `src/bin/gen_assets/sprites.rs`: `chest`, `wall`, `sentry_base`, `sentry_head`, `pipe_hub`, `pipe_seg`, `port`, `chute`, `drill_bit`, `gear_overlay` (call site colour), `wheel`, `paddle`, `beam`, `engine_ring`, `arm_base`, `arm_limb`, `arm_hand`, `solar`, `hive`, `player`, `mite`, `item`, `icon`, `smoke`, `cloud`
  - `src/theme.rs`: the remaining tokens
- Regenerate the assets.

- [ ] **Step 1:** Repaint these in the new palette: wood, brass, paper, lacquer accents, and verdigris for metal. The smoke tint goes warmer and softer. The cloud shadow colour goes lavender instead of plum. Enemies (`mite`, brute, `hive`) keep saturated warm-pink and purple shells, so they read as threats on green (spec "My calls"). Item icons follow the Task 2 item tokens. The player gets the new palette with the same silhouette.
- [ ] **Step 2:** Run `cargo run --release --bin gen_assets && cargo test --release`. Expected: PASS.
- [ ] **Step 3:** Screenshot the inventory panel (all owned items visible) and the world with a chest, wall, sentry, pipes, arms and solar in view. Icons must be readable on `SLOT_WELL`.
- [ ] **Step 4:** Commit: `art: repaint remaining buildings, items and characters`.

### Task 9: Whole-look verification

- [ ] **Step 1:** Grep `src/` (excluding `theme.rs` and `paint.rs`) for raw hex literals `0x[0-9a-fA-F]{6}` used as colours. Expected: none new compared to `da87bfe`.
- [ ] **Step 2:** Run `cargo build --release 2>&1 | grep -c warning`. Expected: no higher than at `da87bfe`.
- [ ] **Step 3:** Run the game at 1440x900 and full 2560x1440. Take screenshots at day phases 0.5, 0.68, 0.8 and 0.95 with the help panel, inventory and research open in turn. Check against the spec's "Done when" list. Click a quickbar slot and a panel's close button to confirm hit-testing.
- [ ] **Step 4:** Fix anything that fails, then commit: `chore: lantern-dusk redesign polish`.
