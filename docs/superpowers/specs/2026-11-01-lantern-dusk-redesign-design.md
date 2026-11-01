# Cogworks visual redesign: "Lantern market at dusk"

Date: 2026-11-01
Status: awaiting owner approval

## Intent

Remake the look of Cogworks into a **happy, cozy, lo-fi** style. The owner's own picture is
"warm lanterns at dusk, soft pastel colours, cozy". It's inspired by the mood of hand-painted
anime fantasy films: the lantern-lit outdoor market street at nightfall (main reference), lush
forest greenery, and lovable patched-together machines.

This is **mood inspiration only**. No characters, creatures, logos or specific designs from any
film are copied.

The owner chose direction **A, "Lantern market at dusk"**, from three mockups (A lantern dusk,
B sunlit spirit forest, C patchwork meadow). They added that the days should be **bright and
sunny, a bit of B**, and turn into the lantern market every evening.

## Goals

1. **Day/night as the story of the look.**
   - Bright, sunny, lush days in the spirit of direction B: saturated soft greens, warm sunlight,
     floating glowing motes.
   - Every evening the world turns into the lantern market: lavender-blue dusk light, glowing
     red paper lanterns and warm pools of light.
   - Night is a deep lavender-blue, never pitch dark or scary.
2. **Lanterns.**
   - Lamp poles become red paper lanterns: an oval red body, dark caps, a warm glow inside.
   - Their light pools are warm orange and generous.
   - Lanterns are the visual centrepiece of the evening.
