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

## Deviations

None: Brazier and Torch are the shipped constants. The other four are design defaults, flagged above.

## Pending index

No new documents. A measured luminous-flux table for wood fires is not in the corpus (searched 2026-10-29: distill returned only the photometry chapters; `paper_search` returned pool-fire radiative-fraction papers that are behind publisher paywalls, e.g. `10.1016/s0010-2180(96)00098-3`); the reaches stay design defaults until one is indexed.
