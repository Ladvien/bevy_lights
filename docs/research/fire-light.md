# Fire light: flicker and kindle

How a fire's light moves, frame to frame, and how it takes hold when someone lights it. Both are
what every game with a torch gets wrong first: a steady light reads as a bulb, and a light that snaps
on reads as a switch. The crate ships the two functions Sanctuary tuned by eye, and this note records
what they are, what the literature says they should be, and what the gap costs.

## Sources

| source | what it is | use |
|---|---|---|
| Sanctuary decision 0127 | the kindle flare: `s + 0.35 sin(πs)` over 0.6 s | `KINDLE_SECONDS`, `kindle_gain` |
| Sanctuary decision 0148 | a light nobody sees costs the renderer its share of the cluster | the `ViewVisibility` skip |
| Sanctuary `src/prototype/town.rs:261-294` | the doubled sine, 7.3 and 13.1 rad/s, mapped to 0.70..1.00 | `flicker_factor` |
| Sanctuary `src/prototype/town.rs:102-106` | `WALL_LIGHT_LUMENS`' budget, which divides by the mean flicker 0.85 | why the mean is fixed at 0.85 |
| home-still `book_real_time_rendering_4th_akenine` §? | RTR4 on light animation and emissive variation | survey |
| home-still `s2007-advances-pdf-slides-2-8-mb` | Mitchell, Francke & Eng 2007, *Illustrative rendering in Team Fortress 2*: warm/cool shift by Lambert term | the look a fire's light is doing at the shading level |

