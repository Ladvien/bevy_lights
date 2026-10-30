# Fire reach: how far a kind of fire lights the ground

`FireKind` (see [`fire-kinds.md`](fire-kinds.md)) names a fire's bed, depth and flame height, but not how much light it puts out. An editor's picker that drops a `Candle` and a `Bonfire` then has to guess two lumen budgets, and a guess is how the baseline got 60 000 lm at 0.9 m. `FireKind::reach()` is the one size a host needs: the metres of ground the kind lights to its pool's edge. `FireKind::lumens(lux, over)` turns it into a budget through the existing `fire_lumens`.

## Sources

| source | what it is | use |
|---|---|---|
| `10.23856_6105` — *Harmony of light and shadow*, Table 1 (source size : distance; corpus, quoted in [`lighting-look.md`](lighting-look.md)) | craft | a flame lights a patch in proportion to its own size: 1:3–1:6 is "soft with smooth transitions", a 0.35 m flame at 2 m is 1:6 |
| `s2011-advances-pdf-slides` — SIGGRAPH 2011 *Advances*, Fire / Flamethrowers / Thrusters (corpus) | production | emission belongs to each fire's own body, so a fire's light is a property of the fire, not of the scene |
| `book_real_time_rendering_4th_akenine` — *Real-Time Rendering* 4th ed., photometry chapter (corpus) | technical | a point light's illuminance falls with the inverse square of distance; `fire_lumens` inverts exactly that (Bevy maps lumens to Φ/4π candela) |

## Craft principle

[`lighting-look.md`](lighting-look.md) records the craft rule the courtyard was fixed with: "a 0.35 m flame lighting a 4 m patch is the 'flame too small for the ground it lights' defect" and `10.23856_6105` Table 1 gives the ratio that reads: "sufficiently contrasty … with fuzzy shadow contours" for 1:6, "soft with smooth transitions" for 1:3–1:6. A kind's reach is therefore a multiple of its flame height in that band, not an unrelated number.

## How shipping games do it

The 2011 *Advances* fire talk: "Rendering is working quite hard to try to hide the particle nature of the system." Each fire archetype carries its own light with it (a candle, a torch, a bonfire are authored as different lights, not one light scaled). The same shape is adopted: the kind owns the size.

## Model adopted

```
FireKind::reach()                 metres of ground lit to the pool's edge (const)
FireKind::lumens(lux, over) = fire_lumens(lux, reach, over)
FireKind::light(lux, over)  -> Flicker   (flicker(lumens))
```

`lux` is the host's own look number (the illuminance wanted at the pool's edge, in its exposure) and `over` the light's height above the ground, exactly as for `fire_lumens`; the crate converts no units. Lumens therefore grow as the square of the reach (inverse square), so a bonfire's budget is the candle's times (9/1)² = 81 at the same edge lux.

## Parameters

| kind | reach (m) | reach / flame height | provenance |
|---|---|---|---|
| Candle | 1.0 | 25 | design default; `fire_lumens` clamps reach to 1 m, and a candle lights a tabletop about that far |
| Lantern | 1.5 | 12.5 | design default |
| Torch | 2.0 | 5.7 | the courtyard's `TORCH_POOL`; inside Table 1's 1:3–1:6 |
| Brazier | 4.0 | 3.6 | the courtyard's `BRAZIER_POOL`; inside Table 1's 1:3–1:6 |
| Campfire | 5.0 | 7.1 | design default |
| Bonfire | 9.0 | 4.5 | design default; inside Table 1's 1:3–1:6 |

Candle and Lantern are flames small enough that the 1 m floor of `fire_lumens` dominates; they are over the table's band by that floor, not by choice.

## Validation targets

- Reach strictly rises Candle → Bonfire and is at least `fire_lumens`' 1 m floor.
- `lumens(lux, over)` equals `fire_lumens(lux, reach, over)` and grows by the square of the reach ratio at equal lux.
- Brazier and Torch reach are exactly the courtyard's two pool constants, so the showcase is unchanged.
- `light(lux, over).mean()` is `lumens(lux, over)`.

## What great looks like

A picker where a bonfire lights a courtyard, a torch a doorway and a candle a table, with no lumen number typed by the author, and with the flame never visibly too small or too large for the ground it lights.

## Read back: `fire_lux` and `fire_reach`

