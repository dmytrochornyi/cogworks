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

### Task 5b: Crisp rendering and gentler red

Spec "Revision" items 3 and 4.

**Files:**
- Modify:
  - `src/main.rs`: `conf()` gets `sample_count: 4`
  - `src/ui.rs`: `rrect`, `rrect_lines`, `surface_alpha`, `Minimap` filter
  - `src/assets.rs`: atlas mipmaps
  - `src/theme.rs`: the `LACQUER_*` ramp, `EDGE`, the `TRIM_*` tokens, plus a new `WOOD_TRIM` token

- [ ] **Step 1:** Write the failing test `soft_red_is_not_aggressive` in `src/theme.rs` tests:
  - HSV saturation of `LACQUER_500` ≤ 0.55
  - `contrast(TITLE, BG) >= 4.5` still holds
  - `EDGE == WOOD_TRIM`
  
  Add a small `fn saturation(hex: u32) -> f32` helper.
- [ ] **Step 2:** Run `cargo test --release theme`. Expected: FAIL.
- [ ] **Step 3:** Retune `LACQUER_*` toward a soft dusty rose-vermilion (500 roughly `0xc9705f`). Add `WOOD_TRIM` (warm mid wood-brown, roughly `0x9a6b4a`) and point `EDGE` at it. Panel borders become `line::BASE` (1.5) wood-brown; red is used only for titles, the selected slot ring and the primary button. `TRIM_SHADOW` becomes a darker wood. Re-run the contrast tests and fix any that break.
- [ ] **Step 4:** Smooth shapes:
  - Draw panel borders as two filled rounded rects (an outer one in the border colour, then an inner one in the fill colour, inset by the border width) instead of `rrect_lines`. Keep `rrect_lines` only for thin decorative outlines.
  - Raise corner segments from 6 to 12 in `rrect`, `rrect_lines` and `rrect_dashed_lines`.
  - Set `sample_count: 4` in `conf()`.
  - Switch the minimap texture filter to `FilterMode::Linear`.
- [ ] **Step 5:** Atlas mipmaps. If miniquad 0.4 exposes mipmap generation for the atlas texture (e.g. `texture_generate_mipmaps` plus a mipmap min-filter on the raw miniquad texture via `get_internal_gl().quad_context`), enable it and raise the atlas `PAD` to 8 px so small mips don't bleed. If it isn't available, skip this and say so in the report.
- [ ] **Step 6:** Run `cargo test --release`, then screenshots (the reviewer runs them): help panel, inventory, research.
- [ ] **Step 7:** Commit: `feat: crisp anti-aliased UI and a gentler red`.

### Task 5c: Make dusk read as lavender and golden hour as gold

Spec "Revision" item 5. The reviewer's screenshots showed golden hour as murky olive and dusk/night as teal-green, because the ambient multiplies green grass.

**Files:**
- Modify:
  - `src/render.rs`: `sky_at_phase` keys, lamp/player/firefly light colours, firefly count and size
  - `src/assets.rs`: `COMPOSITE`, a new uniform
  - `src/theme.rs`: `grade()`, `LAMP`

**Interfaces:**
- Produces: `pub struct Sky { …, tint: Color }`, a mood tint (rgb plus strength in `a`) that the composite applies as a colour-grade mix, mostly into shadows and midtones.
- `COMPOSITE` gets a new `uniform vec4 Tint;`. Do **not** use `Params.x`: `composite_has_no_grain` forbids it.

- [ ] **Step 1:** Write the failing tests in `src/render.rs`:
  - `golden_hour_is_gold`: at p = 0.68, the ambient has r > g > b and r − b ≥ 0.3.
  - `dusk_is_lavender`: at p = 0.8, `tint` has b > g and r > g, with strength ≥ 0.25.
  - `night_is_violet_not_teal`: at p = 0.95, the tint has b > g and r ≥ g.
- [ ] **Step 2:** Run `cargo test --release sky`. Expected: FAIL.
- [ ] **Step 3:** Implement the tint keys:
  - none by day
  - peach-gold at golden hour
  - lavender-pink at dusk
  - violet-blue at night
  
  Apply `Tint` in `COMPOSITE` as `col = mix(col, col * tint_rgb * 1.6 + tint_rgb * 0.08, strength * (1.0 - luma * 0.5))`, or an equivalent that keeps lamp-lit areas warm. Mirror it in `theme::grade()` with an extra `tint` parameter, used by the minimap. `LAMP` becomes warm peach-orange (roughly `0xffb27a`), and player and lamp light use it. Halve the firefly count and shrink their light radius from 0.7 to 0.45.
