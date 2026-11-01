# Sound System Implementation Plan

> **For agentic workers:** Tasks are delegated to Reasonix one at a time (delegate-reasonix skill) and audited before the next starts. Steps use checkbox (`- [ ]`) syntax. Work happens on branch `sound-system` in the worktree `/home/trutix/Documents/Rust/cogworks-sound`. The lantern-dusk redesign runs on `master` at the same time.

**Goal:** Replace the synthesized music and sound effects with professionally arranged lo-fi music and foley built from the owner's licensed Mondo Loops sample packs:
- day/dusk-aware music with crossfades
- ambience beds (birds by day, fire at night)
- varied foley sound effects
- separate music, ambience and effects levels

**Architecture:**
- **Offline:** a Python + ffmpeg build tool (`tools/music/build_music.py`, run with `uv run --with numpy`) reads the raw packs from a gitignored folder. It arranges **original compositions** from them and masters them to target loudness. It writes OGG Vorbis files to `assets/audio/` and a generated `src/audio_list.rs`.
- **In game:** a new module `src/audio.rs` owns all playback:
  - the music director (mood, playlist, crossfades)
  - the ambience beds
  - the effects with variants
  
  `main.rs` only calls into it.

**Tech Stack:** Rust 2024, macroquad 0.4.16 audio (quad-snd: WAV and OGG Vorbis), Python 3 with numpy (via `uv run --with numpy`, a tool-only dependency), ffmpeg with `rubberband`, `loudnorm`, `ebur128` and `libvorbis`.

## Licence rules (Mondo Loops terms; hard constraints)

- Source: the owner downloaded both free packs from the official page https://mondoloops.com/pages/free-lofi-samples (licence is personal and non-transferable). Record this URL in `tools/music/README.md`.

- Samples may be used **only inside original compositions**. Never ship or commit a raw sample as-is.
  - Every shipped music file is a multi-layer arrangement.
  - Every shipped effect or ambience file is edited: trimmed, faded, filtered or EQ'd, pitched or levelled, and for ambience made loop-seamless.
- The raw packs never enter git. Add `/drive-download-*/` and `/samples/` to `.gitignore`.
- Don't register anything with Content ID.

## Global Constraints

- **No gameplay changes.**
- **New crate dependencies:** none. numpy is a build-tool dependency, invoked via `uv run --with numpy`, never needed by `cargo build`.
- **Builds:** `cargo build --release` must work without the raw packs present. Only the generated outputs are needed.
- **Formats:**
  - music and ambience: OGG Vorbis, stereo, **32 kHz**, quality `-q:a 5`
  - effects: OGG Vorbis or 16-bit WAV, 32 kHz or 44.1 kHz
- **Memory budget:** quad-snd decodes every sound to f32 in RAM. The total decoded audio must stay ≤ 160 MB, i.e. about 10 minutes of stereo 32 kHz in all. The builder prints the total and fails if it's over.
- **Loudness targets:** the builder measures with `ebur128` and fails if they're missed.
  - music: −16 LUFS integrated ±1, true peak ≤ −1.0 dBTP
  - ambience: −28 LUFS ±2
  - effects: peak-normalised to −3 dBFS, then levelled by ear in the mix constants
- **File names:** keep the existing effect names `place`, `pickup`, `craft`, `research`, `shot`, `click`, so gameplay call sites don't change. Variants are `<name>_0`, `<name>_1`, … .
- **Other code:** don't touch `src/ui.rs`, `src/theme.rs`, `src/render.rs` or the sprite generator. The redesign is editing them on `master`.
- **Commits:** messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **The game starts or builds on a machine without the sample packs.** Expected: builds and plays from the committed outputs. Pinned by Task 1 Step 6 (build with the pack folder renamed away).
2. **A track ends while the mood flips.** Expected: one crossfade, never two tracks stacking or silence. Pinned by `director_never_plays_more_than_two_tracks` (Task 2).
3. **The mood flickers at the day/dusk boundary.** Expected: hysteresis, not ping-pong. Pinned by `mood_has_hysteresis` (Task 2).
4. **Muting and unmuting with M mid-crossfade.** Expected: silence, then a clean resume at the right gains. Pinned by `mute_scales_all_music_gains` (Task 2).
5. **Rapid repeated effects (dragging belts, sentries firing).** Expected: variant rotation, no identical machine-gun repeats, and the existing shot cooldown kept. Pinned by `sfx_variants_rotate_without_immediate_repeat` (Task 2).

---

### Task 1: Worktree, music builder and mastered outputs

**Files:**
- Create: `tools/music/build_music.py` (stdlib + numpy; shells out to `ffmpeg`/`ffprobe`), `tools/music/README.md` (how to rebuild, licence rules).
- Modify: `.gitignore`.
- Generate: `assets/audio/music/*.ogg`, `assets/audio/ambience/*.ogg`, `assets/audio/sfx/*.ogg`, `src/audio_list.rs`.

