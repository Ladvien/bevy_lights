# Flames: the billboarded shader fire

Every fire in a scene needs to *burn*, not glow. A sprite sheet of a flame is the cheap answer and it
shows: the same eight frames on forty torches, all in step. The answer here is one shader, one
material, one draw per flame, with the two numbers that make two fires different — its clock phase
and its height — riding on the mesh's tag.

## Sources

| source | what it is | use |
|---|---|---|
| Sanctuary decision 0228 | one teardrop distorted by scrolling fbm, replacing a five-column tile of tongues | the silhouette |
| Sanctuary report 1792016494 | "Torches and braziers need a shader flame of some sort. Right now they just glow." | why it exists |
| Nguyen, *Fire in the Vulcan Demo*, **GPU Gems** ch. 6 §6.1 (corpus stem `GPU Gems … Anna's Archive`, indexed 2026-10-26) | the 2D-distortion flame, "perfect for creating candle-like flames" | the shape; see `flame-silhouette.md` — the chapter's verdict is candle-*scale*, and it does not document this shader's noise |
| `book_real_time_rendering_4th_akenine` §6.3 | turbulence (the octave sum over noise) | the fbm |
| Sanctuary `assets/shaders/flame.wesl` | the shader this crate ships, comments retargeted | the code |
| home-still `s2007-advances-pdf-slides-2-8-mb` | Mitchell, Francke & Eng 2007, *Illustrative rendering in Team Fortress 2* | the warm ramp the flame sits in |

The noise's own provenance is **UNVERIFIED**: the 2D simplex noise and the integer gradient hash are
web sources that home-still cannot index, and the note at the top of `src/flame.wesl` says so. The
formula is the classic Perlin/Gustavson simplex, and Jarzynski & Olano 2020 (JCGT, *Hash functions
for GPU rendering*) is the reference the integer hash follows, but whether the constants are theirs
is not established.

## Craft principle

A flame on film is three things at once, and a shader that misses any of them reads as a decal:

1. **A silhouette with a narrow base and a licking tip.** Real flames are widest about a third of
   the way up, narrow to a point, and the point is never still. The profile here is a quarter-width
   at the base, a sine peak at a third, and a taper to `FLAME_TIP = 0.90` of the quad's height.
2. **Turbulence that rises.** The noise must scroll *down* the UV so its features climb the flame,
   and its amplitude must grow with height so the head stays steady and the tip licks. A uniform
   noise reads as a boiling blob.
3. **Heat.** The core is yellow-white, the body is the scene's fire colour, and the rim is a deep
   red. That ramp is what makes an additive billboard read as fire instead of as an orange smudge:
   additive blending alone gives a flat wash.

The craft constraint that decides the implementation: **a flame must not be lit by the scene.** It
is an emitter. Any lighting on it darkens it under a shadow and breaks the illusion that it is the
source. So: `AlphaMode::Add`, no shadow casting, no prepass, no depth write, and the vertex stage
billboards it so a camera orbit never catches it edge-on.

## How shipping games do it

- **Doom (2016)**, **Dark Souls III** and most stylised engines use a small set of flipbook sprites
  for distant fire and a shader for the fires the player stands next to. The flipbook's weakness —
  visible repetition — is why the near ones must not be one.
