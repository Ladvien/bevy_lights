# A brazier is a bowl: the fuel sits below a rim

`examples/courtyard` draws each brazier as a solid 0.8 × 0.7 × 0.8 m cuboid with a 0.34 m coal
patch laid on its top face. Measured on the iteration 135 baseline (`/tmp/l135/before/C.png`, the
elevated camera): the top face is a flat lit plate, the coals are a postage stamp on it, and the box
has no rim, no depth and no underside, so the one prop every frame of the showcase is built around
reads as a crate with a sticker. This is an **example** change (the crate's `src/` draws no
geometry); it matters because the courtyard is the crate's shop window and its capture scene, and
because a host copying it copies the crate.

The question is the loop's long-standing backlog item "a bowl that shades its own rim": where the
fuel sits in a real fire basket, and what that does to the light on the basket itself.

## Sources

| source | what it is | use |
|---|---|---|
| `10.2478_bsmr-2022-0020` — Huttunen, *Ecological approach to cinematographic lighting of the human face* (2022) | why light from below reads as unnatural, and where the eye expects a source | the craft rule: a fire *inside* a vessel lights the vessel's inner wall from below and its rim from within; the outside of the bowl below the rim is the shaded side |
| `10.32657_10220_48312` — *Strategies of colour and lighting design in the cinematography of Christopher Doyle* | chiaroscuro and specular placement | the lip is the one place a prop of this size can carry a highlight: Doyle's "specular lighting … concentrate and highlight the specific area" |
| `s2011-advances-pdf-slides` — SIGGRAPH 2011 *Advances*, Fire/Flamethrowers/Thrusters | production fire | production budget goes on hiding that the fire is made of discrete elements: "Rendering is working quite hard to try to hide the particle nature of the system." A flat plate under a flame is the same failure on the prop |
| `PBR3_12_Light_Sources` — *Physically Based Rendering* 3rd ed., ch. 12 | blackbody emitters | "**Blackbody emitters are perfectly diffuse; they emit radiance equally in all directions.**" A coal bed is therefore as bright from any angle, so a rim *above* it is the only thing that can hide it, and that is the occlusion the bowl adds |

## Craft principle

Huttunen records what the eye does with light from below, in the cinematographer's own terms:

> "know-how of filmmaking stipulates that underlighting can be unnatural and frightening … and it can
> make a character look 'weird, dangerous, or at best untrustworthy'"

A fire is the one source that is *allowed* to light from below, because it is visible and motivated;
the craft's job is then to make its position unambiguous. A fire standing on a flat plate is
ambiguous (the plate is lit from above by the light that the flame itself casts); a fire sitting in a
vessel is not: the **inside wall catches the light from the bed's glow and the flame, the lip is the
brightest line on the prop, and the outside below the lip falls off into shade**. That gradient is
what tells a viewer which way the heat goes.

## How shipping games do it

The 2011 *Advances* fire talk spends its effort on denying that the effect is made of parts:

> "Rendering is working quite hard to try to hide the particle nature of the system."

A game brazier is a modelled prop (a dished bowl on a stem or tripod) with a coal or log mesh set
below its rim and the light parented above it; the bowl is left a non-caster or a cheap caster
because a hard shadow from a lip at a point light's own height lands on the floor as a dark square.
The crate's courtyard already learnt that trap (`NotShadowCaster` on the box).

## Model adopted

One procedural lathe mesh, no assets: a revolved profile in three strips — the outer silhouette
(a flared foot, a narrow stem, a bowl widening to the lip), the flat lip, and the inner wall down to
the bed. Normals come from each strip's own profile edges, smooth along a strip and split at the lip's
two corners so the rim stays a sharp line.

The coal bed drops 0.05 m, so its top face sits **below** the lip: at the camera heights the
showcase uses (1.6 m out to top-down) the near rim hides the bed's near edge, and the bed reads as
inside something. The flame's base, the coal light and the pool's centre do not move.

## Parameters

| value | number | source |
|---|---|---|
| lip height | `BRAZIER_TOP` (0.70 m, unchanged) | the existing constant the light, the flame and the pool's maths are written against |
| bed top | 0.05 m below the lip | design default: the lowest that lets the rim occlude at 1.6 m while the 0.575 m light-to-bed figure in `fuel-bed.md` moves by under 10 % |
| lip outer / inner radius | 0.44 / 0.38 m | design default: the old box's half-width was 0.40 |
| stem radius | 0.12 m at 0.30 m | design default |
| segments around | 32 | design default: a 0.44 m circle's facets are under 0.09 m, sub-pixel past 10 m |
| albedo | unchanged (0.10, 0.09, 0.09) | the existing `bowl_material` |

No crate default changes, so the CHANGELOG entry is an example change only.

## Validation targets

- From the elevated and 1.6 m cameras the bowl reads as a bowl: a lit lip line, an inner wall
  brighter than the outer wall below the lip, the bed's near edge hidden by the near rim.
- No hard dark square on the ground (the bowl stays a non-caster).
- No coal-bed clipping regression: peak `R` over the bed no worse than the baseline's.
- The flame still rises from the bed's centre and the pool's key/fill read is unchanged to within
  noise on the unrelated angles.
- CPU bench unchanged (the geometry is example-only; no library path is touched).

## Deviations

None from the crate's defaults. The bowl's dimensions are design defaults, listed above.

## Pending index

None: every source is already in the corpus. A craft source specifically on fire baskets (a gaffer's
account of a practical fire on a set) was searched for in the corpus (`brazier`, `fire basket`) and
not found.

## What great looks like

A dark iron-grey bowl on a stem whose lip is the brightest line on the prop, whose inside is a warm
gradient from the bed's glow up to the rim, and whose outside under the lip is almost black. The coals
glow *in* it, partly hidden by the near lip, and the flame comes out of the opening.

## Iteration 136: what shipped

Round 1-5 of iteration 135 reached a bowl that read but a lining that was one tone and a bed that was a hard-edged square. Two changes finished it, both example-only:

- **The bed is a disc.** `Circle(bed/2)` laid flat at the bed's top (no side wall, so no striped crescent on its rim) instead of a 0.34 m square, which sat in a round bowl as a stamp. Same texture, same size, same flicker/flame diameter.
- **The lining glows from the bed up.** The soot material has an emissive texture over the lining's profile (`v` 0 at the lip, 0.5 at the bed): radiance of a dull ember (`coal_radiance(0.35)`) times 0.8 at the bed's edge, dying with an e-fold of 0.15 of the profile toward the rim. Craft reason: a heated wall next to coals is the one gradient that tells which way the heat goes (see `## Craft principle`). **Design default**, not from a source; the 0.8 and 0.15 were chosen by eye at the 2.2 m camera and the lining reads rim-dark to bed-warm.

Deviation: `## Model adopted` said the coal bed was a box dropped 0.05 m; it is a disc at the same height. No crate default changed.
Measured (1280x720, `LIGHTS_FRAMES=60`, 6 cameras): mean luma before/after A 80.1/80.8, C 113.6/115.4, D 88.0/88.9, F 43.2/43.9 — the pools are unchanged; B rises 77.4 → 83.5 because the box face is gone.
