# The flame from above: a billboard is not a fire when the camera goes overhead

Every flame this crate draws is one upright quad, camera-facing about the world's vertical axis. That is an
**axial** billboard, and an axial billboard's normal is horizontal, so the area it presents to a camera falls as
`cos(elevation)` and reaches zero when the camera looks straight down. Measured on `examples/courtyard` from a
fixed 4 m away, rising from 0 to 85 degrees of elevation: the flame above a brazier's coal bed was a 159 px plume
at 0 degrees and **completely gone by 75** (a lit coal slab and no fire at all). A tower-defence, isometric or
tactical camera lives at exactly those angles, so this is a game's most common view of its own fires.

## Sources

| source | what it is | use |
|---|---|---|
| `book_real_time_rendering_4th_akenine` §13.6, §13.6.2, §13.6.3 — Akenine-Möller et al., *Real-Time Rendering* 4th ed. | the taxonomy of billboards and the failure of each | the defect, its cause and the shape of the fix: "the viewer flies over the trees and looks down, the illusion is ruined … One workaround is to add a horizontal cross section" |
| `10.1017_jfm.2018.707` — Xia & Zhang, *A vortex-dynamical scaling theory for flickering buoyant diffusion flames* (JFM 2018) | a buoyant diffusion flame solved as axisymmetric | *why* the cross-section is a disc: the flame this crate models has rotational symmetry about its own axis |
| `GP` — Nguyen, *Fire in the Vulcan Demo*, GPU Gems ch. 6 §6.1 | the technique this shader descends from, and its own account of the failure | the ancestor shipped the same defect and says so: "The screen-aligned nature of the effect made it sensitive to the camera view angle (constraints were needed for top and bottom views)" |
| `Ga` — Gregory, *Game Engine Architecture* 3rd ed., §11.4.3 | particle effects in a shipped engine | "The geometry is often camera-facing (i.e., billboarded), meaning that the engine must take steps to ensure that the face normals of each quad always point directly at the camera's focal point" — the norm is *at the focal point*, not merely at the camera's heading, which is what an axial billboard does |
| `s2011-advances-pdf-slides` — SIGGRAPH 2011 *Advances*, Fire/Flamethrowers/Thrusters | production fire | the shipped answer is many overlapping particles precisely so "rendering is working quite hard to try to hide the particle nature of the system": a single quad has one face to lose, and a body of them does not |
| `10.7456_tojdac.1674124` — Doğaner, lighting design in *The Lights of the Dark City* | the craft rule for a visible source | a practical's job is that the viewer can "logically find where the light comes from"; a fire that has vanished overhead cannot motivate the brightest thing on the ground |

## Craft principle

The craft's rule here is the one this crate has cited since iteration 1 and this iteration is the first to apply
to the camera's own position: **a practical must be visibly where its light is**. Doğaner states the mechanism as
the audience's, not the crew's —

> "In the story world, visible sources not only manage the viewer's perception, allowing them to logically find
> where the light comes from, but also allow the film crew to overcome technical limitations."

A gaffer never has this problem, because a real fire is a volume: seen from above, a brazier's flame is a shorter,
rounder, brighter version of itself, not a disappearance. That is the craft target — the fire's *light* keeps
working at every camera angle in this crate (a `PointLight` has no facing), so the fire's *drawing* is the only
part that can break, and it broke in exactly the direction the light does not.

## How shipping games do it

The reference gives the taxonomy and the failure of each type, in its own words, for the two that matter here.

Section 13.6.3, axial billboards — the crate's type:

> "Because of this behavior, a problem with axial billboarding is that if the viewer flies over the trees and looks
> down, the illusion is ruined, as the trees will appear nearly edge-on and look like the cutouts they are. One
> workaround is to add a horizontal cross section texture of the tree (which needs no billboarding) to help
> ameliorate the problem."

Section 13.6.2, world-oriented billboards, and the direct comparison:

> "Being world-oriented, the billboard directly faces the viewer and can rotate along this view axis. It is rotated
> so that the up direction of the billboard aligns as best as possible with the up direction of the world. With an
> axial billboard, the world's up direction defines the fixed axis, and the billboard is rotated around it so that
> it faces the viewer as best as possible."

