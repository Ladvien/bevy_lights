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
| `10.1017_jfm.2018.707` | Xia & Zhang 2018, *A vortex-dynamical scaling theory for flickering buoyant diffusion flames*, J. Fluid Mech. | **the rate law**: `f_p = 0.48 √(g/D)`, i.e. `f = 1.5/√D` Hz |
| `10.1017_jfm.2018.707` (its references) | Cetegen & Ahmed 1993, *Experiments on the periodic instability of buoyant plumes and pool fires*, Combustion and Flame — the `f ∝ D^-1/2` law and its factor 1.5 | the same law, the primary experimental source |
| `GPU Gems` ch. 6 (Nguyen, *Fire in the Vulcan Demo*) | the shipped-fire chapter: 2D-flow flames "perfect for creating candle-like flames" | how a production fire is animated |

**The rate is measured, and it is one law.** Imported for this iteration: Xia & Zhang 2018 (J. Fluid
Mech., open access, converted through scribe) states the experimental finding every earlier study
reported and then derives it from toroidal-vortex shedding:

> "Early researchers [14-19] experimentally found that `f` is related to the fuel source dimension `D`
> as `f ∝ D^{-1/2}` with a proportionality factor of around 1.5 given by Trefethen and Panton [20] and
> Cetegen and Ahmed [21]."

and its own fit to the pool-fire data of six experimental studies:

> "In general, Fig. 3 confirms that the existing experimental data together the current result are in
> good agreement with the trend line, `f_p = 0.48 √(g/D)` with `g` being 9.8 m/s², which is equivalent
> to the scaling relation, `f = 1.5 √(1/D)`, given by Cetegen and Ahmed [21]."

So the frequency of a fire's light is **`1.5 / √D` Hz for a fuel bed of diameter `D` metres**, and the
amplitude is a separate matter the flame-shape literature owns. The primary paper (Cetegen & Ahmed
1993) is not open access and `paper_download` could not get it; Xia & Zhang is, and quotes the same
law with the same constant, so the note cites the accessible source and records the primary as read
through it.

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

Three engine-free functions, `src/model/fire.rs`:

```
flicker_rate(D)                  = 1.5 / √D                       Hz, for a fuel bed D m across
flicker_factor_of(D, t, phase)   = 0.85 + 0.15 · (0.5 sin(ωt + phase) + 0.5 sin(1.7ωt + 1.7 phase))
                                   with ω = 2π · flicker_rate(D)
flicker_factor(t, phase)         = the same shape at Sanctuary's fixed 7.3 and 13.1 rad/s
kindle_gain(t)                   = smoothstep(t / 0.6) + 0.35 sin(π · smoothstep(t / 0.6))
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
| flicker rate law | `f = 1.5 / √D` Hz for a fuel bed `D` m across | `10.1017_jfm.2018.707` (Xia & Zhang 2018), quoting Cetegen & Ahmed 1993 |
| flicker rates (a diameter) | `ω = 2π·f` and `1.7·ω`, so `f` and 1.7`f` Hz | the law's `f`, in the two-sine shape Sanctuary used |
| flicker rates (no diameter) | 7.3 and 13.1 rad/s (1.16 and 2.08 Hz) | design default, Sanctuary's; kept for a host with no size to give |
| flicker amplitude | ±0.15 about a 0.85 mean | design default; the mean is load-bearing for Sanctuary's lumen budget |
| phase coupling | second sine at 1.7 × phase | design default |
| `KINDLE_SECONDS` | 0.6 s | design default, Sanctuary decision 0127 |
| kindle overshoot | 0.35 · sin(π s) | design default, Sanctuary decision 0127 |
| flicker clock | `Time` (virtual) | Sanctuary's: a paused world's fires hold still |
| invisible lights | not written; kindle still advances | Sanctuary decision 0148 |

## Validation targets

- `flicker_factor` stays inside `[0.70, 1.00]` for t ∈ [0, 100] at 0.01 steps and phase 0..6.
  (`src/model/fire.rs`'s own test.)
- `flicker_rate(D)` is `1.5/√D` Hz within 1e-3 for 0.1, 0.34, 1 and 4 m (4.74, 2.57, 1.5 and 0.75 Hz),
  monotone falling in `D`, and a degenerate diameter clamps rather than dividing by zero.
- `flicker_factor_of(D, t, phase)` stays inside `[0.70, 1.00]`, has the same mean 0.85 and the same
  amplitude as `flicker_factor`, and at `D = 0.34` completes more cycles in 10 s than the diameter-less
  form does (the sourced rate is faster than the inherited one).
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
- **The rate is now sized, and it is faster than the inherited one.** `flicker_rate(diameter)` returns
  `1.5/√D` Hz, and `flicker_factor_of(diameter, t, phase)` runs its two sines at that rate and 1.7
  times it, keeping the `[0.70, 1.00]` window and the 0.85 mean that the lumen budgets divide by.
  A 0.34 m coal bed gives **2.57 Hz** against the inherited 1.16 and 2.08: the old pair was tuned by
  eye on a foot-scale world and is slow for a brazier, which is why a fire used to read as a lamp with
  a wobble. `flicker_factor(t, phase)` stays, unchanged, as the diameter-less form the existing hosts
  use.

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
