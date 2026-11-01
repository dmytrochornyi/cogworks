# Bug Fixes, Polish and Module Split Implementation Plan

> **For agentic workers:** Tasks are delegated to Reasonix one at a time (delegate-reasonix skill) and audited before the next starts. Steps use checkbox (`- [ ]`) syntax.

**Goal:**
- Fix the owner's four reported bugs and the eight bugs confirmed by the code review.
- Add a cozy custom cursor, a readable recipe card and a real walk animation.
- Split the oversized files into focused modules.

Gameplay rules stay the same, except where a bug fix changes a broken rule.

**Architecture:** Behaviour-preserving module splits come first (Tasks 1–4), so every fix lands in a small, focused file. Each fix starts with a failing test. Pure functions carry the logic (gestures, poses, cursor state, recipe-card rows), and drawing code stays thin.

**Tech Stack:** Rust 2024, macroquad 0.4.16. No new crates.

**Sources:**
- The owner's report (2026-11-01):
  - "arm is put on the ground only in one position without rotation"
  - "robot doesn't have animation while going somewhere"
  - "E inventory: when I select an item it puts it on the ground instantly"
  - "workbench crafting pop-up is plain text and not friendly to read"
  - "make a custom lofi cozy cursor"
  - "good quality code, modular"
- The Reasonix review report (findings R1–R8 below), each spot-checked by the reviewer.

## Root causes (established before planning)