3. **Buildings.** Machines read like little market stalls and bathhouses:
   - curved **red-lacquer roofs with teal (verdigris) trim**
   - warm cream/wood walls
   - windows that glow warm when active
   **Every building sprite is restyled** (owner's decision), so the whole factory reads as one
   place. Sprite canvases equal the footprint, so roofs are painted **inside** the existing
   canvas. The silhouette may change within the canvas, but the footprint may not.
   - **Red roof + teal trim:** smelter, assembler, lab, generator, refinery, mixer, drill,
     wellhead, Grand Engine.
   - **Repainted in the same palette, no roof:** chest, wall, sentry, belts (straight, curve),
     tunnel, splitter, pipes (hub, segment), arms (base, limb, hand), solar panel, ports,
     chute, drill bit, gear overlay, wheel, paddle, beam, engine ring.
   - The lamp pole becomes the red paper lantern (goal 2).
4. **Ground and nature.**
   - Grass becomes a lush, soft, slightly pastel painted green instead of today's olive/khaki.
   - Decorations (grass tufts, pebbles, small flowers) and ores are re-tuned to sit in the new
     palette.
   - Ores must stay easy to tell apart from each other.
5. **Interface.** Panels, HUD, hotbar, tooltips, toasts, buttons and the minimap frame become:
   - **cream paper with lacquer-red trim**
   - soft rounded corners
   - dark ink text, with red and gold accents
   This replaces today's dark plum boxes.
6. **Font: M PLUS Rounded 1c** (owner's choice), replacing Nunito. It must be clearly more
   readable than today.
   - Today's `assets/fonts/Nunito.ttf` is a *variable* font. macroquad only uses its default
     instance, ExtraLight (weight 200), which is why text looks thin and faint.
   - Ship **static** M PLUS Rounded 1c files (SIL OFL), with their licence, in `assets/fonts/`.
     Body text uses Medium (500); titles and headings use Bold (700). Remove Nunito.
   - Text is rasterised at **physical** size: font size × `ui_scale()`, drawn with
     `font_scale = 1 / ui_scale()`, so it stays sharp on 1440p and 4K instead of being drawn
     small and scaled up. Measuring (`text_w`, `text_in`, etc.) must use the same scaling, so
     layouts don't shift.
   - Text sizes in `TextSize` may be nudged up by a pixel or so if M PLUS reads small, as long
     as nothing overflows its panel.
   - The font files may be subset to the glyphs the game uses (Latin + symbols such as
     `·`, `×`, `→`) to keep the binary small. That's the builder's call.
7. **Happy and calm.** Overall saturation is pastel-soft but cheerful. Nothing grimy or dark.

## Revision 2026-11-01 (owner feedback after Tasks 1–5)

The owner reviewed the running game and asked for the following. These override earlier parts of this document where they conflict.

1. **Seamless ground.** The grass grid must not be visible. Ground is one continuous surface. No square soil patches under ore.
2. **Ores that are attractive and distinct** in shape *and* colour:
   - iron: blue-grey angular chunks with silver glints
   - copper: warm orange, rounded, teal patina spots
   - coal: glossy black lumps with a faint purple sheen
   - stone: pale sandy smooth boulders
   - quartz: lilac crystal spikes
   
   Ore fields fade softly into the grass.
3. **No pixelation or jaggies.** Anti-aliasing on. UI borders drawn as smooth shapes. Smooth minimap. Sprites stay clean when zoomed out.
4. **Gentler red.**
   - The lacquer red becomes a soft dusty rose or vermilion, used as a thin accent.
   - Panel edges become warm wood-brown, with cream paper doing most of the work.
   - Machine roofs use the softer red too.
5. **Lavender dusk that actually reads as lavender.** Screenshots showed golden hour as murky olive and dusk/night as teal-green.
   - Golden hour must read as warm gold-peach.
   - Dusk must read as lavender-pink to violet.
   - Lamp and player light must read as warm peach-orange pools.
   - Fireflies must be fewer and smaller.
6. **"Boiler room" machinery.** The owner likes the boiler room from Spirited Away (mood only; copy no characters, including the soot creatures). Machines combine the soft market-stall roofs with boiler-room details:
   - brass and copper pipes, rivets, dark warm wood
   - glowing furnace mouths and windows
   - steam vents
7. **More life and gentle animation.** The world felt empty and static.
   - **Active machines bustle:** puffs of steam from vents and chimneys, furnace glow that breathes and flickers softly, turning gears and pumping pistons, an occasional spark. Idle machines are calm.
   - **The world breathes:** grass and flowers sway in the breeze, butterflies and drifting petals by day, lanterns swing gently, shadows of birds pass overhead now and then, and fireflies at night.
   - Everything is slow and smooth (periods of seconds, eased motion), never flickery like the removed grain.

## Non-goals

- No gameplay, balance, controls, layout or mechanics changes. Panel positions and sizes stay
  as they are. Only the look changes.
- No new music. The lo-fi track was just redone.
- No new building types, items or UI features.

## Anti-goals (would count as failure even if it "works")

- **Breaking what works.** A previous design pass flipped the UI upside down. The fix (y-flip of
  the UI camera in `src/ui.rs` `draw()`, needed because of how macroquad 0.4.16 handles cameras
  without render targets) must be kept. After the redesign:
  - the UI and the world must render upright
  - mouse hit-testing must match what's drawn
- **Film grain or any per-frame screen noise.** It was removed on purpose and must not come back.
- Copying recognisable characters or designs from the reference films.
- Mud: a palette where items, ores or UI text become hard to read, or where night becomes too
  dark to play.

## Constraints

- All art stays **procedurally generated** by `src/bin/gen_assets/` (`sprites.rs`, `paint.rs`).
  The generator is deterministic and its output is committed under `assets/`. Re-run with
  `cargo run --release --bin gen_assets` and commit the regenerated assets.
- `src/theme.rs` stays the **single source of truth** for colours, shared by the game and the
  generator. New colours go there as tokens. No raw hex literals elsewhere.
- Sprite canvas sizes, footprints, and the anchor points that `render.rs` relies on stay
  unchanged, so drawing, lights and animation offsets keep working. Examples of such anchors:
  `LANTERN` offset, light positions in `draw_lights`, arm/beam pivots.
- Day/night timing is driven by `sky()` in `src/render.rs`, which uses key colours over the
  day fraction. The world composite shader in `src/assets.rs` (`COMPOSITE`) does the colour
  grade and vignette. `theme::grade()` mirrors that grade for flat images such as the minimap
  and must stay in sync with it.
- The redesign starts from commit `da87bfe`, which already contains the upright-UI fix, the
  grain removal and the new music.
- Existing tests (`cargo test --release`) must keep passing. The game must build without new
  warnings.
- No new crate dependencies. New font files are assets, not dependencies.

## Decided by the owner

- Direction A (lantern market at dusk) as the core look.
- Bright sunny days with a bit of direction B, turning into the lantern market each evening.
- Font: M PLUS Rounded 1c (picked from `docs/superpowers/specs/assets/2026-11-01-font-options.html`).
- Restyle all buildings: red roofs on the house-like machines, everything else repainted to
  match (see goal 3).
- Cream paper + lacquer-red trim UI (as in mockup A). The mockups the owner picked from are
  saved at `docs/superpowers/specs/assets/2026-11-01-visual-direction-mockups.html`. They are
  rough mood sketches: match their colours and feel, not their exact shapes.

## My calls (owner can override)

- **Day/night proportions.**
  - Day is bright, but softer and more saturated-green than today's near-white midday.
  - Golden hour and dusk are stretched so the lantern phase is long.
  - Night is lavender-blue with ambient light no darker than roughly today's dusk value.
- **Lantern timing.** Lamps and windows start glowing during golden hour, not only at full night.
- **Day particles.** Floating glowing motes stay by day (today's golden dust, made a bit more
  visible and softer). Fireflies stay at night.
- **Text colour.** Ink-on-cream text needs strong contrast. Body text should be dark plum-brown
  on cream; secondary text a lighter brown, still readable.
- **Belts.** Warm dark wood/lacquer bands with brass rails, to fit the market look.
- **Item icons and player.**
  - Item icons that are near-cream (polymer, glass, lens, iron plate and similar) get re-tuned
    so they stay readable on the new cream panels.
  - Inventory and hotbar slots sit in a slightly darker, warm paper "well" so every icon has
    contrast behind it.
  - The `icon_*` HUD glyphs and the player sprite are re-tuned to the palette. Their shapes
    don't change.
- **Enemies.** Keep their colours readable as threats against the new ground. Their shapes
  don't change.

## Left to the builder

- Exact hex values and ramp structure in `theme.rs`.
- Per-sprite restyling details: roof shapes, trim placement, window glow shapes. These must
  stay within the "market stall / bathhouse, red roof + teal trim" language.
- Exact `sky()` key colours and timings, and composite grade numbers.
- How UI panel trim is drawn (border thickness, inner paper tint, shadow).

## Done when

- The game runs and shows a full day: sunny lush day → golden hour → lantern-lit dusk →
  lavender night → dawn.
- Lamp poles are red paper lanterns that glow at dusk and night.
- All panels (HUD, hotbar, inventory, research, building panels, tooltips, toasts, help, radio
  widget, minimap frame) use the cream-paper/red-trim look, are upright, and respond to clicks
  in the right places.
- All text uses M PLUS Rounded 1c, looks solid (not hairline) and sharp at 1440p.
- No grain or per-frame noise anywhere.
- Tests pass. Assets are regenerated and committed.