- [ ] **Step 4:** Run `cargo test --release`, then screenshots at `COGWORKS_DAY_PHASE` = 0.5 / 0.68 / 0.8 / 0.95 (the reviewer runs them).
- [ ] **Step 5:** Commit: `fix: golden hour gold, dusk lavender, warm lamp light`.

### Task 6: Seamless ground, distinct ores, decorations, belts

Spec goal 4 and "Revision" items 1–2.

**Files:**
- Modify:
  - `src/bin/gen_assets/sprites.rs`: `grass`, `decor`, `rock`/`ore`, `tar`, `belt`, `belt_curve`, `tunnel`, `splitter`
  - `src/bin/gen_assets/main.rs`: the grass sprite list
  - `src/render.rs`: `draw_ground`
  - `src/theme.rs`: the terrain, ore and belt tokens
  - `src/sprite_sizes.txt`: a deliberate baseline change, grass only
- Regenerate the assets.

**Interfaces:**
- Ground sprites are `grass_{i}_{j}` for i, j in 0..4: sixteen 64x64 tiles cut from **one** 256x256 canvas whose noise and blades are periodic with period 256. `draw_ground` uses `grass_{x.rem_euclid(4)}_{y.rem_euclid(4)}`, so neighbouring tiles always continue each other.
- `sprite_sizes.txt`: remove `grass_0..3` and add the 16 new names at 64x64. No other line may change.

- [ ] **Step 1:** Write the failing test `ground_tiles_are_seamless` in the gen_assets binary (a `#[cfg(test)]` in `sprites.rs`). Build the 256 canvas. For every pair of horizontally and vertically adjacent cut tiles, including the wrap from i=3 to i=0, the mean absolute RGB difference between touching edge pixel columns or rows must be < 0.04 (colours in 0..1). Make the 256 canvas through a pure `pub fn ground_sheet() -> Canvas` so it's testable.
- [ ] **Step 2:** Run `cargo test --release --bin gen_assets`. Expected: FAIL.
- [ ] **Step 3:** Implement the sheet: soft pastel meadow greens (`MEADOW_*`), gentle large-scale colour variation (period 256), wrapped blades. Then remove the `draw_rectangle` soil square in `draw_ground`. If ore fields need a ground tint, draw the existing `glow` sprite as a soft, low-alpha round blob instead of a square.
- [ ] **Step 4:** Ores, distinct in shape and colour, with sizes unchanged:
  - **iron:** blue-grey angular faceted chunks with 1–2 small silver glint highlights
  - **copper:** warm orange rounded nuggets with teal patina spots
  - **coal:** glossy black lumps with a faint purple sheen highlight
  - **stone:** pale sandy smooth rounded boulders
  - **quartz:** lilac crystal spikes (keep the current shape idea, refine it)
  
  Decorations get a few small pastel flowers (pink, butter-yellow, lilac). Belts become a dark wood/lacquer band with brass rails.
- [ ] **Step 5:** Run `cargo run --release --bin gen_assets && cargo test --release`. Expected: PASS, including the updated size baseline.
- [ ] **Step 6:** Screenshot at `COGWORKS_DAY_PHASE=0.5` with ore fields in view (the reviewer runs it). There must be no visible grid, no soil squares, and each ore must be identifiable at a glance.
- [ ] **Step 7:** Commit: `art: seamless meadow ground, distinct ores, belts`.

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
- [ ] **Step 2:** Use the "boiler room" language (spec "Revision" item 6): soft dusty-rose roofs combined with brass and copper pipes, rivets, dark warm wood, glowing furnace mouths and windows, and steam vents. Pick one vent point per machine and expose it as a `pub const` in `src/render.rs` (`VENTS: &[(Kind, Vec2)]`, in sprite pixels) for Task 8b. For each of the nine machines, paint a curved red-lacquer roof with a teal (verdigris) ridge or trim inside the existing canvas, over cream or wood walls. Windows sit at the existing light coordinates in a warm `LAMP_INNER` tone. Keep the drill bit, assembler gear, generator wheel, mixer paddle, wellhead beam and engine ring overlays aligned with their current pivots.
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

