# Polish pass

Polish what exists on `art-overhaul`. No new systems, content or balance changes.

**Baseline (2026-11-07):** `cargo test` passes 82 + 5 tests with 1 ignored; clippy has 2 warnings.

**Rules for every phase**

- Keep saves compatible: new saved fields use `#[serde(default)]`.
- No film grain or paper noise anywhere.
- Leave the night lighting alone. A dimmer version was tried and reverted (c55e818).
- Put decisions in pure functions where practical and unit-test them, as `input.rs` does.
- Finish each phase with `cargo test` and `cargo clippy --all-targets` passing.
- Never run the game from the repo root without `COGWORKS_SCREENSHOT`: it would write over the user's saves. Use a scratch directory.

## Phase 1: Flow and save safety

1. **Esc cannot open the pause menu.** `keys()` sets `Screen::Paused`, then `menus::pause` runs in the same frame, sees the same `is_key_pressed(Escape)` and resumes. macroquad clears pressed keys only at end of frame. Add a `menu_just_opened` flag on `Game`, set in `pause_menu()` and cleared at the end of `menus::pause`; ignore Esc while it is set. Do not use `menu_fade` for this, because `update()` advances it before drawing.
2. **Closing the window loses progress.** Up to 3 minutes since the last autosave, plus any unsaved settings, because nothing uses `prevent_quit` / `is_quit_requested`. Call `prevent_quit()` at startup. On a quit request, autosave if `has_game` is true, the player is alive and the game is not in screenshot mode; save settings; then exit. Skip the world save on the title screen, where it was already saved on the way there.
3. **"Save and quit" quits even when the save fails.** Stay in the menu and show the error toast. Make `save_game` return whether it succeeded.
4. **New game destroys old autosave-only progress.** The note says saves stay "until you save over them", but the next autosave replaces the previous game. In `new_game`, rename any existing autosave to `cogworks_autosave.prev.json`, add it to `.gitignore`, and reword the note to match.
5. **A corrupt newest save is a dead end.** When the newest save fails to load at startup or on F9, try the other slot and toast which one was loaded.
6. **F5 does nothing in the pause menu**, though its hint says "F5 saves at any time". Handle it there. Make the F5 branch in `keys()` call `save_game`, removing the duplicate code.
7. **The HUD reacts underneath the pause menu.** Quickbar selection and craft cancelling still fire. Gate both on `Screen::Playing`. Correct the `pause_menu` doc comment too: it says the tool is put away, and it is not.
8. **R mutates the world while paused and out of reach.** Gate rotation on not paused and `in_reach`, and accept right Shift for counter-clockwise.
9. **F9 keeps stale state.** Reset `deconstruct`, `last_drag` and `paused` on load.
10. **Settings gaps.** Slider drags stop when the cursor leaves the track: capture on press and hold until release. Persist the look and music mute in `Settings`, applying them at startup and updating on V and M. Add a Look row to the settings page and resize the panel to fit.

## Phase 2: UI defects seen in screenshots

Capture before and after for each at 2560x1440 (`COGWORKS_FULLSCREEN=1`) and in a narrow tiled window.

1. **Menu veil stripes.** Overlapping translucent bands in `menus::veil` (`band + 1.0`) draw horizontal lines across the world and HUD. Replace them with a smooth vertex-coloured gradient.
2. **Window titles sit too high** in the moss band (`widgets.rs`, `ty = band_mid - 15`). Centre them using measured text height, with the subtitle sharing a baseline.
3. **"Workbench" overprints "Shift-click to craft five"** (`inventory.rs:73-74`). Measure the title and place the hint after it, or on its own line if there is no room, and shift the recipe grid to suit.
4. **Research grid clips its last row** above a dead strip, and scrolls even when the cards could fit. Correct the `view_h` / `want_h` maths. Extend the layout test: with enough height, `max_scroll == 0`; a clipped layout must still scroll to every card.
5. **Radio pill.** It draws over open panels, overlaps the quickbar in narrow windows, and its icon and text are misaligned. Draw it before panels, centre its contents, and collapse to icon-only when there is no room.
6. **Quickbar.** Slot numbers are nearly invisible, so give them a small key badge. The arm icon spills out of its slot (also in the workbench); make `draw_item_tint` fit the whole arm, including its reach.
7. **Craft queue label is unreadable over the world.** Put a plaque behind the queue and its label.
8. **Toasts collide with the help note**, since both start near y=140. Stack toasts below the note while it is open.
9. **Building panel.** Add spacing so "Recipe" does not touch the slots, and update `building_body_height` to match. Label or give an icon to the unlabelled HP bar.
10. **Copy.** Replace "Inserted 0 items" with a useful message such as "Nothing to insert". "Feed cores" gets the same feedback as other buttons. Replace "Carrying 1 item(s)" with correct plurals.
11. **Small windows.** Check the title and pause cards at a 1000x620 window; if they overflow, tighten the layout instead of letting them run off screen.

## Phase 3: Ending and cleanup

1. **Win moment.** Today the win is a toast plus a banner that stays on screen for the rest of the game, including every reload. Replace it with a one-time panel: title, days taken, research count and a "Keep building" button. Track it with `won_seen` (`#[serde(default)]`), and play the existing research sound. An old won save may show the panel once, which is acceptable.
2. **Clippy.** Rename `to_title` to `go_to_title` and move the test module in `assets.rs` to the end of the file, so clippy is clean.
3. **Dead code.** Remove `let _ = (border, border_w)` and the values only it uses in `surface_alpha`. If `grep` confirms the `paper` sprite is never drawn, remove it from `assets_list.rs`, `gen_assets` and `assets/sprites/`, and delete `tools/art/paper.py`.
4. **Docs.** Update the README for save-on-close, the previous-autosave file, the Look setting (V) and the win panel. Add V to the controls table. Refresh only the screenshots that visibly changed.

## Done when

- Tests and clippy pass with no warnings.
- Claude has audited each phase's diff and screenshots.
- A live keyboard run confirms: Esc opens and closes the pause menu; closing the window writes the autosave; settings survive a restart; an existing save still loads.
