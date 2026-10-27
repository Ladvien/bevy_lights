# The fuel bed is a surface, not a slab of light

`examples/courtyard` gives each brazier a coal bed: a 0.34 m box whose `StandardMaterial` has
`emissive = blackbody(1900 K) × 1.4`, a **constant over every fragment of it and the same at every
camera angle**. Measured this iteration, that constant is a red-channel clip at every camera the
showcase uses and a **flat white slab** at the camera iteration 19 just made the flame visible from:
the bed reaches `min channel ≥ 245` over 31 px at 85° of elevation, and `R = 255` over 5867 px of the
frame, while at the hero camera it clips 1839 px of `R`. Because the slab is the brightest thing in
the frame at exactly the angle a flame becomes readable from, it both *is* the artefact and *hides*
the one iteration 19 spent five rounds on.

The craft's answer to this is not a number: it is that a bed of coals is a **surface with a
temperature field on it**. Some of it is charred and dark, some of it is at the temperature that
glows, and the glow is patchy, moves, and is hottest where the air reaches it. A uniform emissive is
the one thing a real fuel bed never is.

## Sources

| source | what it is | use |
|---|---|---|
| `10.1145_3130800.3130814` — Pirk, Jarząbek, Hädrich, Michels & Palubicki, *Interactive wood combustion for botanical tree models* (ACM TOG 2017) | a wood fire simulated as virgin wood → volatiles + char, with the char layer a physical object | the model: char is a distinct material from wood, it insulates, and its fraction is a **field over the surface**, not a constant. The paper's pyrolysis smoothstep runs between two temperatures it states: "smoothly from zero to one for temperatures between $T_0 = 150\,°C$ and $T_1 = 450\,°C$" |
| `10.1017_jfm.2018.707` — Xia & Zhang, *A vortex-dynamical scaling theory for flickering buoyant diffusion flames* (JFM 2018) | the crate's existing fire model | the flame's own rate is unchanged here; what matters is that the **plume is axisymmetric**, so a bed's glow is under the axis, not spread to the rim |
| `PBR3_12_Light_Sources` — *Physically Based Rendering* 3rd ed., ch. 12 | blackbody emitters | "**Blackbody emitters are perfectly diffuse; they emit radiance equally in all directions.**" A glowing char surface is a blackbody emitter, so its brightness must not depend on the viewing angle — the current slab's does not either, but its *shape* does not survive being seen flat-on |
| `10.1525_fq.2004.57.3.24` — Prince, *The emergence of filmic artifacts* (*Film Quarterly* 2004) | what a digital camera's night costs | the craft's own statement of why a clipped source reads as video: Uta Briesewitz on *Session 9*, "the highlights were a dead giveaway that it was video" because "they kept blowing out" |
| `GP` — Nguyen, *Fire in the Vulcan Demo*, GPU Gems ch. 6 | the crate's own ancestor | the fuel source is *not* drawn as geometry in it at all: the fire is billboards over the emitter. The bed here is the one piece of a fire that is geometry, so it is the one piece that must carry real structure |
| `s2011-advances-pdf-slides` — SIGGRAPH 2011 *Advances*, Fire/Flamethrowers/Thrusters | production fire | the production budget goes on hiding that fire is made of discrete elements — "Rendering is working quite hard to try to hide the particle nature of the system." The same instinct applies to a bed: the slab's uniformity is what makes it read as a prop |

## Craft principle

The rule this iteration applies is the one the crate's own exposure note already carries: a practical's
brightest surface has to have *something* in it. Prince records what happens when it does not, in the
words of a working cinematographer rather than a spec —

> "On *Session 9*, the cinematographer, Uta Briesewitz, felt that 'the highlights were a dead giveaway
> that it was video' because they kept blowing out. In situations where she was shooting a character
> in a room with a window, she had to reduce the contrast range by lighting the room."

A blown highlight is not a brightness, it is an absence: the surface stops reporting itself. A gaffer
solving that problem on a fire does not turn the fire down — a fire is supposed to be the brightest
thing in frame. They put **structure in the source**, so that the eye can still read what it is
looking at while it is at the top of the curve. On a real brazier that structure is there for free:
coals are lumps, and between them is ash and shadow.

## How shipping games do it

Pirk et al. give the model in its own terms, and give the parameters with it. Pyrolysis is a front
that converts wood to char, and the conversion is a temperature ramp rather than a switch:

> "smoothly from zero to one for temperatures between $T_0=150\,°C$ and $T_1=450\,°C$."

and the char layer is an object with its own behaviour, thicker where the burn has gone further:

> "The char layer has an insulating effect proportional to its thickness (Char Insulation)."

The paper's whole apparatus exists because a fire's fuel is **spatially varying** — "We model material
properties, such as moisture and charring behavior, and associate them with individual particles" —
which is the property a constant `emissive` throws away.

The ancestor records the complementary choice, from the other side. Vulcan's fire is billboards and a
glow pass over a moving emitter; there is no fuel-bed geometry to be flat, because the fuel bed is
never drawn. That is why this defect is this crate's own: `examples/courtyard` draws the bed as a box,
so the box has to behave like a surface.

## Model adopted

The bed keeps its geometry and its colour, and gains a **char field**: a static, per-fragment
brightness that varies over the bed's surface, applied to the emissive as a texture.

1. **One small greyscale image per fuel bed style**, generated at `Startup` on the CPU into an
   `Image` — not an asset file, so the example stays one binary and the crate gains no data. It is
   built from the same integer-hash noise the flame shader uses, summed over three octaves, so it is
   reproducible from a seed and has no external provenance problem.
2. **The material multiplies its emissive by it**: `emissive = blackbody(K) × strength` unchanged, plus
   `emissive_texture: Some(field)`. Bevy 0.20's `pbr_fragment.wesl` does exactly
   `emissive = vec4(emissive.rgb * textureSample(..).rgb, emissive.a)` (line 325), so the texture is a
   **share of the emissive**, not a second colour.
3. **The field's statistics are the fix**, not its artwork: a mean chosen so the average emissive is a
   little under the clipping point, and a spread large enough that the brightest coals still reach the
   top while the dark ones fall well below it. Measured target: `min channel ≥ 245` over **0 px** at
   every camera and a **standard deviation of the bed's own pixels** that is a real fraction of its
   mean instead of ~0.
4. **The field is the fire's, not the camera's**: it does not change with elevation. PBR's rule is why
   — a blackbody emitter is perfectly diffuse, so its radiance is the same in every direction, and the
   only thing that may vary between angles is how much of it is *in frame*.

The field is the example's, because `examples/courtyard` is the thing that builds a bed's material and
the crate has no opinion about what a coal looks like. What the crate gains is the *knob that makes a
fire's own surface expressible*, and the measurement that proves it works.

## Parameters

The parameters below are the ones that shipped, and two of them differ from this note's first plan for a
reason that only a capture could give: **an 8-bit share of one fire colour cannot draw a coal bed**, and
neither can a ramp whose middle stop is past the display's white point. Both are recorded under
`## Deviations`.

| parameter | value | source |
|---|---|---|
| field size | 128×128, `Rgba32Float` | the bed is 0.34 m and the closest camera in the showcase is 4 m, so a texel is ~2.7 mm — far under a pixel at every captured angle. The format is `Rgba32Float` because the ramp's stops span 3.7 to 795 cd/m² and 8 bits clamps at 1.0 |
| lumps, coarse / fine | 5.6 and 9.4 across, fine weighted 0.74 | design default: a bed of coals is a *crushed* material, so a heap of 10 cm lumps settles into a matrix of fragments a few centimetres across. Measured at 44 px — the bed's own size at the hero camera — one size of lump reads as a mesh |
| lump radius, stretch, sparse cells | 0.36–0.62 cells, 0.62–1.24, 30 % | design defaults, chosen so shards overlap and a third of the cells hold no coal — the ash |
| fracture depth | 0.30 and 0.14 of the radius, two scales | design default: one term gives every lump the same sawtooth |
| core cooling | to 0.62 of the field, exponent 1.4 | the plume is axisymmetric (`10.1017_jfm.2018.707`), so the coals under the axis are the hottest and the rim is coolest |
| char stop | 950 K, floor 0.6/0.5/0.45 cd/m² | design default. Char has stopped burning and shows only the fire's light on it, which is grey; a black hole in a bed reads as a missing texture |
| ember stop | **1020 K** = 26.1 cd/m², DN 157 at `ev100` 6, at 0.30 of the field | `PBR3_12_Light_Sources`: a blackbody emitter's radiance is a temperature. Measured: at 1140 K the ember stop lands at **DN 255**, so every lump above the ash clipped in red and the structure survived only in green. 1020 K puts the ember at DN 157 |
| fuel stop | **1300 K** = 795 cd/m², DN 255/217/0 | the hottest a coal bed gets without a visible flame: red and green at the top of the display's range, so the hottest coals carry their colour |
| field seed | `LIGHTS_SEED`'s default (7) | so a capture is reproducible, like the figures' layout |

