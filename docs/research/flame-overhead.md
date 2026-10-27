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

## What was attempted, and why it was reverted

**The shader change is reverted this iteration.** The defect, its cause and the measurements stand; the visual fix
did not reach GREAT, so the crate is back at the commit this pass started from and `src/flame.wesl` is unchanged.
Five rounds were spent, which is the skill's ceiling. What each round established is recorded here so the next
attempt starts from the fifth, not the first.

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

### Why round 5 is still NOT YET

Judged at 4× against this note's `## What great looks like`:

- **A fire at every angle — PARTLY.** The brazier's disc is drawn and measurable, and the torch's is a visible puff.
  But at 85° the brazier's disc is **swamped by its own coal bed**, which is a clipped white slab 32×9 px in the middle
  of it: the flame's added light peaks at 17 DN against a bed already at 255/244/227, so the thing a player reads at
  that camera is a white rectangle with a slightly warmer halo. That is the *second* defect this iteration found, and
  it is not the flame's — `examples/courtyard` sets the coals' `emissive` to `blackbody × 1.4` as a constant, and at
  85° the ground and box faces are bright enough that the slab is the brightest thing in frame; at the crate's own
  hero camera the same slab never clips at all (0 pixels ≥ 230/240). Fixing it means the coal bed's emissive has to
  stop being a constant, which is a scene-and-exposure question, not a shader one.
- **No pop as the camera rises — NOT YET.** Between 50° and 76° the silhouette is a blend of tongue and disc, and at
  60–75° the *drawn* shape is still the tongue's teardrop with the disc fading in over it: the capture at 75° shows a
  teardrop lying diagonally on the brazier rather than a puff. The blend needs the tongue to shrink as the disc
  arrives, not to sit at full size behind it.
- **The footprint stays on the fire — PASS.** The disc is centred on the coals; the added-light bbox straddles them.
- **The same fire from the side — PASS, measured.** At 0° and 45° the frame is **pixel-identical** to the base build
  (`magick compare -metric AE` = 0 and 0.58 pixels-of-one-channel respectively); `OVERHEAD_START` at 50° makes every
  term exactly its old value below it.
- **Cheap — PASS.** One `asin`, one `sin` and one `length()`; one flat varying; no second draw.

So the verdict is **NOT YET**, and per the skill the code is reverted rather than shipped.

### What the next attempt should do differently

1. **Fix the coal bed first.** A clipped white slab at the camera the flame just became visible from is the bigger
   defect, it is in the example rather than the shader, and it hides whatever the shader does. The measured line: the
   flame must add light to a bed that is not already at the ceiling (`min channel ≥ 245` at 85°, 0 px at the hero
   camera).
2. **Shrink the tongue as the disc arrives.** `body` and `body_round` must be a *dissolve* weighted by `cross`, not a
   `mix` of two full-size silhouettes; at 75° the tongue's teardrop is still full size underneath the disc.
3. **Keep round 5's arithmetic** — the `1/cos(e)` stretch with the `R·sin(e)` semi-axis is what made the disc the
   right size in the world, and it is the part worth carrying forward.
4. **Measure with added light, not pixel counts.** The coal slab clips and is static, so any warm-pixel or
   temporal-variance metric is dominated by it; frame-minus-base in the bed's window is the only honest number here,
   and it is what the numbers above use.

## Parameters

Every number below is from the reverted attempt: it is the tuning the next pass should start from, not a claim about
shipped behaviour.

| parameter | value | source |
|---|---|---|
| `OVERHEAD_START` | 0.873 rad (50°) | design default: below a third-person camera's usual pitch of ~50° the flame is the upright tongue exactly as before, so no ground-level frame changes |
| `OVERHEAD_FULL` | 1.326 rad (76°) | design default: the elevation at which an upright quad has lost `cos 76° = 76 %` of its area, calibrated in captures |
| `squash` floor | 0.18 | design default: `sin(10°)`, keeps the ellipse from becoming a line at a camera that is nearly level |
| disc radius `R` | `FLAME_HALF_WIDTH = 0.20` of the flame's height, rippled `±25 %` by `n1` | the plume's own widest half-width, already in the shader; the ripple is the existing turbulence |
| overhead quad aspect | 0.9 | design default: `4·FLAME_HALF_WIDTH = 0.8` plus a margin for the rippled rim |
| overhead rim cooling | `1 − 0.45·radial` | design default, from the tongue's `1 − 0.85·u` reduced because a cross-section has no long axis to fall off along |

## Validation targets

- **At 4 m and 85 degrees over a brazier, the flame adds light** where the baseline adds none: frame-minus-base in the
  bed's window, not a warm-pixel count. Round 5 reached **2962 px above 3 DN in an 81×57 bbox** against round 3's 143
  px; the bar for GREAT is that the added light is *readable*, which needs the bed itself not to be clipped (below).
- **At 4 m and 85 degrees over a torch, the flame is a compact puff** rather than the baseline's **1-pixel sliver**.
  Met in round 5.
- **Ground level is unchanged, pixel for pixel**, at 0 / 15 / 30 / 45 degrees: `OVERHEAD_START` is 50 degrees, so
  `cross` is exactly 0 and every term it multiplies is exactly its old value. This is the gate that keeps the
  change from costing anything at the camera the crate's own showcase uses.
- **No clipping, and no clipped backdrop.** The flame's own pixels stay under the ceiling at every angle, *and* the
  surface it is drawn over is not already at it. Measured: `examples/courtyard`'s coal bed reaches
  `min channel ≥ 245` over 74 px at 85° and 0 px at the crate's hero camera, so this is the gate the reverted attempt
  failed and the first thing the next pass must fix.
- **Flicker and the catch are untouched**: the brightness byte and `kindle` path are not in this change.
- **No CPU cost**: the change is entirely in the two shader stages, and `bench_lights` runs no shader at all.

## Deviations

- **A world-oriented billboard, the reference's other proposed fix, was tried and rejected on measurement.** It
  makes the flame a leaning plate at intermediate angles, which is worse than an upright tongue there; the record
  and the capture are above so the next attempt does not repeat it.
- **The cross-section is drawn inside the upright quad rather than as a second quad.** A second horizontal quad
  would cost a second draw and a second material, and would need its own depth and blend ordering against the coal
  bed it sits in — for a shape that is one `length()` away in the shader.
- **The flame is still not a volume, and the blend is the reverted attempt's worst flaw.** Between 50 and 76 degrees
  the silhouette was a `mix` of a full-size tongue and a full-size disc, so at 75° the capture shows a teardrop lying
  diagonally across the brazier with a disc fading in over it, rather than one shape turning into another. The fix is
  a dissolve weighted by `cross` — the tongue shrinking as the disc arrives — and it is unbuilt.
- **The `wesl-check` stub fix is kept even though the shader change is not.** The stub declared only `world_from_view`
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
- **Cheap.** One `asin`, one `sin` and one `length()` per vertex and per fragment in the region the flame covers;
  no second draw, no second material, no new instance data.