**Interfaces:**
- Produces `src/audio_list.rs` (generated, with a do-not-edit header):
  ```rust
  pub enum Mood { Day, Dusk }
  pub struct Track { pub name: &'static str, pub mood: Mood, pub secs: f32, pub bytes: &'static [u8] }
  pub const MUSIC: &[Track];
  pub const AMBIENCE_DAY: &[u8];
  pub const AMBIENCE_NIGHT: &[u8];
  pub const SFX: &[(&str, &[&[u8]])]; // (name, variants)
  ```
  `Mood` derives `Clone, Copy, PartialEq, Eq, Debug`.
- CLI: `uv run --with numpy python tools/music/build_music.py --packs <dir> [--check]`. `--check` only re-measures the committed outputs and exits non-zero if any target is missed.

- [ ] **Step 1:** Create the worktree: `git worktree add /home/trutix/Documents/Rust/cogworks-sound -b sound-system`. All later work happens there.
- [ ] **Step 2:** Write the builder. The pipeline per stem:
  1. decode with ffmpeg to 32 kHz stereo float
  2. time-stretch and pitch with ffmpeg `rubberband` where needed (stretch ≤ ±8%, pitch ≤ ±2 semitones)
  3. place stems sample-accurately with numpy
  4. mix
  5. gentle lo-fi bus (light high-shelf cut around 12 kHz, soft-knee saturation `tanh`, subtle tape wobble ≤ 0.3%)
  6. `loudnorm` two-pass to target
  7. encode OGG
- [ ] **Step 3:** Compose **4 original tracks**, each 100–140 s, seamlessly loopable (the tail crossfades into the head):
  - **2 Day tracks:** bright, major-leaning.
  - **2 Dusk tracks:** warm, minor or e-piano, drone.
  
  Each track has 3–5 layers (keys or guitar lead, drums from loops and/or programmed one-shots, texture, optional vocal or drone), plus an arrangement: intro → main → breakdown → main → outro-into-loop. Layers come in and out over time; never the same 4 bars looping unchanged for the whole track.
  
  Only combine loops in the same or relative key after ≤ 2 semitones of shift, and ≤ ±8% tempo stretch. Suggested pairings, to check against file names:
  - Day: "Floaty Lofi Piano (70 BPM) D Maj" + shaker 70 BPM + hats + Dusty Kick/Wood Snare one-shots; "Kalimba Loop 65BPM" accents.
  - Day: "Wobbly Piano [Bb Major] 70" + "Electric Layers Lofi Guitar C Maj" (pitch −2) or "Bright Guitar [D Minor]".
  - Dusk: "Dreamy Lofi Guitar (60BPM) C#m" + "Dreamy Electric 1 (60 BPM) E Maj" (relative major) + brushes 60 BPM + "Vocal Drone".
  - Dusk: "Living Room Piano [F Minor] 58" + "Choppy Vox [F Min]" + "Dark Textures" 60 BPM + "Warm Lofi E Piano C Minor" (key check needed).
  
  Name tracks evocatively (e.g. `day_meadow`, `day_workshop`, `dusk_lanterns`, `dusk_embers`).
- [ ] **Step 4:** Ambience. Day bed from "BirdSong" (or "City Birds Ambiance" with traffic filtered out by a high-pass and EQ). Night bed from "Studying By The Fire Ambiance". Each 30–60 s, loop-seamless, at −28 LUFS.
- [ ] **Step 5:** Effects, 2–3 variants each, edited:
  - `place`: wood snare / brick tap, short and low-passed
  - `pickup`: shell foley
  - `craft`: glass perc
  - `research`: a kalimba or glass flourish with a short swell, ≤ 1.6 s
  - `click`: stick foley, very short
  - `shot`: crunch perc / rim, tight
  
  Each one is trimmed to its transient, with 5 ms fade-in and 20–60 ms fade-out, and DC removed.
- [ ] **Step 6:** Run the builder, then `--check`. Expected: all loudness and memory targets pass, and the total decoded-RAM estimate is printed. Then temporarily rename the pack folder and run `--check` again. It must still pass, because it uses committed outputs only. Rename it back.
- [ ] **Step 7:** Commit `.gitignore`, `tools/music/`, `assets/audio/{music,ambience,sfx}/` and `src/audio_list.rs`: `feat: mastered lo-fi music, ambience and foley from Mondo Loops packs`. Also produce a listening preview: `ffmpeg` concatenating 20 s of each track and ambience into `/tmp/claude-1000/sound-preview.ogg` (not committed).

### Task 2: Audio engine module

**Files:**
- Create: `src/audio.rs`.
- Modify:
  - `src/main.rs`: `mod audio; mod audio_list;`. Replace the `music_on`/`MUSIC_VOLUME`/`play_sounds` sound playback with the `audio::Audio` calls. Keep the shot cooldown logic.
  - `src/assets.rs`: stop loading the old `SOUNDS`.
  - `src/bin/gen_assets/main.rs` and `src/bin/gen_assets/audio.rs`: stop generating music and effects, and delete the synth code. `assets_list.rs` keeps only `SPRITES`.
- Delete: `assets/audio/*.wav` (the old synth outputs).