> "overhead each type of billboard, the world-oriented version will fully face it, while the axial version will be
> more affixed to the scene."

So there are two candidate fixes and the reference names both: turn the quad to face the viewer fully (a
world-oriented billboard), or add a cross-section. Section 13.6.3's own recommendation for the case where the
object has a *shape* worth keeping is the cross-section, with the caveat that the quad itself is not re-oriented.

The crate's ancestor is the same technique and records the same weakness, from production:

> "The screen-aligned nature of the effect made it sensitive to the camera view angle (constraints were needed for
> top and bottom views) and to motion in general (moving toward and away from the camera didn't work well in 2D
> sometimes)."

and the production answer — a body of particles — is quoted in the SIGGRAPH 2011 fire talk as effort spent on the
same concealment: "Rendering is working quite hard to try to hide the particle nature of the system." A hundred
overlapping puffs cannot all be edge-on at once; one quad can.

## What was attempted, and what shipped

**Round 6 ships (iteration 139); rounds 1-5 (iteration 19) were reverted.** The defect, its cause and the measurements
stand. Rounds 1-5 are kept because each one rules something out; round 6 is the section after them.

### What was measured, before

The camera rises over a brazier from a fixed 4 m, looking at the coals. The flame above the coal bed was:

| elevation over the fire | flame above the coal bed |
|---|---|
| 0° | 159 px tall plume |
| 15° | 196 px |
| 45° | present |
| 60° | present, foreshortened |
| 75° | **invisible** — the lit coals and nothing else |
| 85° | **invisible** |

The cause is exact and is RTR4 §13.6.3's: the quad's normal is horizontal, so the area it presents falls as
`cos(elevation)`; at 75° it has 26 % of its side-on area and at 85° 9 %, and the flame's own translucent edges vanish
before its core does. The torch-bearers' flames at 85° were a **1-pixel sliver**.

### The five rounds

1. **Tilt the quad's `up` axis toward horizontal** as the camera rises — the literal reading of "world-oriented
   billboard". **Worse than nothing.** It draws the tongue on a plate leaning away from the camera: at 60° the flame
   became a broad flat wash across the brazier's top with its tip smeared over the coal slab. Geometric, not tunable:
   a vertical flame has no extent along the direction the camera looks, so rotating its quad into that plane can only
   foreshorten it into a smear. Captures: `/home/ladvien/l19/elev5/d60.png`.
2. **Keep the quad upright, draw the disc inside it, centred along world-up** — a horizontal disc of radius `R` seen
   from elevation `e` projects to an ellipse `R` wide and `R·sin(e)` tall, so draw that. The disc landed **coplanar
   with the coal bed** and was depth-rejected; the brazier still showed a bare slab.
3. **Same, lifted to the plume's own centre** (`FLAME_CENTRE = 0.38` of the flame's height) and with the base band
   faded by `cross` (the `smoothstep(0.0, 0.06, y)` seat erased the whole disc, because every fragment of a
   cross-section drawn in an upright quad is at the quad's base's height). The torch's flame appeared; the brazier's
   did not.
4. **Isotropic quad overhead** (`FLAME_OVERHEAD_ASPECT`). The disc was an ellipse squashed in the wrong direction: the
   radial used `FLAME_ASPECT` for both axes.
5. **Stretch the quad's height by `1/cos(e)`** to cancel the upright face's own foreshortening, with the disc's
   vertical semi-axis at `R·sin(e)`. This is the round that worked arithmetically: the brazier's added light overhead
   went from **143 px of ≥3 DN** (round 3) to **2962 px in an 81×57 bbox** (round 5), and the torch's 1-pixel
   sliver became a compact puff at 85°.

### Why round 5 was NOT YET

At 85° the brazier's disc was swamped by a clipped white coal slab (an example defect, fixed since by the char-field bed,
`fuel-bed.md`), and between 50° and 76° the tongue's full-size teardrop sat under the disc instead of giving way to it.
Round 5's arithmetic (a `1/cos` stretch with an `R·sin(e)` semi-axis) was right; its construction was not.