Nothing in the corpus measures a fire's flicker rate. Searching `puffing frequency`, `flame
oscillation` and `candle` returns no fire-physics paper: the corpus's combustion hits are biomass
and atmospheric chemistry, and the plume-instability literature (Cetegen & Ahmed 1993; Kitahata et
al. 2005) is not in it. `docs/research/corpus.md` records that gap under *Fire and flicker physics*.

## Craft principle

A cinematographer lighting a fire on set does not use the fire. A practical flame is far too dim
and far too unsteady to expose for: it is a *motivation*, and the light that does the work is an
off-screen unit, gelled warm (full CTO, about 2900 K), flickered by a dimmer board or by an operator
rocking a flag in front of it. The rock is the point. A live flame's output wanders at 1–4 Hz by
tens of percent, and the eye reads that wander as *alive*; a fixed light of the same mean intensity
reads as a lamp, and worse, as a lamp the viewer starts to suspect is a bug.

The craft's numbers, as published in lighting texts and repeated on set:

- **Warm against cool.** Fire is the warm key; the fill is a cool moon or sky. The *ratio* is the
  composition, not either colour alone. A night scene with only warm light reads as a brown wash; a
  night scene with only cool reads as day-for-night. The ratio is what says "night" and "fire" at
  once.
- **The flicker is small and slow.** A practical flame is not a strobe: an amplitude of roughly
  ±15 % about the mean at 1–2 Hz is the range a gaffer's dimmer chase is set to, and doubling the
  amplitude reads as a failing bulb rather than a fire.
- **The catch.** A fire that is being lit flares *past* its running brightness and settles, because
  the kindling catches unevenly before the fuel is alight. That overshoot is the sound cue's visual
  half; without it the sound arrives on a light that was already on.

## How shipping games do it

- **The Last of Us Part II** and **Ghost of Tsushima** both animate practicals as a base intensity
  times a low-frequency noise or sine bank, and both put a *hand-authored* envelope on the ignition
  so a lit torch flares. Neither simulates the flame's fluid dynamics for the light.
- **Team Fortress 2**'s illustrated lighting (Mitchell, Francke & Eng, SIGGRAPH 2007 *Advances*)
  is the clearest published statement of the warm/cool split as a *shading* rule: the same
  half-Lambert term drives a warm-to-cool ramp, so a fire's light reads warm on the lit side and the
  shadow side goes cool without a second light. This is the technique behind a scene staying
  readable with one practical and one fill.
- **Unity and Unreal** both ship a "flicker" component or a light-function material that is this
  same shape: a base intensity modulated by summed sines or a noise texture, with a separate
  ignition curve.

## Model adopted

Two engine-free functions, `src/model/fire.rs`:

```
flicker_factor(t, phase) = 0.85 + 0.15 · (0.5 sin(7.3t + phase) + 0.5 sin(13.1t + 1.7 phase))
kindle_gain(t)           = smoothstep(t / 0.6) + 0.35 sin(π · smoothstep(t / 0.6))
```

`flicker_factor` is bounded by construction to `[0.70, 1.00]`, mean 0.85, and the second sine runs
at 1.7 times the phase so two fires never resynchronise. `kindle_gain` is 0 at ignition, 0.85 at
half time, peaks just over 1 near 0.85 of the way and is exactly 1 from 0.6 s on.

The ECS side (`src/flicker.rs`) writes `intensity = base · flicker_factor(t, phase) · kindle_gain`
on every `PointLight` and `SpotLight` carrying `Flicker`, on **virtual** time, and skips the write
for a light the camera cannot see. A `Kindle` advances regardless of visibility, and writes
`Kindled { entity }` on the frame it appears, so the host's sound is never lost to the light being
off screen.

## Parameters

| parameter | value | source |
|---|---|---|
| flicker rates | 7.3 and 13.1 rad/s (1.16 and 2.08 Hz) | design default, Sanctuary's |
| flicker amplitude | ±0.15 about a 0.85 mean | design default; the mean is load-bearing for Sanctuary's lumen budget |
| phase coupling | second sine at 1.7 × phase | design default |
| `KINDLE_SECONDS` | 0.6 s | design default, Sanctuary decision 0127 |
| kindle overshoot | 0.35 · sin(π s) | design default, Sanctuary decision 0127 |
| flicker clock | `Time` (virtual) | Sanctuary's: a paused world's fires hold still |
| invisible lights | not written; kindle still advances | Sanctuary decision 0148 |

## Validation targets

- `flicker_factor` stays inside `[0.70, 1.00]` for t ∈ [0, 100] at 0.01 steps and phase 0..6.
  (`src/model/fire.rs`'s own test.)
- `kindle_gain(0) == 0`, `kindle_gain(0.3) ≈ 0.85`, `kindle_gain(0.6) ≈ 1`, `kindle_gain(t > 0.6) == 1`.
- A kindling light writes exactly one `Kindled` over two seconds and settles inside its window.
  (`tests/flicker.rs`.)
- A spot light's intensity varies, and every sample is inside 0.70..1.00 of its base.
- An unseen light keeps its last intensity and its `Kindle.elapsed` still reaches `KINDLE_SECONDS`.
- In the courtyard, the four braziers print `kindled` once each on the first frames and once more
  per `K`.

## Deviations

- **The crate applies the flicker to spot lights too.** Sanctuary's `flicker_fires` queried only
  `&mut PointLight`, while the wall sconces that carry `Flicker` are spots: they were meant to
  flicker (their lumen budget divides by 0.85) and never did. Applying it here is the fix the
  budget already assumed, and Sanctuary's decision record records it.
- **No fire-size parameter yet.** The literature says a larger fire puffs more slowly, so a brazier
  and a candle should not flicker at one rate. This crate has one rate for every fire. That is the
  first item on `docs/loop-log.md`'s backlog, and it is where the missing fire-physics literature
  has to land.

## Pending index

Nothing in the corpus to index for this note: the search terms that would carry it (`puffing
frequency`, `flame oscillation`, plume instability against heat-release rate) return no fire-physics
document. To fill the gap:

- Cetegen & Ahmed 1993, *Periodic instability of buoyant plumes and pool fires* (Combustion and
  Flame) — the puffing frequency against pool diameter.
- Kitahata et al. 2005, *Oscillation and synchronization in the combustion of candles* (J. Phys.
  Chem. A) — the 1–2 Hz range and the coupling between two flames.
- Malalasekera, Versteeg & Gilchrist 1996, *A review of research and an experimental study on the
  pulsation of buoyant diffusion flames and pool fires*.