### Task 8b: Life and gentle animation

Spec "Revision" item 7. Everything is slow and smooth, never flickery.

**Files:**
- Modify:
  - `src/render.rs`: particles (`PKind`), `draw_ground` decorations, entity drawing, lights
  - `src/bin/gen_assets/sprites.rs` and `main.rs`: new sprites `lantern` (split from `pole`), `butterfly_0/1`, `petal`, `bird_shadow`, `steam`
  - `src/sprite_sizes.txt`: add the new sprite names (deliberate). `pole` keeps its size; only its lantern part moves into the separate `lantern` sprite.

**Interfaces:**
- Produces pure helpers in `src/render.rs`, all tested:
  - `pub fn sway(t: f32, seed: f32, amp: f32, period: f32) -> f32`: a smooth sine with a per-seed phase.
  - `pub const MAX_BUTTERFLIES: usize = 6; pub const MAX_PETALS: usize = 12; pub const MAX_STEAM: usize = 40;`

- [ ] **Step 1:** Write the failing tests:
  - `sway_is_gentle`: over 10 s sampled at 60 Hz with amp 0.08 and period 3.0, |sway| ≤ 0.08 and the per-frame change ≤ 0.01.
  - `particles_stay_capped`: run `update_particles` on a world with 30 active machines, by day and at night, for 60 s at dt = 1/60. Each kind's count stays ≤ its cap. If `update_particles` isn't testable without a window, extract its spawn logic into a pure function and test that.
- [ ] **Step 2:** Run `cargo test --release`. Expected: FAIL.
- [ ] **Step 3:** Implement:
  - **Active machines:** steam puffs from `VENTS` (`PKind::Steam`: soft white, rising, growing, fading over 2–3 s). Furnace and window light breathes (a slow 0.85–1.0 sine plus a very small flicker). An occasional spark from smelters.
  - **Lanterns:** the lantern is drawn as its own sprite, swinging on its hook with `sway(t, seed, 0.06, 3.5)`. Its light follows the swing.
  - **Grass tufts and flowers** (decorations): rotate about their base with `sway(t, x*0.37+y*0.11, 0.07, 3.0)` (use `SpriteParams.pivot`).
  - **Day:** up to 6 butterflies near the camera (two-frame flap at about 6 Hz, wandering paths) and up to 12 drifting petals.
  - **Every 20–40 s:** a soft `bird_shadow` crosses the view at about 2.5 tiles/s, alpha ≤ 0.18, only by day.
  - **Night:** fireflies stay as they are after Task 5c.
- [ ] **Step 4:** Run `cargo test --release`, `cargo run --release --bin gen_assets` and `cargo test --release`. Expected: PASS.
- [ ] **Step 5:** The reviewer screenshots day and dusk with a working smelter, an assembler and a lamp pole. They also record a 5 s clip (`COGWORKS_SCREENSHOT` frames, or by eye) to confirm the motion is smooth and calm.
- [ ] **Step 6:** Commit: `feat: steam, swaying lanterns and grass, butterflies and bird shadows`.

### Task 9: Whole-look verification

- [ ] **Step 1:** Grep `src/` (excluding `theme.rs` and `paint.rs`) for raw hex literals `0x[0-9a-fA-F]{6}` used as colours. Expected: none new compared to `da87bfe`.
- [ ] **Step 2:** Run `cargo build --release 2>&1 | grep -c warning`. Expected: no higher than at `da87bfe`.
- [ ] **Step 3:** Run the game at 1440x900 and full 2560x1440. Take screenshots at day phases 0.5, 0.68, 0.8 and 0.95 with the help panel, inventory and research open in turn. Check against the spec's "Done when" list. Click a quickbar slot and a panel's close button to confirm hit-testing.
- [ ] **Step 4:** Fix these known issues from the reviewer's Task 4 screenshots:
  - Slot item counts (`slot()` in `src/ui.rs`) are drawn in dark ink directly over the icon and are hard to read. Put each count on a small `BG` paper badge (rounded, `radius::SM`) at the slot's bottom-right.
  - Research card unlock text (light teal) has weak contrast on cream. Use a darker verdigris token with `contrast(_, BG2) >= 4.5`, and add that assertion to `ui_text_contrast_is_readable`.
- [ ] **Step 5:** Fix anything else that fails, then commit: `chore: lantern-dusk redesign polish`.