The same inverse-square law (`E = Φ / 4π / slant²`, `book_real_time_rendering_4th_akenine`) read the other way, for an editor that draws a placed fire's reach: `fire_lux(lumens, reach, over)` is the ground illuminance, `fire_reach(lumens, lux, over)` the radius of the iso-lux circle. Both clamp as `fire_lumens` does, so they invert it exactly for reach >= 1. No new constant.

## Flat ground: `ground_lux` and `ground_reach`

`fire_lux` and `fire_reach` are the illuminance on a surface square-on to the light, the law a light is *sized* with. Ground is horizontal, and a surface receives `E = I cos θ / d²` (Lambert's cosine law, `book_real_time_rendering_4th_akenine`, the same `N·L` the renderer shades stone with), with `I = Φ / 4π` Bevy's candela for a point light (`bevy_pbr-0.20.0/src/render/light.rs`) and `cos θ = over / slant` for ground under a light `over` high. So `ground_lux(lumens, distance, over) = Φ/4π · over / (distance² + over²)^1.5`, and `ground_reach` solves it for `distance`: `slant² = (I·over / lux)^(2/3)`. At 4 m out and 0.9 m up the ground gets 0.22 of the square-on number, so an overlay painting `fire_lux` would draw a lit circle about 2x too wide at 12 lx. The craft reading is a gaffer's: a lamp hung low throws a pool on the floor that is bright under it and falls off faster than inverse square, which is why a hung practical is "a pool of light" and a tall one a wash. No constant is added and no existing function or default changes; sizing (`fire_lumens`, `FireKind::light`) stays square-on.

A light at or below the ground (`over <= 0`) puts no light on it: both functions answer 0, there is no clamp to a minimum height (a fire's height is the host's).

## A linear-ramp footprint: `ground_ramp`

A host that keeps a coarse illuminance grid (a stealth or AI tile map; `bevy_light_grid`'s `bake` takes `(cell, intensity, range)` and adds `intensity * (1 - dist/range)` to each visible cell inside `range`) has no way to say "this fire" in those terms, so it guesses a pair per fire and the guess drifts from what the renderer draws. The obvious pair, the peak under the light and the `ground_reach` of a floor lux, is *wrong* (checked 2026-10-30 before building it): the true ground curve `I·h/(d²+h²)^1.5` is a sharp spike over a long tail, and a ramp from its peak to zero at the floor circle over-reads the middle of the pool by 5-10x (a 12 lx-at-4 m brazier, `over` 0.9: ramp/true = 4.8 at half reach). A flux-preserving ramp is no better (3.8-6.6x at half reach). A linear ramp cannot follow inverse-square, so the fit is instead made where a grid is *read*: a tile is called lit above a dim threshold and bright above a bright one.

`ground_ramp(lumens, over, dim_lux, bright_lux) -> (peak, reach)` returns the line through the two true points `(ground_reach(dim_lux), dim_lux)` and `(ground_reach(bright_lux), bright_lux)`, so the ramp `peak * (1 - d/reach)` crosses *exactly* the dim and the bright circle the renderer's ground shows, and is the chord between them. Outside the bright circle it under-reads the spike (a creature standing at the flame is not "brighter than bright" in a three-state map, so this is the safe side); beyond the dim circle it keeps falling to zero at `reach`, past the dim circle. `peak` is in lux; `reach` in world units (the host divides by its cell size and scales `peak` to its own grid units, so the crate converts nothing and names no grid).

Craft principle, `10.23856_6105` Table 1 (quoted in `lighting-look.md`): "soft with smooth transitions" is the 1:3-1:6 ratio, i.e. the pool's edge is where the eye decides lit from dark, and that edge is what a stealth map must agree with. Production, `s2011-advances-pdf-slides`: a fire's emission is a property of the fire, so its footprint comes from its own lumens. Technical, `book_real_time_rendering_4th_akenine`: the inverse-square and cosine laws behind `ground_lux`.

Degenerate inputs give `(0.0, 0.0)`: a light at or below the ground, a non-positive or `NaN` threshold, `bright_lux <= dim_lux`, or a bright circle that does not exist (`bright_lux` above the spot under the light).

## Deviations

None: Brazier and Torch are the shipped constants. The other four are design defaults, flagged above.

## Pending index

No new documents. A measured luminous-flux table for wood fires is not in the corpus (searched 2026-10-29: distill returned only the photometry chapters; `paper_search` returned pool-fire radiative-fraction papers that are behind publisher paywalls, e.g. `10.1016/s0010-2180(96)00098-3`); the reaches stay design defaults until one is indexed.
