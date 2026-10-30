# Fire kinds: named presets for the numbers a fire is made of

An editor (`bevy_emerge`, family issue F7 / #103) places "fires, torches and braziers", and a game author reaching for `Flicker`, `FlameSite` and the pool's candidate has to choose a fuel-bed `diameter`, a `depth` and a flame `height` for each, with the right answer scattered over four research notes. `FireKind` is those answers by name: a closed list, serialisable, that a picker can show and a game can store in a project file instead of three loose numbers.

## Sources

| source | what it is | use |
|---|---|---|
| `10.7456_tojdac.1674124` — Doğaner 2025, *The narrational role of lighting design in The Lights of the Dark City* (corpus; quoted in [`flicker-depth.md`](flicker-depth.md)) | craft | the flicker's strength is a story decision, so each kind carries its own depth |
| `s2011-advances-pdf-slides` — SIGGRAPH 2011 *Advances*, Fire / Flamethrowers / Thrusters (corpus) | production | emission belongs to each fire's own body: two fires in one scene differ by construction, so a kind is a bundle of per-fire properties, not a global look |
| `10.1038_s41598-018-36754-w` — Chen, Guo, Jia & Xiao 2019, *Frequency and Phase Characteristics of Candle Flame Oscillation* (Sci. Rep. 9:342; read from PMC6344522) | technical | the rate law at the small end, where a candle's size is a measured diameter |
| Heskestad, *Flame heights of fuel arrays with combustion in depth* (IAFSS FSS 5, p. 427), as restated in PMC6463528 | technical | the order of magnitude of a flame's height against its bed, used as a sanity bound only |
| `10.13140_marples.nav.lighting.thesis` — Marples 2017, PhD thesis (corpus; quoted in [`carried-fire.md`](carried-fire.md)) | production | a flame is the thing seen and the thing that lights at once, so a kind's light and flame are one object in one place (issue #9) |
| `book_real_time_rendering_4th_akenine` — Akenine-Möller, Haines & Hoffman, *Real-Time Rendering* 4th ed., 5.2 (corpus) | technical | a punctual light is a position with no size; where a sized flame puts it is a choice, and `flame_light_offset` is the crate's (issue #9) |

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

## One call for the flame and its light

A picker that stores a kind spawns `site()` and `candidate(lux, over)` by hand; forgetting the second leaves a flame that lights nothing, which a cinematographer reads as an unmotivated prop. `FireKind::fire(lux, over)` returns them as one bundle, `(FlameSite, PoolCandidate)`. No new number: every value is the existing accessor's. The bundle holds the pool's candidate, not a `Flicker` (the pool is the crate's way to light a fire; a host with its own real light spells `kind.flicker(..)`). The sparks stay `embers()`: Bevy 0.20 has no `Bundle` for `Option<Component>`, and a candle's absent sparks are the point of the `Option`.

Validation: each part equals its accessor's value and the light's diameter and depth are the flame's.

## The kind's light stands in its flame (issue #9, iteration 274)

`FireKind::candidate` (and so `fire`) built a `PoolCandidate` with `offset: Vec3::ZERO`, and its doc told the host to spawn it "on the `FlameSite` entity, whose origin is the flame's foot". The pool put the light at the foot. [`carried-fire.md`](carried-fire.md) had already settled where a fire's light belongs (`flame_light_offset(height)` = `(0, h/2, 0)`, the flame billboard's own centre, and the courtyard's torches and braziers use it), and [`flame-lean-light.md`](flame-lean-light.md) then added `lean_light_shift`, which is the displacement of the flame's **middle** when the flame turns about its foot. Adding the middle's displacement to a light that started at the foot lands it on neither: the flame turns rigidly about its foot by `θ = flame_tilt(speed, diameter)`, the middle goes to `(h/2)(sin θ · wind, cos θ)`, and a foot light shifted by `(h/2)(sin θ, cos θ − 1)` ends at `(h/2)(sin θ, cos θ − 1)`, always exactly `h/2` straight below the leaned flame's middle, and below the coal bed (`y < 0`) in any wind that tilts the flame past 0°. A host that spawned `FireKind::X.fire(..)` as documented got a light that sank under its fire the moment `FlameWind` blew.

Measured with the crate's own `flame_tilt` at a 3 m/s wind (a throwaway script of the formula in `src/model/fire.rs`, checked against the doctest's 59.8° for the brazier), light position `(downwind, up)` from the flame's foot, in metres:

| kind | height | tilt | light today | the flame's middle |
|---|---|---|---|---|
| Candle | 0.04 | 68.5° | (0.019, −0.013) | (0.019, +0.007) |
| Lantern | 0.12 | 65.8° | (0.055, −0.035) | (0.055, +0.025) |
| Torch | 0.35 | 63.3° | (0.156, −0.096) | (0.156, +0.079) |
| Brazier | 1.10 | 59.8° | (0.475, −0.273) | (0.475, +0.277) |
| Campfire | 0.70 | 57.0° | (0.294, −0.160) | (0.294, +0.190) |
| Bonfire | 2.00 | 52.4° | (0.792, −0.390) | (0.792, +0.610) |

In still air the light is at the foot, `h/2` under the middle (0.55 m for the brazier, 1.0 m for the bonfire). A pool light that is also one of `max_shadowed`'s casters then sits inside the brazier's bowl or under the bonfire's logs and blacks out the ground round it.

**Craft.** Doğaner (`10.7456_tojdac.1674124`), quoting Bordwell & Thompson: "these visible light sources will motivate the lighting decisions made during the production", and, on what the audience does with them, "when the audience associates a visible light source in the environment of the characters, they accept the reality of space and time more easily." The viewer looks at the flame to find where the light comes from; a light 0.55 m under a brazier's flame is a light the flame does not motivate.

**Production.** Marples 2017 (`10.13140_marples.nav.lighting.thesis`, on Left 4 Dead's flaming barrels): "the motion and simultaneous illumination offered by flames". A game's fire is both the thing seen and the thing that lights, so the two must be one object in one place; the 2011 *Advances* fire talk (`s2011-advances-pdf-slides`) puts it the same way: "Rendering is working quite hard to try to hide the particle nature of the system", i.e. the drawn fire and its light agree.

**Technical.** Akenine-Möller et al., *Real-Time Rendering* 4th ed. (`book_real_time_rendering_4th_akenine`, 5.2): a punctual light "has a location ... no dimensions to them, no shape or size, unlike real-world light sources". A flame has a size, so the one point that stands in for it is a choice, and the crate already made it: `flame_light_offset` is `h/2`, the centre of the billboard `src/flame.wesl` draws (`carried-fire.md`, a design default), and `lean_light_shift` moves that point. Illuminance falls as the square of the distance from whatever point is chosen, so a point 0.55 m below the flame's middle puts the brazier's ground pool 0.55 m lower than the pool the sizing (`fire_lumens`) was computed for.

**Change.** `FireKind::candidate` sets `offset` to `flame_light_offset(self.height())`, so the bundle `fire(lux, over)` is a light in its flame, upright in still air and leaning with it in wind, from the same `FlameSite` entity. `over` was "the light's height above the ground" and stays that: it is now the height of the flame's middle, which is where the light is, as `carried_light_height` in the courtyard has always meant it. The sentence of `PoolCandidate::offset`'s doc that said a candidate on the `FlameSite` entity "needs no offset at all: its translation is already the flame's foot" contradicts the sentence before it and is deleted, and `FlameWind`'s doc ("the fire's light does not move with it") has been false since the lean reached the pool.

Validation: with a stiff `FlameWind` and a pool of one, a `FireKind::Brazier.fire(..)` at the origin is handed a light at `(h/2)(sin θ, cos θ, 0)` = `(0.475, 0.277, 0)`, above the foot (`y > 0`), not at `(0.475, −0.273, 0)`; in still air at `(0, h/2, 0)`; the light's offset from the `FlameSite` is `flame_light_offset(height)` for every kind. Hosts that built their own candidate (the courtyard, the README quick start) are unchanged.

## Deviations

Design default, as `carried-fire.md`'s: `FireKind::candidate`'s `offset` changes from zero to `flame_light_offset(height)`, so a pooled light a host spawned from a `FireKind` bundle moves up by `h/2` (0.175 m for a torch, 0.55 m for a brazier) and no longer sinks under the flame in wind. The signature and `over`'s meaning (the light's height) are unchanged. The Brazier row of the numbers is still the existing behaviour; Candle, Lantern, Campfire and Bonfire are new design defaults, flagged above.

## Pending index

Heskestad's paper is not in the corpus (searched 2026-10-29: distill returns nothing on flame height correlations); cited from its publisher's open PDF via a secondary restatement. `paper_search` for it is a candidate for a later pass.

Iteration 274: `distill_search` for a flame's point-source placement failed at 17:30 CDT with `gpu busy: 4648 MB free < 5000 MB required to load bge-m3` (a vLLM engine held 14.6 GB); the three sources above come from the local corpus (`~/.cache/bevy_lights_corpus/markdown`) and were quoted from the files, not from the index. Nothing new to index.