## Validation targets

- **No clipped fuel bed, at any camera.** `min channel ≥ 245` over **0 px** at 0/15/30/45/60/75/85°,
  against the baseline's 31 px at 85°. This is iteration 19's own gate and the one its attempt failed.
- **The bed has structure.** The bed's own pixel window has a **standard deviation ≥ 12 DN** in the red
  channel at 85°, against a baseline whose bed is a rectangle of near-constant value.
- **The source stays the brightest thing.** Peak red on the bed is **≥ 240** at 85°: the fix must give
  the bed structure, not turn the fire down.
- **The hero camera reads the same fire.** At the default `(0, 9, 16)` camera the mean of the bed's
  window is within **10 %** of the baseline's, and nothing else in the frame moves by more than a
  pixel's worth: the field is a texture on the bed and touches no light, no flame and no stone.
- **The flame from iteration 19's reverted work is not regressed**, because nothing here touches the
  shader: 0/15/30/45/60/75/85° flame pixel counts unchanged to within the noise of the flame's own
  animation.
- **The frame budget does not move**: no new draw (the texture is sampled in the existing stone pass),
  and `bench_lights` runs no shading at all, so the check is that the CPU bench is unchanged and the
  capture's frame time is unchanged.

## Deviations

- **The field is example-side, not a crate feature.** `AGENTS.md`'s ownership map gives "what a lamp is
  made of" to the host: the crate owns the practical's *light*, and a coal bed is geometry. Adding a
  `CoalBed` component here would be inventing a feature for one example's prop.
  The crate does get a real change from this, though: `blackbody_colour`'s own doc claims a
  surface's emissive must not depend on the viewing angle, and nothing in the crate tested it.
- **The char field is noise, not an authored texture.** An authored tile would be better art and is
  not available: this repository ships no image assets but its GIF, and adding one for one example's
  prop is a larger change than the defect. The noise carries the *statistics* that matter (structure,
  a hot and a cold fraction) and not the *shape* of real coals, which the note records as its gap.
- **The bed is not given per-fragment temperature evolution.** Pirk et al. animate char formation over
  a burn; this bed is a fire that has been burning for hours, so its field is static. The alternative —
  a field that kindles with the fire — was considered and rejected for this iteration because the
  brazier's `Kindle` is a *re*-light, not a first burn, and a re-lit bed that re-charred would be the
  wrong story.

## Pending index

Read live from the running library on 2026-10-26.

- **The char field's shape.** The corpus has combustion *models* (Pirk et al., Scintilla) and no
  authored coal-bed reference. A photograph of a brazier's coals at a known exposure, or a game
  production talk that shows one, would replace the noise with a real field. Recorded as this note's
  main gap.
- **A char surface's actual radiance.** Pirk et al. give the pyrolysis temperatures, not the emissive
  radiance of char at them. A measurement of a charcoal bed's surface temperature would turn
  `low`/`high` from design defaults into a cited spread.
- The crate's long-standing gap is unchanged: the flame shader's simplex-noise and hash provenance.

## What great looks like

Judged at full frame and at 4× on the bed, at every camera the crate's own showcase and a tactical
game use:

- **A bed of coals, not a lit rectangle.** At 85° the bed is a patchwork: bright coals with dark
  char and shadow between them, and no pixel at the top of every channel.
- **The source is still the brightest thing in frame.** The hottest coals sit at or near the top of the
  curve; the fix adds structure under a bright source rather than removing the brightness.
- **The flame reads against it.** At 85° the flame's own warm plume is distinguishable from the bed
  it stands in, which the baseline's white slab made impossible.
- **The same bed from the side.** At the showcase camera the braziers read as they did: a warm glow in
  a dark bowl, with no new visible texture noise at the scale of a pixel.
- **No new cost.** One texture sample in a pass that already samples the stone, and no extra draw.