- **Inventory click places the item (owner bug 3).**
  - `ui.rs` `inventory_panel` sets `g.tool` and closes the panel on click.
  - On the next-but-one frame, `main.rs` `mouse()` sees the button still held, `mouse_over_ui()` is false (the panel's rect is gone), and `last_drag` is `None`, so `paint()` places the building immediately.
- **Arm "one position" (owner bug 1).**
  - Mostly the same cause: the arm lands before R can be pressed.
  - Also, the placement ghost draws the limb at `angle(dir) − π/2` (`render.rs` `draw_kind`), while a placed arm idles at `(−dir).to_angle()` (`draw_entity`). The two never match.
  - `draw_ghost` skips the flow arrow for `Kind::Arm`.
- **No walk animation (owner bug 2).**
  - `draw_player` alternates `player_0`/`player_1`, which differ only by a 1.5 px bob (under 1 screen pixel at default zoom).
  - There's no lean, no steps, and no facing beyond a horizontal flip.
- **Plain tooltip (owner bug 4).** `g.tooltip` is `Vec<String>`. `recipe_lines()` formats everything as strings, and `draw_tooltip()` prints them as lines.

## Global Constraints

- No new crates. No raw hex colours outside `src/theme.rs`. No new warnings. `cargo test --release` green after every task.
- Save files stay compatible: no change to serialized types or field names unless a task says so.
- Keep `ui_camera` (Y-flip), `raster_params`, `font_for`, `line_metrics`, the sprite-size baseline and the audio module unchanged unless a task says otherwise.
- Module splits (Tasks 1–4) are **pure moves**: no behaviour change, no renames of public items beyond what the move needs. They're verified by tests plus a screenshot diff (see Task 1, Step 3).
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **A click that starts on any UI element** (panel, help note, hotbar, minimap, tooltip, radio) must never build, deconstruct or open a building, even if the UI disappears while the button is still held. Pinned by `gesture_started_on_ui_never_builds` (Task 5).
2. **The ghost must show exactly what will be placed.** Pinned by `arm_ghost_matches_placed_arm` (Task 6).
3. **Walking animates by distance travelled, not by wall time**, so feet don't slide at different speeds. Pinned by `walk_frame_follows_distance` (Task 7).
4. **Recipe card states:** enough of an input, short but auto-craftable, and truly missing must look different. Pinned by `recipe_card_marks_each_input` (Task 8).
5. **No item or fluid silently destroyed** by deconstructing, re-recipeing or research changes. Pinned by the Task 10 tests.

---

### Task 1: Split `main.rs` into game, input and cursor modules

**Files:**
- Create: `src/game.rs` (`Game` struct, `new`, `update`, `play_sounds`, `toast`, `Panel` enum), `src/input.rs` (`impl Game { input, keys, mouse, paint, try_place, select_slot, ghost_pos, mouse_world, mouse_tile, mouse_over_ui }`), `src/cursor.rs` (move the existing in-world cursor drawing: placement ring and mining ring).
- Modify: `src/main.rs` keeps only `mod` declarations, `conf()`, `main()` and the dev aids (`COGWORKS_SCREENSHOT`, `COGWORKS_DAY_PHASE`, `COGWORKS_HELP`, `COGWORKS_PANEL`).

- [ ] **Step 1:** Before moving anything, capture baseline screenshots into `/tmp/claude-1000/baseline/`: the demo save (`DEMO_SAVE=/tmp/claude-1000/demo.json cargo test --release -- --ignored write_demo_save`) at `COGWORKS_DAY_PHASE=0.5` and `0.8`, plus `COGWORKS_PANEL=inventory` and `research`. If the sandbox can't open a window, skip this and say so; the reviewer captures them.
- [ ] **Step 2:** Move the code. `crate::Game` and `crate::Panel` paths used by `ui.rs` keep working, via `pub use game::{Game, Panel};` in `main.rs`.
- [ ] **Step 3:** `cargo test --release` passes and `cargo build --release` has no warnings. The reviewer compares the screenshots: the mean absolute pixel difference must be < 2/255, allowing for particles.
- [ ] **Step 4:** Commit: `refactor: split main.rs into game, input and cursor modules`.

### Task 2: Split `ui.rs` into `ui/`

**Files:** `src/ui.rs` becomes:
- `src/ui/mod.rs`: `draw()` and re-exports
- `src/ui/metrics.rs`: `ui_scale`, `ui_size`, `ui_width`, `ui_height`, `ui_mouse`, `ui_camera`, `clip`, `unclip`
- `src/ui/text.rs`: `raster_params`, `font_for`, `line_metrics`, `text*`
- `src/ui/primitives.rs`: `rrect*`, `circle*`, `hovered`, `clicked`, `surface`, `Elevation`
- `src/ui/widgets.rs`: `bar`, buttons, `slot`, `Panel` (the window builder), `recipe_icon`, `stock_lines`
- `src/ui/hud.rs`: `hud`, `quickbar`, `craft_queue`, `radio`, `toasts`, `help`, `overlays`
- `src/ui/tooltip.rs`: `draw_tooltip`, `recipe_lines`
- `src/ui/minimap.rs`: `Minimap`
- `src/ui/panels/{inventory,research,building}.rs`

Tests move with the code they test.

- [ ] Pure move, then tests, then the reviewer's screenshot diff, as in Task 1. Commit: `refactor: split ui.rs into focused modules`.

### Task 3: Split `render.rs` into `render/`

**Files:**
- `src/render/mod.rs`: `Renderer`, `frame`, re-exports
- `src/render/camera.rs`: `Cam`
- `src/render/sky.rs`: `Sky`, `sky`, `sky_at_phase`, key tables, `composite_rgb`
- `src/render/particles.rs`: `PKind`, `Particle`, `spawn_particles`, `advance_particles`, the caps, `sway`, particle drawing
- `src/render/ground.rs`: `draw_ground`, decorations
- `src/render/entities.rs`: `draw_kind`, `draw_entity`, belts and items, pipes, arms, lanterns, `VENTS`
- `src/render/actors.rs`: `draw_player`, enemies
- `src/render/lights.rs`: `draw_lights`, `light`
- `src/render/overlays.rs`: `draw_ghost`, wires, clouds, pollution

Tests move with their code.

- [ ] Pure move, then tests, then the screenshot diff. Commit: `refactor: split render.rs into focused modules`.

### Task 4: Split `world/mod.rs`

**Files:**
- Create:
  - `src/world/generate.rs`: `World::new`, `generate`, `blob`, `Rng`
  - `src/world/entity.rs`: `spawn_entity`, `delete`, `place`, `deconstruct`, `contents`, `check_place`, `footprint_free`, `rotate`, `set_dir`, `mark_dirty`, entity accessors
- `src/world/mod.rs` keeps the types, `update` and `describe`.

- [ ] **Step 1:** Pure move. `save_round_trip` and all world tests pass. Also load the demo save written before the move, to prove save compatibility. Commit: `refactor: split world generation and entity management out of world/mod.rs`.

### Task 5: Clicks that start on UI never reach the world (owner bugs 1 and 3, review R8)

**Files:** `src/input.rs`, `src/ui/hud.rs` (`help` pushes its rect to `g.ui_rects`).

**Interfaces:**
- `pub enum Gesture { Idle, World, Ui }` and `pub fn next_gesture(g: Gesture, pressed: bool, down: bool, over_ui: bool) -> Gesture` in `src/input.rs`:
  - A press over UI → `Ui`. A press elsewhere → `World`. Release → `Idle`. Otherwise unchanged.
- Building, opening buildings, drag-painting, deconstructing and mining happen only while the gesture is `World`. A gesture that began on UI stays `Ui` until release, even if the UI vanishes.

- [ ] **Step 1:** Failing tests:
  - `gesture_started_on_ui_never_builds`: press over UI, then 10 frames held with `over_ui = false` → still `Ui`.
  - `gesture_in_world_survives_crossing_ui`: press in world, then the drag passes over the hotbar → still `World`.
  - `release_resets_gesture`.
  - `help_note_blocks_clicks`: build a `Game`-free check that `help()`'s rect is registered. If that needs a window, test the extracted `help_rect(ui_w, ui_h) -> Rect` and assert that `draw` pushes it.
- [ ] **Step 2:** Implement and wire it into `mouse()`, for both left and right buttons. Keep the `last_drag` logic.
- [ ] **Step 3:** `cargo test --release`. The reviewer runs the game: open E, click the Arm, and it must only become the held tool. Press R, and the ghost rotates. Click on the map, and it's placed with that rotation. Commit: `fix: clicks that start on UI never build or open things`.

### Task 6: Arm ghost matches the placed arm, with a flow arrow (owner bug 1)

**Files:** `src/render/entities.rs`, `src/render/overlays.rs`.

**Interfaces:** `pub fn arm_idle_angle(dir: Dir) -> f32` (pointing at the pickup side, = `(-dir.vec()).to_angle()`), used by both `draw_kind` (ghost) and `draw_entity`.

- [ ] **Step 1:** Failing test `arm_ghost_matches_placed_arm`: for every `Dir`, the angle used by the ghost path equals `arm_idle_angle(dir)`. Extract `fn arm_limb_angle(state: Option<f32 /*swing*/>, dir) -> f32` so both paths call it.
- [ ] **Step 2:** Implement. Remove `Kind::Arm` from the arrow-exclusion list in `draw_ghost`, so the arm ghost shows a small flow arrow from pickup to drop. Commit: `fix: arm placement ghost shows the real orientation and flow`.

### Task 7: A real walk animation (owner bug 2)

**Files:** `src/render/actors.rs`, `src/bin/gen_assets/sprites.rs` and `main.rs` (new frames), `src/sprite_sizes.txt` (append only), `src/world/player.rs` (add `pub stride: f32`, distance walked, `#[serde(default)]` so old saves load).

**Interfaces:**
- `pub struct Pose { pub frame: usize, pub bob: f32, pub lean: f32, pub squash: f32, pub face_left: bool }`
- `pub fn walk_pose(stride: f32, vel: Vec2, t: f32) -> Pose`
- Sprites: `player_walk_0..3` (64x64), little feet or wheels alternating plus an arm swing, with the same body and scarf as `player_0`. Keep `player_0`/`player_1` for idle, as a breathing animation.

- [ ] **Step 1:** Failing tests:
  - `walk_frame_follows_distance`: the frame advances once per 0.35 tiles of `stride`, independent of `t`.
  - `walk_bob_is_visible_but_gentle`: bob amplitude is between 0.04 and 0.08 tiles.
  - `walk_leans_into_motion`: the sign of lean follows `vel.x`, with |lean| ≤ 0.12 rad.
  - `idle_breathes_slowly`: with zero velocity, the bob period is ≥ 2 s and amplitude ≤ 0.02.
- [ ] **Step 2:** Implement. The player accumulates `stride` while moving. The renderer applies bob (vertical offset), lean (rotation about the feet), a small squash on footfalls, and `face_left`. A small soft dust puff every second footfall uses the existing `steam` sprite tinted with `DIRT` at low alpha, counted under the steam cap.
- [ ] **Step 3:** Regenerate the assets, run the tests and commit: `feat: robot walk cycle with bob, lean and footsteps`.

### Task 8: Readable recipe card (owner bug 4)

**Files:** `src/ui/tooltip.rs`, the callers in `src/ui/panels/*` and `src/ui/hud.rs`, `src/game.rs` (the `tooltip` field type).

**Interfaces:**
- `pub enum Tip { Lines(Vec<String>), Recipe(usize) }`. `Game.tooltip: Option<Tip>` replaces `Vec<String>`. `world.describe()` results wrap in `Tip::Lines`.
- `pub enum InputState { Enough, AutoCraft, Missing }` and `pub struct CardRow { item: Option<Item>, fluid: Option<Fluid>, need: u32, have: u32, state: InputState }`.
- `pub fn recipe_card(world: &World, recipe: usize) -> (Vec<CardRow>, Vec<(OutputKind, u32)>, f32 /*time*/)`, which is pure. `AutoCraft` means short, but the workbench can craft the missing parts from what's in the backpack (reuse the existing auto-craft logic; don't duplicate it).