### Round 6: what shipped

1. **The coal bed was already fixed** (iteration 137), so the first prerequisite of the round-5 list was met: at 85° the
   bed is a textured ember field, not a slab.
2. **The upright stretched quad was the real fault, not the blend.** A world-space quad stretched by `1/cos(e)` is
   `stretch · size` tall in the world, so its upper vertices rise towards a camera 4 m up and are magnified by
   perspective; the screen image was a trapezoid whose bed-side edge was nowhere near the plume's centre (debug capture,
   solid colour). And a plane through the brazier's origin is **inside the bowl**, which the depth test hides. Sliding
   the stretched quad along its view rays onto a camera-facing plane keeps the corner pixels but not the interior: the
   perspective-correct `uv` is then no longer the flame's image on screen, which is what moved the flame up and off
   the bed in three captures.
3. **Past `OVERHEAD_START` the quad is a camera-facing quad through the plume's centre** (`SOURCE_HEIGHT`, the point the
   depth fade already uses), `OVERHEAD_HEIGHT` flame heights tall. The upright flame is drawn on it as the image it has on
   screen: a world height `h` lands `h·cos(e)` above the centre, so the tongue is sampled at
   `SOURCE_HEIGHT + sy / cos(e)`. Below 50° the vertex stage keeps the upright quad, so level frames do not change.
4. **The cross-section** is an ellipse `R` wide and `R·sin(e)` tall (floor `OVERHEAD_SQUASH`), faded in by
   `smoothstep(OVERHEAD_START, OVERHEAD_FULL, e)`. Its edge and its brightness are rippled by **its own fbm in the
   ellipse's coordinates**: reusing the tongue's `n1`, which runs along the flame's height (a few pixels long from above),
   drew horizontal streaks across the bowl at 80° and 85°.
5. **The tongue gives way**: its foot softens by `0.30·topdown` (a hard horizontal line across the bed was the tongue's seat)
   and it is dimmed by `OVERHEAD_TONGUE` at full overhead.

Measured on `examples/courtyard`, brazier at 4 m, 1280×720, frame 60 (`/tmp/l139/`): a bare coal bed at 75°, 80° and 85°
before; a bright core with an orange halo centred on the bed at every angle from 50° to 86° after, the tongue shortening
into it over a sweep of eleven angles with no step (window means move by under 5 DN between neighbouring angles, which is
the camera's own travel). At 0°, 30° and 45° the frame differs from the build before by 1 channel of 1 pixel
(`magick compare -metric AE`: 0.004, 0.007, 0.004), i.e. is unchanged.

### Round 7: structure in the cross-section (iteration 140)

Round 6's deviation list names its own next step: from above the flame "is a rippled ellipse with a hot core ... its edge is
soft, which is the main thing a sharper art director would ask to be crisper". The capture at 75° and 85° (`/tmp/l140/before/z75.png`)
agrees: a soft orange halo over a hard-edged coal bed, with a brightness ripple and no licks.

**Craft source** (cinematography), the existing citation: a practical must let the viewer "logically find where the light comes from"
(Doğaner, `10.7456_tojdac.1674124`). A blur reads as a lamp; a fire reads as a fire when its outline has tongues that lean and
part, which is the one thing a viewer checks about a source at a glance.

**Production source:** Gregory, *Game Engine Architecture* 3rd ed. §11.4.3 and `s2011-advances-pdf-slides`, already quoted above: shipped fire hides
its nature by structure, "rendering is working quite hard to try to hide the particle nature of the system". One soft disc is the opposite.

**Technical source:** Xia & Zhang, `10.1017_jfm.2018.707`: the flame is a buoyant diffusion flame whose structure is set by "toroidal vortices"
shed around an axis, so its plan view is rotationally symmetric on average and lobed instantaneously: licks radiate from the axis,
not ripple across it. Akenine-Möller et al. §6.3's turbulence sum (already the shader's `fbm`) is the way to make them.

**Model adopted.** The cross-section's radius is modulated by a function of the *direction* from the axis (a noise of the unit vector, so it is
periodic in angle and has no seam), scrolling in time, with a larger amplitude than the round-6 ripple; its edge is `smoothstep(0, 0.18)` instead of
`0.35`; and its brightness is streaked radially by a second noise of the direction, scrolled outward. Each constant is a design default,
judged in captures. **Shipped values:** lobes `fbm(dir*0.9 + (0.6 t, 3.7))`, radius `R*(0.85 + 0.8*(lobes-0.5))`, streaks `fbm(dir*1.6 + (5.1, -1.5 t + 4 r))` mixed in from 0.1 to 0.6 of the radius, edge `smoothstep(0, 0.26)`. A first try (`dir*1.5`, amplitude 1.0, edge 0.18) read as a starburst/explosion and was softened. The tongue, the quad and the 50°/76° blend are unchanged, so frames below 50° are bit-identical.

