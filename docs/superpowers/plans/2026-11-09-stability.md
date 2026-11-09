# Stability pass

Make Cogworks hard to crash and hard to lose progress in. No gameplay, art or UI layout changes.

**Rules**

- Keep saves compatible: every save that loads today must still load, with the same contents.
- No film grain or paper noise. Do not change the lighting.
- Put decision logic in pure functions with unit tests.
- `cargo test` and `cargo clippy --all-targets` must pass with zero warnings.
- Never run the game from the repo root without `COGWORKS_SCREENSHOT`: it would write over the user's saves. Tests must never read or write `cogworks_save.json`, `cogworks_autosave.json`, `cogworks_autosave.prev.json` or `cogworks_settings.json` in the working directory; use a temp dir (`std::env::temp_dir()` plus a unique name) for any test that touches files.
- Commit after each phase. Do not push.

## What can crash today

- `save::load` only runs serde and then `after_load`. Nothing checks the loaded data, so a hand-edited, truncated-but-valid or future/older save can panic later:
  - `research.done` shorter than `TECHS` (a save made before a tech was added) panics in `src/ui/panels/research.rs:96` and `src/world/production.rs:151`.
  - `research.current` out of range or already done.
  - `res`, `grid`, `pollution` or non-empty `trees` not `MAP_W * MAP_H` long panic on indexing.
  - A `grid` cell pointing at a missing entity panics in `entity()` ("dangling entity id"); an entity whose tiles are not marked in `grid`, or are out of bounds, breaks placement and removal.
  - NaN or infinite floats (player position, `time`, `hp`, pollution, enemy positions) spread through the simulation and the camera.
- `cogworks <file>` panics when the file won't load (`src/main.rs:79`).
- `save::save` renames the temp file without `sync_all`, so a power cut can leave an empty save. `Settings::save` uses `fs::write` directly and can be cut in half.
- A panic anywhere closes the window with no trace and loses up to 3 minutes of play.

## Phase 1: validate and repair loaded saves

1. Add `World::repair(&mut self) -> Result<Vec<String>, String>` in a new `src/world/repair.rs`, called from `save::load` after deserializing and before `after_load`. `Err` means the save can't be used (so `load_newest_or_fallback` falls back to the other slot as it already does). `Ok(notes)` lists what was fixed.
   - **Fatal:** `res`, `grid` or `pollution` with the wrong length; `trees` neither empty nor the right length (empty stays allowed, as today).
   - **Repaired:**
     - `research.done` padded with `false` or truncated to `TECHS.len()`.
     - `research.current` cleared when out of range or already done.
     - The grid rebuilt from `entities`: clear it, then mark each entity's footprint. Drop an entity whose footprint leaves the map or overlaps an earlier entity. Rebuild `free` from the `None` slots.
     - Non-finite floats: `time`, `evolution`, `pollution_timer`, pollution cells, entity `hp` and `calm` set to safe values (0, or full `max_hp` for hp); hp clamped to `0..=max_hp`. Player position non-finite: move the player to `spawn` (and `spawn` to the map centre if it is also bad). Enemies with non-finite position or hp are dropped.
   - Keep notes short and few: one note per kind of fix, with a count ("Removed 2 overlapping buildings").
2. `save::load` returns the notes too: `load(path) -> Result<(World, Vec<String>), String>`, or keep `load` returning `World` and add the notes to `LoadedSave`. Whichever is simpler; all callers updated. When notes exist, startup and F9 show one toast: "Save repaired: <first note>" (plus "and N more" when there are several).
3. `cogworks <file>` no longer panics. A load failure falls back to the normal startup path (title screen over the newest save) and toasts the error.
4. Add `save::load_from_str(json: &str)` (used by `load`) so tests can load without files.

Tests (in `src/world/tests.rs` or a `repair` test module):
- A save of a fresh world loads back unchanged (repair makes no notes).
- `research.done` with one entry removed loads, has `TECHS.len()` entries, and `tech_available` works for every tech.
- `research.current = Some(TECHS.len())` loads as `None`.
- A grid cell pointing at a `None` entity and an entity missing from the grid both load, and afterwards every grid cell points at an entity covering it and every entity's tiles point back at it.
- Two overlapping entities: one is dropped.
- NaN player position, NaN pollution cell, infinite hp: load and all become finite.
- Wrong `res` length: `Err`.
- Truncated JSON and random garbage: `Err`, no panic.

## Phase 2: safe writes and crash reports

1. Add `save::write_atomic(path: &str, bytes: &[u8]) -> Result<(), String>`: write `path.tmp`, `sync_all`, rename into place. Use it for world saves (serialize to a `Vec<u8>` first, or keep the streaming writer and call `get_ref().sync_all()` after flushing) and for `Settings::save`.
2. Panic hook, installed at the top of `main` except in screenshot mode: append to `cogworks_crash.log` a timestamp (seconds since the epoch is enough), the panic message, location and `std::backtrace::Backtrace::force_capture()`, then call the previous (default) hook. The hook must never panic itself; ignore write errors.
3. Emergency save: wrap the fixed-step `game.world.update(TICK)` loop in `std::panic::catch_unwind(AssertUnwindSafe(..))`. On a panic, save the world to `cogworks_crash_save.json` (a new constant in `save.rs`, never one of the normal slots, so a possibly broken state can't overwrite a good save), ignoring errors, then `resume_unwind`. Skip the emergency save in screenshot mode.
4. README: one short paragraph under saves explaining `cogworks_crash.log` and `cogworks_crash_save.json`, and that `cogworks cogworks_crash_save.json` loads the crash save (Phase 1 repair applies to it).
5. Add `cogworks_crash.log` and `cogworks_crash_save.json` to `.gitignore` if the other saves are listed there.

Tests:
- `write_atomic` writes the bytes, replaces an existing file, and leaves no `.tmp` behind (temp dir only).
- Settings save and load round trip through a temp path: give `Settings` `save_to(path)` and `load_from(path)` used by `save` and `load`, and test those.

## Phase 3: soak tests

1. A deterministic soak test in `src/world/tests.rs`: a small seeded RNG (the world's `Rng` or a local xorshift), `World::new(seed)` for 3 seeds, and for each about 3000 steps. Each step picks one of: `place` a random placeable `Kind` at a random tile near the player with a random `Dir` (errors are fine), `deconstruct` a random tile, `rotate` a random tile, or run 1–30 `update(TICK)` ticks. Also give the player plenty of items so placement succeeds often. Leave hives and enemies on (not peaceful) for one seed.
2. After every step check `World::check_invariants()` (a `#[cfg(test)]` method in `repair.rs` sharing logic with `repair`): grid and entities agree both ways, `free` lists exactly the `None` slots, every float in player, entities, enemies and pollution is finite, and `research.done.len() == TECHS.len()`.
3. Every 500 steps, save to a string and load it back with `load_from_str`; repair must make no notes and the invariants must hold.
4. A save-corruption test: serialize a world that has run a few hundred ticks, then 200 times either truncate it at a random byte or overwrite a random byte with a random ASCII character, and call `load_from_str`. It must return `Ok` or `Err` and never panic; when `Ok`, the invariants hold.
5. Keep the whole test suite under about 20 seconds in debug. Lower the counts if needed and mark nothing `#[ignore]`.

Any panic the soak or corruption tests find is a real bug: fix its root cause in the simulation, with a focused regression test, rather than weakening the test.