Card layout:
- a header with the output item's icon and name in `TITLE`
- one row per input: icon, name, then a right-aligned `have / need` in `TEXT` (Enough), `KEY` gold with a small "auto" chip (AutoCraft) or `TERRA_600` (Missing)
- a thin `WOOD_TRIM` divider
- an output row: "Makes" with the output icons and counts, and a small clock glyph with the time
- a footer of key chips: `Click` → craft 1, `Shift` + `Click` → craft 5

Paper surface, M PLUS text, sizes from `TextSize`, spacing from `space::*`. The card never goes off-screen.

- [ ] **Step 1:** Failing tests: `recipe_card_marks_each_input` (Enough, AutoCraft and Missing on a constructed world) and `recipe_card_lists_fluids_and_outputs`.
- [ ] **Step 2:** Implement and switch all tooltip callers. `Tip::Lines` keeps today's look, but gets a title row and the new spacing.
- [ ] **Step 3:** Commit: `feat: recipe card tooltip with icons and have/need colours`.

### Task 9: Cozy custom cursor (owner request)

**Files:** `src/cursor.rs`, `src/bin/gen_assets/sprites.rs` and `main.rs` (cursor sprites), `src/sprite_sizes.txt` (append), `src/main.rs` (`show_mouse(false)` after the window opens).