**Validation.** At 60°-85° the outline has at least three visible lobes and the edge is crisper than round 6's; no pixel with min channel
>= 245 in the bed window; ground-level frames (0°, 30°, 45°) unchanged to 1 channel of 1 pixel.

### Round 8: a stack of three discs (iteration 154) — NOT YET, not shipped

**Sources, all already quoted above:** craft — Doğaner (`10.7456_tojdac.1674124`), a practical must be visibly where its light is; production — Gregory §11.4.3 and `s2011-advances-pdf-slides`, shipped fire is many overlapping layers so the single-sprite nature is hidden; technical — Xia & Zhang (`10.1017_jfm.2018.707`), a buoyant diffusion flame is a vertical column of toroidal vortices, so its cross-sections are a *series* along the axis, and Akenine-Möller §13.6.3's "horizontal cross section" workaround, applied more than once. distill was down, so no new search was run; no new default is cited, every constant below is a design default.

**Model tried.** The single disc becomes three, at world heights `-0.25, +0.05, +0.35` flame heights from the plume centre (`OVERHEAD_LAYER_BASE -0.25`, `STEP 0.30`), each lifted up the screen by `height * cos(elevation)`, narrower by 20 % per layer (`TAPER`), dimmer by 25 % per layer (`COOL`), each with its own lobe and streak noise offset; round 7's gain (mask x1.6 clamped 1.4, heat x1.3) and a crisper edge (`smoothstep(0, 0.16)`). The patch is `docs/attempts/overhead-stack-round8.patch`.

