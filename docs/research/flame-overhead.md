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

## Model adopted

**The quad stays upright and keeps facing the camera about the vertical axis; the silhouette inside it changes.**

Tilting the quad's own `up` axis toward the horizontal-away-from-camera — the literal reading of "turn an axial
billboard into a world-oriented one" — was tried first and is **worse than doing nothing**, so the record of it is
kept here rather than in the code. It draws the tongue on a plate leaning away from the camera: at 60 degrees the
flame becomes a broad flat wash across the brazier's top with its tip smeared across the coal slab, visible in
`/home/ladvien/l19/elev5/d60.png`. Two rounds of captures were spent on it before it was abandoned. The reason is
geometric rather than tunable: a vertical flame has no extent *along* the direction the camera is looking, so
rotating its quad into that plane can only foreshorten it into a smear.

The model that works is the projection of the fire's real cross-section, drawn inside the upright face:

- **`cross`** — the camera's elevation over this flame, `asin` of the unit vector to the camera's `y`, mapped
  through `smoothstep(OVERHEAD_START, OVERHEAD_FULL, ·)` to 0 from the side and 1 overhead. One flat varying.
- **`squash`** — `sin(elevation)`, floored at 0.18: the vertical squash of the horizontal disc the camera sees.
  One flat varying. A horizontal disc of radius `R` projects to an ellipse `R` wide and `R·sin(e)` tall, so this
  is what the geometry does, not a fudge.
- **The disc** — `radial` is the ellipse's normalised radius in the quad's own uv, `(uv.x − 0.5)·quad_x/R` across
  and `(uv.y − 0.5)/(R·squash)` up, with `R = FLAME_HALF_WIDTH` (the plume's own half-width, the widest part of
  the flame the crate already draws) rippled by the same `n1` the tongue's edge uses. `body_round = 1 − radial`.
- **The quad narrows and widens, never tilts** — `FLAME_ASPECT` 0.6 at the side to `FLAME_OVERHEAD_ASPECT` 0.9
  overhead, because the disc's diameter `4·FLAME_HALF_WIDTH = 0.8` of the flame's height has to fit inside it with
  a margin for the turbulence on its rim. Its height never changes, so the flame's footprint stays anchored on the
  fire.
- **The base band and the low-down heat fade with `cross`** — both are properties of a flame's *foot*, which a
  cross-section does not have. Left on, the `smoothstep(0.0, 0.06, y)` seat erased the entire disc, because every
  fragment of a horizontal cross-section drawn in an upright quad is at the quad's base's height.
- **The heat ramp becomes radial overhead** — `1 − 0.45·radial` instead of `1 − 0.85·u`: hottest in the middle,
  cooling to the rim, which is what a plume's cross-section looks like rather than a tongue's yellow core.

Nothing else changes. The quad's placement, the catch (`took`), the brightness byte and the clock are untouched.

## Parameters

| parameter | value | source |
|---|---|---|
| `OVERHEAD_START` | 0.873 rad (50°) | design default: below a third-person camera's usual pitch of ~50° the flame is the upright tongue exactly as before, so no ground-level frame changes |
| `OVERHEAD_FULL` | 1.326 rad (76°) | design default: the elevation at which an upright quad has lost `cos 76° = 76 %` of its area, calibrated in captures |
| `squash` floor | 0.18 | design default: `sin(10°)`, keeps the ellipse from becoming a line at a camera that is nearly level |
| disc radius `R` | `FLAME_HALF_WIDTH = 0.20` of the flame's height, rippled `±25 %` by `n1` | the plume's own widest half-width, already in the shader; the ripple is the existing turbulence |
| overhead quad aspect | 0.9 | design default: `4·FLAME_HALF_WIDTH = 0.8` plus a margin for the rippled rim |
| overhead rim cooling | `1 − 0.45·radial` | design default, from the tongue's `1 − 0.85·u` reduced because a cross-section has no long axis to fall off along |

## Validation targets

- **At 4 m and 85 degrees over a brazier, the flame is visible**: warm pixels above the coal bed's top row, and a
  round silhouette, where the baseline has a bare lit slab. Measured by temporal variance in a window on the bed
  (the bed is static, so anything that moves is flame).
- **At 4 m and 85 degrees over a torch, the flame is visible** and reads as a small ellipse, where the baseline
  shows nothing at all.
- **Ground level is unchanged, pixel for pixel**, at 0 / 15 / 30 / 45 degrees: `OVERHEAD_START` is 50 degrees, so
  `cross` is exactly 0 and every term it multiplies is exactly its old value. This is the gate that keeps the
  change from costing anything at the camera the crate's own showcase uses.
- **No clipping.** The flame's own pixels stay under the ceiling at every angle; the coal slab is already the
  brightest thing in the frame and the flame must not add to that.
- **Flicker and the catch are untouched**: the brightness byte and `kindle` path are not in this change.
- **No CPU cost**: the change is entirely in the two shader stages, and `bench_lights` runs no shader at all.

## Deviations

- **A world-oriented billboard, the reference's other proposed fix, was tried and rejected on measurement.** It
  makes the flame a leaning plate at intermediate angles, which is worse than an upright tongue there; the record
  and the capture are above so the next attempt does not repeat it.
- **The cross-section is drawn inside the upright quad rather than as a second quad.** A second horizontal quad
  would cost a second draw and a second material, and would need its own depth and blend ordering against the coal
  bed it sits in — for a shape that is one `length()` away in the shader.
- **The flame is still not a volume.** Between about 50 and 76 degrees the silhouette is a blend of a tongue and a
  disc, which is the correct thing for a *drawing* but is not what a real flame does: a real one is a plume whose
  length foreshortens while its cross-section grows. That is what `cross` approximates, and a three-quad
  cross-tree would be the next step if the blend ever reads as a cheat. It does not in the captures.
- **The `wesl-check` stub for `View` was incomplete and this iteration found it.** The stub declared only
  `world_from_view` and `clip_from_world`; a shader reading `view.world_position` failed to *compile* rather than
  silently passing, so the gap was safe here — but the same gap would have let a mis-typed field through had the
  name happened to exist. The stub now carries `world_position` and `exposure`, in the real struct's order.

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
