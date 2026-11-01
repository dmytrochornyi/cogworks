# Music builder (`tools/music/build_music.py`)

Offline build tool that turns the owner's Mondo Loops sample packs into the
game's mastered lo-fi music, ambience beds and foley. It is a **tool-only**
dependency (Python 3 + numpy + ffmpeg), never part of `cargo build`.

## Licence rules (hard constraints)

- Source: both free packs were downloaded from the official page
  <https://mondoloops.com/pages/free-lofi-samples>. The licence is **personal
  and non-transferable** to the owner, so the raw packs must **never be
  committed** to git.
- Samples may be used **only inside original compositions**. This repo ships no
  raw sample:
  - every music file is a multi-layer arrangement (4 original tracks),
  - every effect / ambience file is edited (trimmed, faded, EQ'd/filtered,
    pitched or levelled, loop-seamless for ambience),
  - the packs themselves stay outside the repo (`/drive-download-*/` and
    `/samples/` are gitignored).
- Do not register anything with Content ID.

## Rebuilding

Prerequisites: `ffmpeg`/`ffprobe` with `rubberband`, `loudnorm`, `ebur128` and
`libvorbis`, plus `uv` (numpy is fetched as a tool dependency).

```sh
uv run --with numpy python tools/music/build_music.py --packs <packs-dir>
```

`<packs-dir>` is the folder containing the two extracted packs
(`Dreamy Lofi Hip Hop Samples - Vol #1 (Free Version)` and
`Organic Lofi Hip Hop Vol #1 [Free Pack]`); the `.zip` files, `__MACOSX`
folders and `.DS_Store` are ignored.

The builder writes:

- `assets/audio/music/*.ogg` — 4 tracks (2 Day, 2 Dusk), 100–140 s, seamlessly
  loopable, mastered to −16 LUFS ±1, true peak ≤ −1.0 dBTP, OGG Vorbis `-q:a 5`
  at 32 kHz stereo.
- `assets/audio/ambience/*.ogg` — day birds / night fire beds, 30–60 s,
  loop-seamless, −28 LUFS ±2.
- `assets/audio/sfx/*.ogg` — foley variants (`place`, `pickup`, `craft`,
  `research`, `click`, `shot`), trimmed to the transient, 5 ms fade-in,
  20–60 ms fade-out, DC removed, peak −3 dBFS.
- `src/audio_list.rs` — generated manifest (do not edit).

It also writes a listening preview to `/tmp/claude-1000/sound-preview.ogg`
(not committed): 20 s of each track + 10 s of each ambience + every sfx
variant, separated by 0.5 s of silence.

## Verifying without the packs

```sh
uv run --with numpy python tools/music/build_music.py --check
```

`--check` only re-measures the committed outputs under `assets/audio/` and exits
non-zero if any target is missed. It needs no packs, so it passes on a machine
that only has the repository. It prints the loudness table and the total
decoded-f32 RAM estimate (budget 160 MB).

## Composition notes

Per-stem pipeline: decode to 32 kHz stereo float → `rubberband` time-stretch
(≤ ±8 %) / pitch (≤ ±2 semitones) where needed → sample-accurate placement and
mix in numpy → lo-fi bus (light ~12 kHz high-shelf cut, soft `tanh` saturation,
≤ 0.3 % tape wobble) → two-pass `loudnorm` → true-peak limiting (for Vorbis
headroom) → OGG Vorbis `-q:a 5`.

Each track arranges 3–5 layers into intro → main → breakdown → main →
outro-into-loop, and the tail is crossfaded into the head so it loops seamlessly.
Keys and tempos are matched from the file names; only same/relative keys after
≤ 2 semitones of shift and ≤ ±8 % tempo stretch are combined. The `Kalimba Loop
65BPM` in the suggested Day pairing was dropped: its pitch-class profile
(B/C/A/G, no F#) does not sit in the D-major key of the `Floaty Lofi Piano`
anchor, and its file name carries no key to verify.