**Result** (`courtyard`, frame 90, 1280x720, eyes 6.63,4.85,5 / 5.5,6.5,5.2 / 5.3,9,5, seeds 7 and 3). Three rounds: with layers at `-0.10, 0.07, 0.24` the change was barely visible; widening them to the values above made the fire wrap and stand on the coals at 66° (the skirt now overlaps the bed's near half instead of fanning behind it) and gave a defined pale core with a lit halo at 76° and 85°. Ground level is unchanged: 0° identical, 30° and 50° differ by 1 channel of 1 pixel (`magick compare -metric AE`). Critique: at 66° the plume above the layers is still a soft fan, the three layers read as stacked translucent cut-outs rather than one body, and the ground is a near-uniform saturated orange so contrast stays low (scene exposure). Verdict **NOT YET**, so reverted; the next step is a fourth, sharper construction or a darker exposure to judge it, not more constants.

### Round 9: five discs, lower gain (iteration 155) — NOT YET, not shipped

Same sources as round 8. Applied `docs/attempts/overhead-stack-round8.patch`, then changed the stack to five discs (`STEP 0.16`, `TAPER 0.12`, `COOL 0.15`, gain 1.6/1.3): at 66° the plume stands taller and wraps the coals more convincingly than round 8's three discs, but at 76° and 85° it fills the bed with an opaque cauliflower burst. A second try (`TAPER 0.16`, `COOL 0.2`, gain 1.15/1.1, `docs/attempts/overhead-stack-round9.patch`) lets the coals show again and is within a few DN of the unmodified shader at 76° and 85°. Neither reads as a fire from directly above rather than a blob; the ground around the brazier is saturated salmon, so a slightly brighter core cannot be judged apart from it. Reverted. The next step is not another constant: it is a tongue-quad construction (upright leaning ribbons seen end-on) or a darker courtyard exposure to judge against.

## Parameters

Round 6's values, all in `src/flame.wesl`; each is a design default calibrated in captures unless a source is named.

| parameter | value | source |
|---|---|---|
| `OVERHEAD_START` | 0.873 rad (50°) | design default: below a third-person camera's usual pitch of ~50° the flame is the upright tongue exactly as before, so no ground-level frame changes |
| `OVERHEAD_FULL` | 1.326 rad (76°) | design default: the elevation at which an upright quad has lost `cos 76° = 76 %` of its area, calibrated in captures |
| `squash` floor | 0.18 | design default: `sin(10°)`, keeps the ellipse from becoming a line at a camera that is nearly level |
| disc radius `R` | `FLAME_HALF_WIDTH = 0.20` of the flame's height, rippled `±25 %` by the disc's own fbm | the plume's own widest half-width, already in the shader |
| `OVERHEAD_HEIGHT` | 1.0 flame heights | design default: covers the foreshortened tongue (±0.35) and the ellipse (±0.2) at every elevation past 50° |
| `OVERHEAD_COS_FLOOR` | 0.05 | design default: `cos` is floored when a screen distance is turned back into a world height, so a camera straight down does not divide by zero |
| `OVERHEAD_TONGUE` | 0.6 × `topdown⁵` | design default (round 12: was `× topdown`): the share of the tongue's brightness that is gone at full overhead, applied late so a flame still in view at 60-70° keeps it, judged in captures |
| tongue foot softening | `+0.30·topdown` on the 0.06 foot | design default: removes the hard line across the bed |
| disc brightness | `clamp(1.2·(ripple − 0.5) + 1.0, 0, 1.3)` | design default |
| overhead rim cooling | `1 − 0.45·radial` | design default, from the tongue's `1 − 0.85·u` reduced because a cross-section has no long axis to fall off along |

## Validation targets

- **At 4 m and 85 degrees over a brazier, the flame adds light** where the baseline adds none: frame-minus-base in the
  bed's window, not a warm-pixel count. Round 5 reached **2962 px above 3 DN in an 81×57 bbox** against round 3's 143
  px; the bar for GREAT is that the added light is *readable*, which needs the bed itself not to be clipped (below).
- **At 4 m and 85 degrees over a torch, the flame is a compact puff** rather than the baseline's **1-pixel sliver**.
  Met in round 5; **not re-measured in round 6** (the only torch in the 85° capture sits at the frame's edge, where the ray's
  elevation is about 70°, and it reads the same before and after).
- **Ground level is unchanged, pixel for pixel**, at 0 / 15 / 30 / 45 degrees: `OVERHEAD_START` is 50 degrees, so
  `cross` is exactly 0 and every term it multiplies is exactly its old value. This is the gate that keeps the
  change from costing anything at the camera the crate's own showcase uses.
- **No clipping, and no clipped backdrop.** The flame's own pixels stay under the ceiling at every angle, *and* the
  surface it is drawn over is not already at it. Measured: `examples/courtyard`'s coal bed reaches
  `min channel ≥ 245` over 74 px at 85° and 0 px at the crate's hero camera. Round 6: 0 px at 75° and 85°, before and
  after (the bed is the char-field bed now), and the flame's brightest pixels are the pale-yellow core.
- **Flicker and the catch are untouched**: the brightness byte and `kindle` path are not in this change.
- **No CPU cost**: the change is entirely in the two shader stages, and `bench_lights` runs no shader at all.

## Deviations

- **A world-oriented billboard, the reference's other proposed fix, was tried and rejected on measurement.** It
  makes the flame a leaning plate at intermediate angles, which is worse than an upright tongue there; the record
  and the capture are above so the next attempt does not repeat it.
- **The cross-section is drawn inside the upright quad rather than as a second quad.** A second horizontal quad
  would cost a second draw and a second material, and would need its own depth and blend ordering against the coal
  bed it sits in — for a shape that is one `length()` away in the shader.
- **The flame is still not a volume.** Round 7 gave its outline lobes and its body radial streaks (edge `smoothstep(0, 0.26)`), but from above it is still one lobed disc with a hot core and a foreshortened tongue, not layered fire; the interior streaks are subtle.
- **Round 8 (a three-disc stack) improved the overhead fire's read but stayed NOT YET and was not shipped**; the patch is kept in `docs/attempts/`.
- **The camera's elevation is taken from the ray to the plume's centre** (`view.world_from_view[3]`), which is exact for a
  perspective camera. For an orthographic camera that translation is the near/far midpoint, so a flame well off the view axis
  reads an elevation a little off its true one; the error is the flame's offset over the camera's distance, small at any
  far plane a game uses (the occlusion example's is 6000).
- **Hard switch at 50° to the camera-facing quad.** The tongue is sampled so that its image is the upright quad's to within
  a pixel, and the cross-section's weight is 0 at 50°, so there is no visible step; but the geometry changes at that angle.
- **The `wesl-check` stub fix** (an earlier iteration's). The stub declared only `world_from_view`
  and `clip_from_world`; a shader reading `view.world_position` failed to compile rather than passing silently, so the
  gap was safe here, but it is a real hole in the tool and the next shader change will read that field at once. The
  stub now carries `world_position` and `exposure` in the real struct's order, with the values non-zero.

## Pending index

Read live from the running library on 2026-10-26; every source above is in the corpus and indexed. Gaps that would
move a default to a citation:

- A measurement of how a **real flame's apparent length and width change with the viewing elevation** — a flame
  photographed or rendered from a known series of angles — would replace `OVERHEAD_START`, `OVERHEAD_FULL` and the
  `squash` floor with numbers. The corpus has flame dynamics (Xia & Zhang) but no view-dependent appearance study.
- Section 13.6.3's own citation for the cross-section workaround is Klint [908] on SpeedTree, which the corpus does
  not hold; the technique is confirmed by the text but its numbers are not available here.
- `flame.md`'s long-standing gap is unchanged: the simplex-noise and hash provenance.

## What great looks like

Judged at full frame and at 4× on the fire, at every camera the crate's own showcase and a tactical game use:

- **A fire at every angle.** At 0, 30, 45, 60, 75 and 85 degrees above a brazier there is a warm plume where the
  coals are: a tongue from the side, turning to a round puff overhead, and never a bare lit slab.
- **No pop as the camera rises.** A smooth sweep from 30 to 85 degrees shows the silhouette turning continuously;
  no frame where the flame jumps size, jumps brightness or blinks.
- **The footprint stays on the fire.** The disc is centred on the coal bed, not sliding off it as the elevation
  changes; its bright core is over the coals.
- **The same fire from the side.** At the showcase camera the flame is pixel-identical to before this change.
- **Cheap.** One `asin` and a few vector operations per vertex, and one extra fbm per fragment only where the cross-section is
  drawn (camera past 50°); no second draw, no second material, no new instance data.

### Round 10 — judging exposure (iteration 156)

No shader change. `courtyard` gains `LIGHTS_EV100` (default 6). Captured the unmodified shader at 76° and 85° at EV 6, 9 and 11 (`/tmp/l156/cmp.png`): at EV 9 the ground reads as dark stone with a warm pool, and the overhead flame is a small ragged orange burst with a pale core on the dark coal bed, so its shape (a cauliflower, not tongues) can finally be judged against its surroundings; at EV 11 the stone is lost. Use `LIGHTS_EV100=9` for the next overhead attempt, which should be a tongue-quad construction, not disc constants.

### Round 11 — the edge-on tongue (iteration 157)

At `LIGHTS_EV100=9`, straight down (eye 5.3,9,5), the flame reads as a ragged burst with a pale core on a dark bed, and one defect shows at 24x zoom: a faint one-pixel horizontal slab right of the core, past the ellipse. It is the upright tongue seen edge-on: `rise` is clamped by `OVERHEAD_COS_FLOOR`, so the widest row is drawn as a line, and `OVERHEAD_TONGUE` leaves 40 % of it. Fix: the tongue's factor is also multiplied by `1 - smoothstep(0.970, 0.996, sin(elevation))` (design default; 0.970 is sin 76°, where every earlier term is already settled). Captures `/tmp/l157/{before,after}/`: the slab is gone at 90°, 76° differs by 2 pixels (GPU noise), 60° by 0. Not claimed: the burst is still not tongues; that stays NOT YET.

### Round 12 — the mid-blend dip (iteration 207)

**Defect.** Sweeping a camera 4 m from a brazier (`/tmp/l207/before`, seed 7, frame 90, 1280x720) the flame is a vivid tongue at 45° and 55° and a pale haze at 66°-70°: the tongue is dimmed by `OVERHEAD_TONGUE · topdown`, which is already 0.4 · 0.6 = 24 % gone at 66° while the cross-section has barely arrived. Mean grey of the tongue window (120x70 px above the coal centre): 134.5, 129.8, 118.1, 112.9, 111.4 at 55°, 60°, 66°, 70°, 76°, a fall of 11.7 and then 5.2 DN per step.

**Sources, all quoted above:** craft — Doğaner (`10.7456_tojdac.1674124`), a practical must stay visibly where its light is, which a fire that dims as the camera rises does not; production — Gregory §11.4.3 and `s2011-advances-pdf-slides`, shipped fire layers hide any one layer's loss of face; technical — Akenine-Möller et al. §13.6.3, an axial billboard loses area as `cos(elevation)`, which the quad's facing switch at 50° already undoes, so the brightness it keeps is not an artefact to dim away.

**Model.** The tongue's dimming is `OVERHEAD_TONGUE · topdown⁵` instead of `· topdown`: nothing below ~60°, the full 0.6 only at 76° and past, where the disc is fully in. Design default, judged in captures. Frames at or below 55° and at 76° and past are unchanged (`magick compare` AE 0.01 px at 45°, 55°, 85°; 0.7 px at 76°).

**Result.** Same window, same seed: 131.3, 123.6, 116.2, 113.2, 111.4 at 60°, 66°, 70°, 73°, 76° (steps 7.7, 7.4, 3.0, 1.8), so the fall is spread evenly instead of landing between 60° and 66°. At 66° the tongue's mean is +5.5 DN over round 11's. Still NOT YET as a whole: the flame is lower in contrast than at 45° because the floor behind it is lit orange, and a faint stub of the tongue's tip stands above the bowl's rim at 60°-66°.

### Round 13 — the core out-shines the floor it lit (iteration 208)

**Defect.** From 60° up the flame is an additive (alpha 0) layer over ground the fire itself has lit orange (about 130-150 DN), so it cannot darken that ground and its contrast falls to the difference of two oranges; at 76° the cream core also took a sage-grey cast where `CORE_YELLOW` summed with the coal bed (`/tmp/l208/before/e76.png`, 3x zoom of the core). Craft: Doğaner (`10.7456_tojdac.1674124`), a practical must stay visibly where its light is; production: Gregory §11.4.3 and `s2011-advances-pdf-slides`, shipped fire keeps its hot core over what it lights; technical: RTR4 §13.6.3, the cross-section stands in for the flame from above, so it carries the flame's brightness. All quoted in the sections above.

**Model.** `colour *= 1 + OVERHEAD_CORE · topdown · smoothstep(0.30, 0.80, heat)` with `OVERHEAD_CORE = 0.9`: extra emission in the hot core only, rising with the overhead blend, exactly 1 at or below 50° (frames at 45° are pixel-identical, `magick compare` AE 0.0013 over 1280x720 from GPU noise). Design default, judged in captures.

**Result.** Core-window mean grey (60x60 px, seed 7, frame 90) at 60°, 66°, 76°: 148.1, 137.6, 131.4 before; 149.2, 141.0, 139.6 after; peak 225 to 250 at 76°. The 76° core is cream-yellow instead of grey-green. Still NOT YET as a whole: a hard horizontal line crosses the coal bed where the tongue's seat meets the disc, and the 60°-66° plume stays pale against the lit floor.
