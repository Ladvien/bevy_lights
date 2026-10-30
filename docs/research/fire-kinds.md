# Fire kinds: named presets for the numbers a fire is made of

An editor (`bevy_emerge`, family issue F7 / #103) places "fires, torches and braziers", and a game author reaching for `Flicker`, `FlameSite` and the pool's candidate has to choose a fuel-bed `diameter`, a `depth` and a flame `height` for each, with the right answer scattered over four research notes. `FireKind` is those answers by name: a closed list, serialisable, that a picker can show and a game can store in a project file instead of three loose numbers.

## Sources

| source | what it is | use |
|---|---|---|
| `10.7456_tojdac.1674124` — Doğaner 2025, *The narrational role of lighting design in The Lights of the Dark City* (corpus; quoted in [`flicker-depth.md`](flicker-depth.md)) | craft | the flicker's strength is a story decision, so each kind carries its own depth |
| `s2011-advances-pdf-slides` — SIGGRAPH 2011 *Advances*, Fire / Flamethrowers / Thrusters (corpus) | production | emission belongs to each fire's own body: two fires in one scene differ by construction, so a kind is a bundle of per-fire properties, not a global look |
| `10.1038_s41598-018-36754-w` — Chen, Guo, Jia & Xiao 2019, *Frequency and Phase Characteristics of Candle Flame Oscillation* (Sci. Rep. 9:342; read from PMC6344522) | technical | the rate law at the small end, where a candle's size is a measured diameter |
| Heskestad, *Flame heights of fuel arrays with combustion in depth* (IAFSS FSS 5, p. 427), as restated in PMC6463528 | technical | the order of magnitude of a flame's height against its bed, used as a sanity bound only |

## Craft principle

[`flicker-depth.md`](flicker-depth.md) quotes Doğaner: "the flicker of flame in a candlelit room contributes to the restless atmosphere of the story and the character's inner tension." A calm lantern and a gusting torch are different statements, so the depth is part of what *kind* of fire it is.

## How shipping games do it

The 2011 *Advances* fire talk: "Splat the emissive part of the particle into 4 channel texture – split on Z values, additively accumulate grayscale 'hotness' for each of 4 layers." Each fire owns its emission. Shipping games author a handful of named fire archetypes (candle, torch, campfire, bonfire) and vary the scale, not three unrelated knobs per instance; that is the shape adopted here.

## Model adopted

```
FireKind::{Candle, Lantern, Torch, Brazier, Campfire, Bonfire}
  .diameter()  fuel-bed diameter, metres   → Flicker::diameter, FlameSite::diameter
  .depth()     flicker depth               → Flicker::depth, FlameSite::depth
  .height()    drawn flame height, metres  → FlameSite::height
  .flicker(mean_lumens) -> Flicker     (Flicker::at_mean, so `mean` is the averaged budget)
  .site() -> FlameSite
```

The rate is not stored: it is `flicker_rate(diameter)` (`1.5/√D` Hz). Chen et al.: "the frequency remains in the range of 10-12 Hz ... The data fit an empirical formula between the frequency and the burner diameter: f ∝ D^-0.49", which is the crate's `-1/2` law, so the candle's diameter is chosen from their bundle (1.3 cm → 13 Hz, inside 20 % of the measured band).

## Parameters

| kind | diameter (m) | rate (Hz) | depth | height (m) | provenance |
|---|---|---|---|---|---|
| Candle | 0.013 | 13.2 | 0.10 | 0.04 | diameter from Chen et al.; depth and height design defaults |
| Lantern | 0.04 | 7.5 | `CALM_FLICKER_DEPTH` 0.05 | 0.12 | depth from `flicker-depth.md`; rest design defaults |
| Torch | 0.10 | 4.7 | `WHIPPING_FLICKER_DEPTH` 0.3 | 0.35 | depth from `flicker-depth.md`; height is the courtyard's carried torch |
| Brazier | 0.34 | 2.6 | `FLICKER_DEPTH` 0.15 | 1.1 | exactly the courtyard's brazier (bed, depth and flame height as it already ships) |
| Campfire | 0.8 | 1.7 | 0.20 | 0.7 | design defaults; height inside Heskestad's `0.235·Q^0.4 − 1.02·D` for ~100 kW (0.68 m) |
| Bonfire | 3.0 | 0.87 | 0.25 | 2.0 | design defaults; Heskestad for ~2 MW gives 1.8 m |

Every number not marked cited is a design default. All heights are at most `FlameLook::max_height`'s default (4 m), so a kind never packs a tag it cannot represent.

## Validation targets

- Every kind's `flicker(m).problems()` and `site().problems()` are empty.
- `Flicker::mean()` of `kind.flicker(m)` is `m`.
- Rate decreases monotonically from Candle to Bonfire; depth is within `0..=MAX_FLICKER_DEPTH`; each height is under 4 m.
- Brazier is bit-for-bit the three numbers the courtyard used before, so the showcase is unchanged.

## What great looks like

A picker whose six entries read as six different fires on screen without any further tuning: the candle a fast fine shimmer, the bonfire a slow heave.

## Embers by kind

A `FireKind` was a light and a flame; since `FlameEmbers` (`flame-embers.md`) it can also say whether the fire throws sparks. `FireKind::embers() -> Option<FlameEmbers>`.

Basis: Scintilla (`sig2024_Scintilla_Simulating_Combustible_Vegetation_for_Wildfires`, Sec. 5.7) — "the particles released per kilogram of fuel consumed are around 10³": sparks scale with the fuel burned, so a bigger fire sheds more. A candle and an enclosed lantern wick burn paraffin or oil vapour with no solid fuel to spit, so they shed none (`None`, not a rate of 0, so a picker can leave the component off). Rates, sparks a second, are design defaults rising with the bed's size: Torch 2, Brazier 3 (the crate's default and the courtyard's), Campfire 5, Bonfire 8 (`MAX_EMBER_RATE`, the rate at which all sixteen slots stay busy). The sparks' size and climb are in flame heights, so a bonfire's are larger without a second knob.

Validation: every returned value passes `problems()`; rate never falls from one kind to the next; `Candle` and `Lantern` are `None`; the courtyard's Brazier is the `FlameEmbers` default.

## Deviations

None: the Brazier row is the existing behaviour. Candle, Lantern, Campfire and Bonfire are new design defaults, flagged above.

## Pending index

Heskestad's paper is not in the corpus (searched 2026-10-29: distill returns nothing on flame height correlations); cited from its publisher's open PDF via a secondary restatement. `paper_search` for it is a candidate for a later pass.