- **Little Nightmares** and **Inside** (both named in the corpus's game-art literature) treat the
  flame as the scene's only saturated warm element; the whole frame's colour script is built around
  it, which is only possible if the flame's colour is a parameter, not baked into an atlas.
- **Ghost of Tsushima** layers two additive quads at different phases for its braziers rather than
  one, which is the cheap way to break up a silhouette's repetition. This crate's answer is the
  same trick at the *instance* level: one quad, but every fire at its own phase and height.
- **Team Fortress 2**'s illustrated ramp is the reference for keeping the flame's warm core from
  clipping to white under a bloom pass: the core's yellow is deliberately below 1.0 in two channels.

## Model adopted

`src/flame.rs` + `src/flame.wesl`:

- One `FlameMaterial`, built once at `Startup` from `FlameLook { color, strength, max_height }`.
  Its uniform is `(color with alpha = strength, Vec4(seconds, max_height, 0, strength))`.
- One mesh: `Rectangle::new(1.0, 1.0)`, a unit square in the local XY plane. The vertex stage
  billboards it (its X follows the camera's horizontal right, its Y is world up), scales it by the
  flame's height, and lifts its base to the site's origin.
- Per-flame numbers ride on `MeshTag::new(pack_flame_tag(phase, height, max_height))`: phase
  unsigned-normalised in the low 16 bits, height as a fraction of `max_height` in the high 16, which
  is what WGSL's `unpack2x16unorm` reads.
- `dress_flames` gives every `FlameSite` without a `Flamed` marker exactly one `Flame` child, through
  `queue_silenced` so a site despawned in the same frame cannot panic. `update_flames` writes the
  shared clock once a frame on `Time<Real>`, wrapped hourly, so a paused world's fires still move;
  a changed `FlameLook` is written in place rather than rebuilding the material.
- The child carries its own `Aabb` (and `NoAutoAabb`) instead of the engine's: the CPU box of the 1×1 quad does
  not cover what the vertex stage draws, and `NoFrustumCulling`, this note's first answer, drew every flame in
  the world every frame. The box, and why the instance scale must be divided out of it, are in
  `docs/research/flame-culling.md`.

## Parameters

| parameter | value | source |
|---|---|---|
| quad aspect | `FLAME_ASPECT = 0.6` (width / height) | design default, Sanctuary's |
| half width at the widest | `0.20` of the height | design default |
| tip | `0.90` of the quad's height | design default |
| noise | 3 fbm fields, 4 octaves each, scrolling at 1.7 / 1.7 / 2.4 units/s | design default |
| lean | `0.04 sin(1.7t) + 0.02 sin(3.1t + 1)` | design default |
| default colour | `LinearRgba::rgb(1.0, 0.55, 0.18)` | design default (a campfire's orange) |
| default strength | `1.0`, host-clamped to 0.35..1.6 | Sanctuary's palette `ember_strength` clamp |
| default `max_height` | 4.0 world units | design default: a metre world's tallest brazier |
| clock | `Time<Real>`, wrapped hourly | Sanctuary's |

`max_height` is a **startup** parameter: a site's height is packed as a fraction of it, and a tag
already packed cannot be re-scaled. Changing it after sites spawn re-scales the shader's size but
not the tags, so it is documented as set-once.

## Validation targets

- Every `FlameSite` gets exactly one `Flame` child, and a second update does not twin it.
- The child's `MeshTag` unpacks to the site's phase and height within 1e-3.
- A changed `FlameLook` changes the shared material's `base` and `params.w`.
- `pack_flame_tag` / `unpack_flame_tag` round-trip, a height over `max_height` packs full, and a
  `max_height` of zero packs a height of zero.
- In the courtyard, every brazier and torch has a flame, and the flame is *at* the fire with the
  light a little below it, not inside it.
- `tools/wesl-check` compiles `src/flame.wesl` to WGSL and naga parses it. A WESL error is silent at
  build time and shows only as a dark surface in a capture, so this is a gate, not a nicety.

## Deviations

- **`FlameSite` replaces the "a fire is a light" assumption.** Sanctuary dressed a flame on every
  `FireLight` and let a `NoFlame` marker opt out. The crate draws only where a host spawns a
  `FlameSite`, so the decision of which fires have a visible flame stays with the game — and a torch
  whose *light* is 1.5 m up but whose *flame* is on its head needs exactly that split.
- **The material is not rebuilt on a look change.** Sanctuary rebuilt it on a palette change; here
  `Assets::get_mut` writes the three changed values, so a recolour does not re-prepare a pipeline.
- **`max_height` is a `FlameLook` field with a documented set-once contract**, rather than a
  constant, because a foot-scale world and a metre-scale world need different ones. The
  set-once restriction is the honest consequence of packing the height in 16 bits against a shared
  scale.
- **The flame's brightness is the fire's own flicker, on virtual time, and its shape stays on real
  time.** This note used to say the whole flame ran on `Time<Real>`; only the silhouette's noise does
  now. The brightness is the light, so it follows the light's clock; see
  [`fire-light.md`](fire-light.md) and [`flame-flicker.md`](flame-flicker.md). The tag gained a
  brightness byte, taking it from (phase 16 | height 16) to (height 16 | phase 8 | brightness 8).

## Iteration 7: the flame's own kindle

**The defect.** A fire that a host lights (a `Kindle` on its light) flares its *light* up over
`KINDLE_SECONDS` (0.6 s) — and its *flame* appeared at full height, full brightness, on the first frame.
Measured on the courtyard: a brazier's flame mesh is spawned whole, so the frame the fire is lit shows a
full-size flame over a light at 0 % of its base. The craft's own line in `fire-light.md` says the two are
one event ("that overshoot is the sound cue's visual half"), and a flame that is already there when its
light arrives is the opposite: the source precedes the light it motivates, which is the one thing
`lighting-look.md`'s motivated-source rule forbids.

**What the craft says.** The same ignition envelope the light uses, applied to the flame:
`fire-light.md` — "a fire that is being lit flares *past* its running brightness and settles, because the
kindling catches unevenly before the fuel is alight". A flame grows from a coal's glow to its full height
as the fuel catches; it does not blink into existence.

**Model adopted.** `FlameSite` gains `kindle: Option<Timeline>` — no, a *duration* is enough: the same
`Kindle` the light carries, read once. The flame's **height and brightness** are scaled by
`kindle_gain`:
- `src/model/fire.rs` already has `kindle_gain(t)`, which is 0 at ignition, peaks a little over 1 at 85 %
  of the way, and is exactly 1 after `KINDLE_SECONDS`. It is the envelope the light already uses, so the
  flame and the light share one curve rather than two that could drift apart.
- The site's height is packed into the `MeshTag` as a fraction of `max_height`, and a packed tag cannot
  be re-scaled — so the *scale* is not in the tag. It goes in the flame's `Transform`, which is free: the
  child's transform is `IDENTITY` today. `update_flames` writes each flame's scale from its site's kindle,
  and the shader's own tip/noise animation is untouched.
- A site with no kindle (the common case: a fire that has been burning all along) is left at scale 1 and
  costs nothing.

**Deviations.** The flame's *width* scales with its height, because a fire that is catching is smaller in
both; a flame that grew tall but stayed wide would read as a sheet. No separate flame-only curve: sharing
`kindle_gain` is the point.

### Iteration 9: the flame's colour while it catches

**The gap.** Iteration 7 made a flame *grow* with its light, but its colour stayed the steady ramp from
the first frame: a catching fire showed the same host fire-colour and yellow core it has when fully alight.
A young, small fire is redder — Planck's law is why: a cooler emitter is redder, which is the crate's own
`blackbody_colour` argument, cited in `lighting-look.md` from *Physically Based Rendering* ch. 12.

**Model adopted.** The shader's ramp is pulled towards an ember red while the fire catches. `params.z`
carries **how far the scene's catch has run** (1 for a flame that took long ago), written by
`update_flames` from the *youngest* fire in the frame; the fragment mixes `EMBER_RED * max(heat, 0.35)`
towards the steady ramp by that number. One shared material means one number for the scene, which is
honest for the courtyard (its four braziers light together) and is stated rather than hidden.

**Measured on the courtyard's catch.** The flame core's blue channel — the number that says redder — is
**34–42 while catching (frames 15–17) against 65–69 once settled (frames 27–35)**, roughly half. The
signal the shader reads is pinned by `the_shader_is_told_when_a_fire_is_still_catching` (`params.z` < 0.2
mid-catch, exactly 1 after) and `a_scene_with_no_catch_keeps_the_steady_ramp`.

**A trap worth recording:** `catch` is a **reserved WGSL word**, and `tools/wesl-check` failed on it —
which is exactly the class of error `AGENTS.md` warns is silent at build time and loud only in a capture.

### Iteration 10: the catch is per flame, not per scene

Iteration 9 carried the catch in the material's shared uniform, with the *youngest* fire in the frame
deciding. For the courtyard, whose four braziers light together, that is exact — but for any host it is
wrong: lighting **one** torch tinted every flame in the world for three tenths of a second. A visible
artifact, so it ranks above the remaining beauty items.

**Model adopted.** The catch is already per flame in the entity world — `update_flames` writes each
flame's `Transform` scale from its own fire's `kindle_gain` — so the shader reads it there. The vertex
stage takes `length(world_from_local[0].xyz)` (the instance's x scale), passes it flat, and the fragment
mixes the ember in by it. The uniform's `params.z` goes back to unused, and one shared material can no
longer carry one fire's state to another.

**A claim of mine that was wrong, corrected in iteration 12.** I said here that the stub of
`get_world_from_local` returning the identity was a blind spot that had to be fixed first. It is not:
`tools/wesl-check` **compiles** the module — it never runs it — so a stub's *body* is dead weight and only
its signature is checked. Reverting the stub to the identity still passes, and a deliberate type error in
the fragment still fails, which is the proof both ways. Stub bodies are documentation, not a check.

**What the test can and cannot catch.** `one_fire_catching_does_not_tint_another` passes under both of the
two ways the isolation holds — the `f32::MAX` skip and a `kindle: 0` site's gain of exactly 1 — and
removing either does not fail it. That is recorded in the test's own comment: it pins the property, it
does not detect a regression in the loop's shape.

**Pending index.** Nothing new to index: this is the existing `kindle_gain` applied to the second half of
the same event. A source that measured a *flame's* height against time after ignition would replace the
envelope's shape; the corpus has none (Maynard 2013 measures light and is unobtainable, per
`fire-light.md`).

## Pending index

The two noise sources (the simplex formula and the integer gradient hash) are not in the corpus and
cannot be indexed: they are web references. Jarzynski & Olano 2020 (JCGT 9(2)) is the paper that
would settle the hash if it can be imported; it is not present today.

**GPU Gems ch. 6 itself is now in the corpus and indexed** (stem `GPU Gems … Anna's Archive`, read live
2026-10-26 in iteration 16). Its §6.1 has been read in full, and it supports less than this note's
`Sources` row used to imply: it describes "2D distortion-based flames" perturbed by "a GPU-generated
perturbation function that altered a flame shape to give it a realistic motion", calls that class
"perfect for creating candle-like flames", and records that the Vulcan demo itself abandoned it —
"Both procedural techniques have strong advantages, but they didn't meet our goal of creating a
believable raging fire with smoke". So it is the source for the *technique* and for the puff-motion
principle, and **not** for the noise in `src/flame.wesl`, whose provenance stays UNVERIFIED.
`flame-silhouette.md` carries the same correction.
