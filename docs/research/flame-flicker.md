# The flame's own flicker: a drawn fire that moves with its light

A fire in this crate is two things: the light it casts (`Flicker`, `src/flicker.rs`) and the flame
drawn on it (`FlameSite` / `src/flame.wesl`). Until this note they animated on **different clocks and
different laws**: the light breathed at the puffing law's rate (`1.5/√D` Hz), and the drawn flame's
brightness did not breathe at all — only its silhouette's noise scrolled. Measured on the courtyard at
400 frames: the light's pool on the ground moved ±15–22 % (peak to peak) while the flame's own pixels
moved ±3.5 %, and the two series correlated at **+0.07**. A fire whose flame is visibly still while
the ground under it flickers is the "motivated source" failure a cinematographer spots first: the
flame is the motivation, and it is not doing the motivating.

## Sources

| source | what it is | use |
|---|---|---|
| `10.7456_tojdac.1674124` | Doğaner, *The narrational role of lighting design in The Lights of the Dark City* | the craft: a visible flame is the *motivation*, and its flicker is what the shot reads |
| `s2011-advances-pdf-slides` | SIGGRAPH 2011 *Advances in Real-Time Rendering*, the Fire / Flamethrowers / Thrusters talk | production: a fire's emission is a property of its own fluctuating body, splatted from the same simulation that moves it |
| `s2008-advances-chapter-5` | SIGGRAPH 2008 *Advances*, StarCraft II (Filip Strugar) | production: a flickering practical is a lighting *case*, not a decoration |
| `10.1017_jfm.2018.707` | Xia & Zhang 2018, *A vortex-dynamical scaling theory for flickering buoyant diffusion flames*, J. Fluid Mech. | the physics of *why* a flame's emission must move: the luminous body itself sheds, pairs and merges |
| `10.1017_jfm.2018.707` (its refs) | Cetegen & Ahmed 1993; Chen, Roquemore et al. | the rate law, and the toroidal-vortex observation it rests on |
| `book_real_time_rendering_4th_akenine` §6.3 | RTR4, turbulence | the noise the silhouette already uses |
| `10.1109_tvcg.2019.2898741` | Denes et al. 2019, *Temporal resolution multiplexing* | critical flicker fusion: why a ±15 % swing at 1–3 Hz reads as life, not as a strobe |

## Craft principle

A cinematographer does not expose for the fire — the note in `fire-light.md` records that, and the
unit that does the work is a warm off-screen source. But the fire is *always in frame* when it is a
practical in shot, and Doğaner's study of *The Lights of the Dark City* is explicit about what the
audience gets from it:

> "A table lamp or street lantern on a set is not usually the main source of light used by the
> technical crew, as these sources may be inadequate or uncontrolled. However, these objects
> determine the motivation of the light in the scene. For example, if there is a candle burning in a
> room, the director and cinematographer use professional equipment that mimics the warm tones and
> soft shadows created by the candle."

The set light *mimics the candle*. If the candle's own flame held perfectly still while the shadows it
motivates danced, the mimicry would break in the one direction the audience can check by eye, because
the flame is the only part of the source they can see. And the flicker is not incidental mood
painting; it is the readable signal:

> "Flickering light, created by the reflection of water or flickering light by the flicker of a flame,
> is often used to depict the fragility of perception of reality and mood swings. … Similarly, the
> flicker of flame in a candlelit room contributes to the restless atmosphere of the story and the
> character's inner tension."

So the craft's rule here is a coherence rule, not an amplitude rule: **the flame and the light it
motivates are one fire, so they must breathe together** — same rate, same phase, same envelope. An
audience forgives a wrong amplitude far more readily than a source and its cast light disagreeing.

## How shipping games do it

SIGGRAPH 2011's Fire / Flamethrowers / Thrusters talk is a shipped engine's fire, and its emission is
not a constant tint on a moving shape — it is accumulated *from the body that moves*:

> "Splat the emissive part of the particle into 4 channel texture – split on Z values, additively
> accumulate grayscale 'hotness' for each of 4 layers."

> "Particle comp pass converts hotness to a blackbody color texture, adds it on top of smokey
> particles renderered with usual alpha blending."

The "hotness" that becomes the fire's visible colour is the *same accumulation* that the motion and
the post-processes read, so when the fire's body surges, its emission surges with it. There is no path
in that pipeline by which a fire's drawn brightness could stay flat while the thing it lights moves.

StarCraft II's lighting note shows the other half — that a flickering fire is a *lighting* case a
shipped renderer budgets for:

> "One of the stress cases we used was a group of marines, with each marine casting a flickering light
> that in turn shades the surrounding marines around him."

The light and the thing the player sees are the same fire in both talks. Neither one decouples the
drawn emission from the light's fluctuation, because a decoupled pair is a bug an artist files.

## Model adopted

One multiplier, computed once, used twice:

```
swing(phase, diameter, t)  = flicker_factor_of(diameter, t, phase)   if diameter is Some
                           = flicker_factor(t, phase)                if it is None
```

- `src/model/fire.rs` gains **`flicker_swing(diameter: Option<f32>, t, phase)`**: the single place the
  "sized or unsized" choice is made. `flicker_lights` calls it for the light, and `update_flames` calls
  it for the flame, so the two cannot drift apart — the same discipline `model::pool::preferred`
  introduced for the pool's rank and its hold in iteration 14.