**Interfaces:**
- `pub enum CursorKind { Point, Hand, Build, Mine }` and `pub fn cursor_kind(over_clickable_ui: bool, tool: bool, right_held_on_minable: bool) -> CursorKind`.
- Sprites, each 48x48 with a soft shadow:
  - `cursor_point`: a rounded arrow in cream paper with a wood-brown outline and a dusty-rose tip
  - `cursor_hand`: a soft mitten-like pointing hand in the same style
  - `cursor_build`: the point cursor plus a small brass plus badge
  - `cursor_mine`: a tiny wooden-handled pick
  
  Each has a hotspot const in `cursor.rs`.
- The cursor is drawn last, after tooltips, in the UI camera at `ui_mouse()`, at 32 logical px (so it scales with `ui_scale`). There's a gentle 0.9→1.0 press squash while the left button is down.
- Clickable UI is known via a new `g.clickables: Vec<Rect>` that `slot`, `button_kind` and `recipe_icon` push into, cleared each frame like `ui_rects`.
- The cursor is not drawn in `COGWORKS_SCREENSHOT` mode, so screenshot diffs stay stable.

- [ ] **Step 1:** Failing test `cursor_kind_priorities`: Mine beats Build, Build beats Hand, Hand beats Point, with each case asserted.
- [ ] **Step 2:** Implement, regenerate the assets and commit: `feat: cozy custom cursor`.

### Task 10: Fix the confirmed simulation bugs (review R1–R7)

**Files:** `src/world/logistics.rs`, `fluids.rs`, `production.rs`, `entity.rs`, `src/input.rs`.

Each fix starts with its failing test in `src/world/tests.rs` (or `input.rs`):

- [ ] **R1** `deconstructing_tunnel_exit_refunds_underground_items`: picking up an exit refunds the entrance's in-flight items (`p > 1`) to the player instead of dropping them.
- [ ] **R2** `deleting_pipe_keeps_network_fluid`: `delete(pipe)` removes the id from its `FluidNet.pipes` (or `sync_pipes` skips dead ids and redistributes over live ones). Total fluid minus the removed pipe's own share is conserved.
- [ ] **R3** `changing_recipe_keeps_matching_fluid`: `set_recipe` keeps `fluid_in`/`fluid_out` when the new recipe uses the same fluid. Otherwise the fluid is discarded with a world event toast: "Emptied N Tar from Refinery".
- [ ] **R4** `lab_returns_vials_when_research_stops`: when the current research completes elsewhere or changes, a lab mid-unit puts the consumed vials back into its input and resets its progress.
- [ ] **R5** `tunnel_pairs_in_either_order`: placing the exit first, then the entrance, pairs them.
- [ ] **R6** `pipes_refuse_to_mix_fluids`: `check_place` rejects a pipe whose neighbours belong to networks of different fluids: "Can't mix Tar and Naphtha".
- [ ] **R7** `paused_world_ignores_build_and_deconstruct`: while paused, no build, deconstruct or mining progress. UI stays usable, and a toast "Paused" appears once per attempted action.

Commit: `fix: no silent item or fluid loss; tunnels pair either way; pause freezes actions`.

### Task 11: Final verification

- [ ] **Step 1:** `cargo test --release` and `cargo build --release` (0 warnings). `cargo clippy --release` shows no new warnings compared to the start of this plan.
- [ ] **Step 2 (reviewer):** Play-test with screenshots:
  1. Open the inventory and pick an arm. Nothing is placed.
  2. Rotate with R, then place. The orientation is correct.
  3. Walk in all four directions.
  4. Hover a workbench recipe. The card shows all three input states.
  5. The cursor states (point, hand, build, mine) appear in the right places.
  6. Load an old save (`cogworks_save.json` from before this plan, or the demo save). It loads.
- [ ] **Step 2b (from the Task 7 audit):** the idle frames `player_0`/`player_1` have no feet, while the walk frames do, so the robot seems to grow legs when it starts moving. Give the idle frames the same two feet in a standing pose, and keep the breathing bob.
- [ ] **Step 3:** Report file sizes after the split. No source file over 600 lines, except generated `assets_list.rs` and `sprites.rs` (data-heavy, may stay).