**Interfaces:**
- Produces in `src/audio.rs`:
  ```rust
  pub struct Levels { pub music: f32, pub ambience: f32, pub sfx: f32 } // defaults 0.55, 0.35, 0.8
  pub struct Audio { /* loaded Sounds, director, levels, muted */ }
  impl Audio {
      pub async fn load() -> Audio;
      pub fn update(&mut self, dt: f32, night: f32); // drives director, crossfades, ambience mix
      pub fn sfx(&mut self, name: &str, volume: f32); // picks a variant
      pub fn toggle_music(&mut self) -> bool;        // returns new on-state, for the radio widget
      pub fn music_on(&self) -> bool;
  }
  // Pure, testable cores:
  pub fn equal_power(t: f32) -> (f32, f32);                  // (out_gain, in_gain), t in 0..=1
  pub fn next_mood(night: f32, current: Mood) -> Mood;      // Dusk when night >= 0.45, Day when night <= 0.30, else unchanged
  pub struct Director { /* current, previous, fade, elapsed, playlist indices */ }
  impl Director {
      pub fn new(start: Mood) -> Self;
      pub fn step(&mut self, dt: f32, night: f32, tracks: &[(Mood, f32)]) -> Vec<(usize, f32)>; // (track index, gain 0..1) currently audible
  }
  pub struct VariantPicker { /* last index per name, xorshift state */ }
  impl VariantPicker { pub fn pick(&mut self, name: &str, count: usize) -> usize; }
  ```
- Behaviour:
  - **Crossfade:** 6 s equal-power.
  - **Starting the next track:** begins 6 s before the current one ends, picking the next track of the current mood in rotation.
  - **Mood change:** crossfades immediately to a track of the new mood.
  - **Ambience gains:** day bed = `1 - night`, night bed = `night`, both smoothed.
  - **Track gains:** gains from `step` are applied with `set_sound_volume`. A track that gets gain 0 after its fade is stopped with `stop_sound`, and a track starts with `play_sound` (not looped; the director re-triggers).
- `ui.rs` reads `g.music_on`. Keep a `pub music_on: bool` on `Game`, mirrored from `audio.music_on()` each frame, so `ui.rs` needs no change.

- [ ] **Step 1:** Write the failing tests in `src/audio.rs`:
  - `equal_power_is_constant_power`: for t in 0..=1 step 0.05, `out² + in² ≈ 1` (±1e-4), `equal_power(0) == (1,0)` and `equal_power(1) == (0,1)`.
  - `mood_has_hysteresis`:
    - `next_mood(0.40, Day) == Day`
    - `next_mood(0.45, Day) == Dusk`
    - `next_mood(0.35, Dusk) == Dusk`
    - `next_mood(0.30, Dusk) == Day`
  - `director_never_plays_more_than_two_tracks`: with tracks `[(Day,30.0),(Day,30.0),(Dusk,30.0),(Dusk,30.0)]`, step with dt=0.1 for 600 s while `night` follows a sine between 0 and 1 with period 120 s. At every step, at most 2 entries have gain > 0, all gains are in 0..=1, and when two are audible, `gainA² + gainB² ≈ 1` (±0.02).
  - `director_never_goes_silent`: in the same run, at every step at least one gain is ≥ 0.7, except during a crossfade, where the equal-power sum must still hold.
  - `mute_scales_all_music_gains`: the engine's pure gain mapping `fn music_gain(director_gain: f32, levels: &Levels, muted: bool) -> f32` returns 0 when muted and `director_gain * levels.music` otherwise.
  - `sfx_variants_rotate_without_immediate_repeat`: for count 3, 300 picks never give the same index twice in a row, and every index appears. For count 1, it always returns 0.
- [ ] **Step 2:** Run `cargo test --release audio`. Expected: FAIL (not defined).
- [ ] **Step 3:** Implement `src/audio.rs` and wire it into `main.rs`:
  - load the `audio_list` sounds
  - call `audio.update(dt, sky(world.time).night)` every frame
  - call `audio.sfx(name, volume)` where `play_sounds` played before
  - M calls `toggle_music`
  
  Then remove the old synth sound pipeline as listed under Files.
- [ ] **Step 4:** Run `cargo test --release` (all pass) and `cargo build --release` (no new warnings). Then run `cargo run --release --bin gen_assets`. It must still produce sprites and an `assets_list.rs` with only `SPRITES`, and the size baseline test must still pass.
- [ ] **Step 5:** Run the game for 10 s (`timeout 10 ./target/release/cogworks`) and confirm no panic in the output. Report startup time and RSS (`/usr/bin/time -v`). RSS must be below 400 MB.
- [ ] **Step 6:** Commit: `feat: day/dusk music director, ambience beds and foley effects`.

### Task 3: Merge

- [ ] **Step 1** (done by the reviewer, not Reasonix): after the owner has listened to the preview and approved, merge `sound-system` into `master` once the redesign phase in progress has committed. On conflicts in the generated `src/assets_list.rs`, re-run `cargo run --release --bin gen_assets` on the merged tree. Then `cargo test --release`, a play test, and remove the worktree.
