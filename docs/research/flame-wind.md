# Flame wind: a flame leans downwind

A flame in a draught leans away from it and gets shorter. Every flame the crate draws stands bolt upright in any weather, so a courtyard in a gale reads as still air. The crate cannot own the weather (`bevy_elements` does), but it can own what a fire does *with* the weather: the host copies its wind into one plain resource and every flame leans by the amount physics gives that fire's size.

## Sources

| stem / location | kind | what it gives |
|---|---|---|
| `10.7456_tojdac.1674124` (Doğaner 2025, *The narrational role of lighting design in The Lights of Dark City*) | craft, in the corpus | what a flickering, wavering flame is *for* in a frame |
| Welker & Sliepcevich (1966), *Bending of wind blown flames from liquid pools*, Fire Technology 2; constants of Pritchard & Binding, as printed in DGMR, *Technische methode voor het modelleren van plasbranden* (https://dgmr.nl/wp-content/uploads/2025/08/Technische-methode-voor-het-modelleren-van-plasbranden.pdf, eqs. 7–9) | technical | the tilt law `tan θ / cos θ = c · Fr^a · Re^b` and its constants |
| Wen et al. (2019), *Crosswind burner aspect ratio and flame drag length of diffusion flames*, Univ. of Warwick (https://wrap.warwick.ac.uk/id/eprint/111596/1/WRAP-crosswind-burner-aspect-ratio-flame-drag-length-diffusion-flames-Wen-2019.pdf), §3.1 | technical | tilt rises and height falls monotonically with wind speed |
| Raj (2009), `10.1007/s10694-009-0107-7`; Jiang & Lu (2016), `10.1016/j.proeng.2016.01.122` | technical, **not read** | paywalled; `paper_download` returned an HTML landing page for both. Listed so the next pass can read them if a copy appears |

No shipping-game or production source on wind-bent flames is in the corpus (`distill_search` for "wind effect on flame shape lean torch fire game visual effects" and "flame tilt angle in crosswind" returned only forest-fire ecology and an unrelated game-design book). That gap is recorded under `## Pending index`; the production-side claim below is therefore made from what the crate's own captures show, not from a cited game.

## Craft principle

The flame is the picture of the light's motion. Doğaner:

> "Flickering light, created by the reflection of water or flickering light by the flicker of a flame, is often used to depict the fragility of perception of reality and mood swings."

A flame that moves tells the viewer the air moves. The crate already makes the flame breathe (flicker) and catch (kindle); what an exterior night shot still lacks is the thing that sells *outdoors*: every fire in the frame bending the same way. A cinematographer asks for it with a fan off-camera; here it is the host's weather. The lean must be one direction for the whole scene (wind is a field, not a per-torch choice) and it must scale with the fire's size, or a gale reads as a gust: a brazier and a bonfire do not bend alike.

## How shipping games do it

Not cited (see Sources). What the crate does about it: a billboard flame cannot be rotated by a skeleton, so the lean is drawn in the fragment stage as a bend that grows with height (rooted at the foot, leaning at the tip), exactly where the flame already carries its own sway (`lean * s * s` in `flame.wesl`).

## Model adopted

The tilt from vertical, `θ`, of a wind-blown pool-fire flame (DGMR eq. 7–9, after Welker & Sliepcevich):

> "The tilt of the flame as a result of the windspeed can be calculated with the following formula found by Welker and Sliepcevich: tan θ / cos θ = c · Fr^a · Re^b … Pritchard and Binding formulated the following: a = 0.333, b = 0.117 and c = 0.666."

with `Fr = U² / (g·D)` and `Re = D·U / ν` (`U` the wind speed, `D` the fire's diameter, `ν` the air's kinematic viscosity). `tan θ / cos θ = sin θ / cos² θ`, so with `k` the right-hand side, `k·sin²θ + sin θ − k = 0` and

`sin θ = (√(1 + 4k²) − 1) / (2k)`

which is in `0 ≤ sin θ < 1` for every `k ≥ 0` and is 0 at no wind. That closed form is `flame_tilt` in `src/model/fire.rs` (engine-free, SI units).

Wen §3.1:

> "the flame tilt angle increases monotonically with increasing wind speed, and the flame height decreases monotonically with increasing wind speed."

So the drawn flame keeps its *length* and leans: its height on screen is `length · cos θ` and its tip sits `length · sin θ` downwind. (The length itself shrinking with wind is the other half of Wen's statement and is not modelled: the paper gives no correlation to quote.)

The wind is a world-space vector on the ground plane (`FlameWind::velocity`, metres per second in `x`/`z`). Each flame's tilt is computed on the CPU from the wind's speed and its own fire's `diameter`, written to the flame's instance transform (`scale.y`/`scale.z` carry the downwind direction times `sin θ`, shifted by 1 so the lengths stay positive and survive a parent's rotation), and the shader projects it on the camera's right. One resource, one path.

## Parameters

| parameter | value | source |
|---|---|---|
| `c`, `a`, `b` | 0.666, 0.333, 0.117 | Pritchard & Binding, via DGMR (quoted above) |
| `g` | 9.81 m/s² | physical constant |
| `ν` (air, 20 °C) | 1.5e-5 m²/s | physical constant, textbook value |
| `FLAME_TILT_LIMIT` | 70° (`sin θ` 0.94) | design default: the correlation reaches 80° at a gale's 10 m/s; a flame laid flat stops reading as a flame, and the billboard's quad widens by `sin θ` (0.94 at 70°) |
| default diameter when a fire has none | 0.2 m | design default: a torch head to a lantern's flame bed |
| lean drawn as | a rigid rotation of the flame about its foot | design default: a first version bent the flame (`x - tan θ · h^1.6`, a shear) and at 57° it collapsed to a one-pixel streak, because a shear of a narrow shape keeps its horizontal width and loses its perpendicular one; rotating the flame keeps the silhouette, the ramp and the noise as they were |
| lean from above | fades with the overhead blend | design default: from straight overhead the plume's cross-section is what is seen, and it is not translated |

## What great looks like

A row of braziers in a crosswind, each leaning the same way, the large ones less sharply than the torches, every flame still rooted at its coals and with its rim ramp intact; the flicker, the kindle and the pool's light unchanged. No flame detaches from its bowl, none is clipped at the quad's edge, none flips when the camera orbits through the wind's direction. In zero wind the picture is bit-identical to before.

## Validation targets

- `flame_tilt(0, d) == 0` for every `d`; monotonic in `U`; a larger `d` tilts less at the same `U` (the Froude number falls faster than the Reynolds number rises); never reaches 90°.
- Anchor values by hand from the formula: brazier `D = 0.34 m`, `U = 3 m/s` → `k ≈ 3.4`, `θ ≈ 59.8°`; a 0.2 m torch at 1 m/s → 47.5°; a 0.1 m/s draught on the brazier → 13°.
- Zero wind leaves every flame's transform scale at exactly `(took, 1, 1)`: the existing flame tests hold unchanged.
- Capture: courtyard at `LIGHTS_WIND` 0 vs 2 m/s, ground and eye level, two seeds; flame foot pixels identical to the no-wind frame, tip displaced downwind, the flame not clipped by its quad.

## Deviations

- The correlation was fitted to liquid pool fires of 0.1–1 m; a torch is smaller. Used anyway (a lab flame does lean at 0.3 m/s); `FLAME_TILT_LIMIT` bounds the extreme.
- A lean towards or away from the camera is drawn only as the axis's foreshortening, not as a billboard turned out of the screen plane.
- Flame *length* does not shrink with wind (Wen), only its height through the tilt.
- The light's position and its flicker do not move with the lean (the light sits at the flame's foot-third, which moves by under 0.1 of the height at the tilts shown).

## Pending index

- Raj (2009) and Jiang & Lu (2016): paywalled, not downloaded; the tilt constants are read from a secondary source (DGMR) that cites Pritchard & Binding and Welker & Sliepcevich, neither in the corpus. `Welker & Sliepcevich 1966` and `Pritchard & Binding 1992` are the primary sources to fetch.
- No shipping-game source on wind-bent flames (no corpus hit).
- Doğaner 2025 is indexed (`distill_search` found it); nothing to index from this note.
