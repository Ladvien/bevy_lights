# The light follows the lean

`flame-wind.md` leans every flame downwind and records, under Deviations, that "the light's position ... does not move with the lean". A pooled light stays on the vertical axis above the foot while the flame it is the light *of* hangs to one side. This note closes that gap: the pool's light moves to where the drawn flame's middle now is.

## Sources

| source | kind | use |
|---|---|---|
| `10.7456_tojdac.1674124` (Doğaner 2025) | craft | A practical's light must be motivated by what is visible: "the light has to come from the visible source". A light left upright beside a leaning flame has the wrong source. |
| `s2011-advances-pdf-slides` (the fire talk) | production | The light belongs to the fire's body: "Rendering is working quite hard to try to hide the particle nature of the system", i.e. the light and the drawn fire agree. |
| `docs/research/flame-wind.md`, Wen et al. 2019 and DGMR eq. 7-9 | technical | The tilt `flame_tilt(speed, diameter)`; the flame turns rigidly about its foot, so its middle (where `flame_light_offset` puts the light) is at `(h/2)(sin θ · wind, cos θ)`. |

Quote (Doğaner, via `docs/research/flame-wind.md`'s source list and `lighting-look.md`): a light with no visible source is a defect; here the source is visible and the light is the part that is wrong.

## Craft principle

A cinematographer places the fire's light where the fire is. A flame bent by a draught throws its warm pool and its shadow from where the flame's body is, not from where its foot is, so the pool of light slides downwind of a bowl and the shadows of the figures beside it lengthen the other way.

## How shipping games do it

The same fire talk treats the light as part of the fire effect, authored with it. The adopted shape: the light is a consequence of the flame's pose, so it is computed from the same `flame_tilt`, never set independently.

## Model adopted

`lean_light_shift(height, speed, diameter, toward) = (h/2) · (sin θ · toward.x, cos θ - 1, sin θ · toward.y)` with `θ = flame_tilt(speed, diameter)`. It is the move from the upright middle `(0, h/2, 0)` to the rotated middle. The pool adds it to a winner's position when the winner's entity carries a `FlameSite` and a `FlameWind` resource exists. The diameter is the candidate's own, else the site's, else `DEFAULT_FIRE_DIAMETER`: the same precedence the flame itself uses. Zero wind, no site or a non-finite wind add exactly zero.

## Parameters

None new. Every number is `flame_tilt`'s and the flame's own height. Cost: two component reads per *winner* (at most `pool.size`), none per candidate.

## What great looks like

A brazier in a crosswind throws its pool a hand's breadth downwind with the flame, key light and visible source agreeing; in still air the picture is bit-identical.

## Validation targets

- Zero wind: shift is exactly `[0, 0, 0]`.
- Shift horizontal component is `(h/2) sin θ` along the wind and vertical is non-positive (the middle sinks as the flame lies down).
- A pooled candidate on a `FlameSite` entity gets the lean; one with no `FlameSite` does not.

## Deviations

Design default: the light leans by the full flame tilt although the flame fades its lean toward straight overhead (`flame-wind.md`, lean from above); the shift is under half a flame height. A candidate that is not on the `FlameSite` entity (a site linked by `FlameSite::fire`) does not lean.

## Pending index

No new documents; see `flame-wind.md` (no production source on wind-bent flames).