- `FlameSite` gains **`diameter: Option<f32>`**: the fuel bed that sizes the flame's flicker, set to the
  same value as the `Flicker::diameter` on the light this flame motivates. A host that gives the light a
  size and the flame none would reintroduce this iteration's defect, so the field is documented as a
  pair with `Flicker::diameter`.
- `Flame`'s tag gains the flame's brightness: **height 16 bits (unchanged) | phase 8 | brightness 8**.
  The height keeps its full 16 bits because it is the number a host chooses and can see; the phase loses
  8 bits to 2π/256 ≈ 0.025 rad, which is 4 ms of clock at 6 Hz and invisible; brightness gets
  1/255 ≈ 0.4 % of the light's full intensity, against a swing of 0.30, so 1.3 % of the swing's range.
- **Brightness is on virtual `Time`; the silhouette is on `Time<Real>`.** That is the crate's existing
  split, applied one level finer: the brightness *is* the light, so a paused world's fires hold still;
  the drawn shape *is* the drawing, so its noise keeps climbing. `docs/research/fire-light.md`'s
  "a paused world's fires hold still while the flames drawn on them keep moving" is preserved and
  sharpened — now the flame both holds (in brightness) and moves (in shape) while paused.
- The catch (`kindle_gain`) stays where it is, on the instance scale; brightness is a second, independent
  number. They multiply in the fragment stage, as the light multiplies them on the CPU.

## Parameters

| parameter | value | source |
|---|---|---|
| flame brightness multiplier | the light's own `swing`, 0.70..1.00 | `10.1017_jfm.2018.707`'s rate law + this crate's inherited shape (`fire-light.md`) |
| brightness clock | `Time` (virtual), same elapsed as `flicker_lights` | design default, this crate's split; see `Deviations` |
| silhouette clock | `Time<Real>`, unchanged | `flame.md` |
| brightness quantisation | 8 bits of the 0..1 multiplier | design default: 0.4 % of full intensity, far under the swing |
| phase quantisation | 8 bits, 2π/256 rad | design default: 4 ms of clock at 6 Hz |
| `FlameSite::diameter` default | `None` (the un-sized rate) | the crate's existing default for a fire whose size is not modelled |

## Validation targets

- `flicker_swing(Some(d), t, p)` equals `flicker_factor_of(d, t, p)` and `flicker_swing(None, t, p)`
  equals `flicker_factor(t, p)` for a sweep of `d`, `t` and `p`; a `NaN` diameter does not panic.
- `pack_flame_tag` / `unpack_flame_tag` round-trip phase within 2π/256, height within 1e-3, and
  brightness within 1/255.
- `update_flames` writes a `Flame`'s tag brightness equal to the `flicker_swing` of its site's
  `diameter` and `phase`, at the virtual clock, and the value stays inside 0.70..1.00.
- In a headless app: two sites with the same `diameter` and `phase` but different heights get the same
  brightness byte; a site with `Some(0.34)` and one with `None` do not.
- In the courtyard capture: the flame region's peak-to-peak luminance rises to within a few percent of
  its pool's, and the correlation between the flame's mean and its pool's mean goes from ≈ 0 to
  **above 0.8**. (The +0.07 measured before this change is the number this iteration must move.)
- A paused world still shows a moving silhouette and a still brightness (`Time<Real>` vs `Time`).

## Deviations

- **The drawn emission tracks the light instead of being a constant.** Before, the shader's output was
  `colour · mask · strength`, with only the silhouette's noise moving. The 2011 *Advances* fire splats
  its emission from the body that moves; the crate now does the cheapest version of the same thing, by
  multiplying the fragment's emission by the light's own swing. Design default for the *coupling
  strength*: the full swing (1.0 × the light's), because the two are one fire. Halving it would be an
  amplitude judgement no source measures.
- **The clock split is finer than before.** The flame's brightness reads virtual `Time`, its shape real
  time. `flame.md`'s "Real time, so a paused world's fires still flicker" is now about the *drawing* only;
  a paused world's fires no longer brighten and dim, which is what `fire-light.md` already says the
  light does. This is the only reading of the two notes that does not contradict itself.
- **The flame's tag layout changes** from (phase 16 | height 16) to (height 16 | phase 8 | brightness 8).
  A tag packed before this change still reads its height correctly and reads a phase from the middle
  byte that was part of its old phase — so a host that persisted tags across the upgrade would see its
  phases shift. Nothing in the crate persists a tag; `dress_flames` packs one per flame at spawn.

## Pending index

- **The amplitude is still uncited**, and this iteration did not close it. `paper_search` returned the
  right paper at last — **Chamberlin & Rose 1928, *The Flicker of Luminous Flames*, Industrial &
  Engineering Chemistry** (`10.1021/ie50226a009`), which is exactly the measurement wanted: a
  photometric trace of a luminous flame against time. `paper_download` has no copy (unpaywall:
  "not open access"). Also tried and unowned: Hamins et al. 1991, *Estimate of flame radiance via a
  single location measurement in liquid pool fires* (`10.1016/0010-2180(91)90102-h`) and Lee 1975,
  *Estimates of luminous flame radiation from fires* (`10.1016/0015-2180(75)90153-4`). The `±0.15`
  stays a design default. The coupling this iteration adds is independent of it: whatever the
  amplitude, the flame and its light now share one.
- `hs paper search` is the working path for discovery while the MCP `distill_search` / `paper_search`
  bridge answers 404; `hs distill search` failed with the distill server unreachable on
  `192.168.1.110:7434` after a restart. The corpus under `~/.cache/bevy_lights_corpus/markdown` was
  used for every quote above, so nothing here is unindexed.
