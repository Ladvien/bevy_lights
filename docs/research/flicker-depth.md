# Flicker depth: how far a fire dips, and who chooses

`33fd290` made a fire's flicker depth the host's to choose: `Flicker::depth`, `FlameSite::depth` and
`PoolCandidate::depth` feed `model::fire::flicker_swing(diameter, depth, t, phase)`, with
`FLICKER_DEPTH = 0.15` as the default and `MAX_FLICKER_DEPTH = 0.5` as the ceiling. That commit changed
a public API and a default's *meaning* without a note or a CHANGELOG entry, which `AGENTS.md` forbids.
This note is the missing record, and it takes the opportunity to bound the one number every earlier
note called "uncited": the amplitude.

## Sources

| source | what it is | use |
|---|---|---|
| `10.7456_tojdac.1674124` | Doğaner, *The narrational role of lighting design in The Lights of the Dark City* (corpus, quoted in [`flame-flicker.md`](flame-flicker.md)) | the craft: the flicker is a *readable signal* whose strength is a story decision, so it is the host's to set |
| `s2011-advances-pdf-slides` | SIGGRAPH 2011 *Advances*, Fire / Flamethrowers / Thrusters (corpus, quoted in `flame-flicker.md`) | production: emission is a property of the fluctuating body, per fire, not a global constant |
| Mandin & Most, *Characterization of the Puffing Phenomenon on a Pool Fire*, IAFSS Fire Safety Science 6, p. 1137 (<https://publications.iafss.org/publications/fss/6/1137/view/fss_6-1137.pdf>) | technical: a measured upper bound on how far a flame's own state fluctuates about its mean | the bound the amplitude is checked against |
| `10.1038_s41598-020-78229-x` | Gergely et al. 2020, *Flickering candle flames and their collective behavior*, Sci. Rep. 10:21305 | technical: the radiation law the bound is converted through, and the observation that a *single* small flame is not a steady oscillator |

## Craft principle

Doğaner (quoted in `flame-flicker.md`): "the flicker of flame in a candlelit room contributes to the
restless atmosphere of the story and the character's inner tension." A restless scene wants a deeper
flicker than a calm one, so the strength of the flicker is a *per-fire art decision*, and a single
crate-wide `0.15` takes that decision away from the host. That is the whole case for `depth`, and it
is a case about authority, not about a better number.

## How shipping games do it

The 2011 *Advances* fire talk (quoted in `flame-flicker.md`): "Splat the emissive part of the particle
into 4 channel texture – split on Z values, additively accumulate grayscale 'hotness' for each of 4
layers." Emission there belongs to each fire's own body, so two fires in one scene differ by
construction. A per-fire depth is the same stance applied to the intensity envelope.

## Model adopted

```
flicker_swing(diameter, depth, t, phase) = 1 − depth · (1 − wobble)      wobble ∈ [−1, 1]
```

The multiplier runs `1 − 2·depth ..= 1` with mean `1 − depth`. **The peak is always the base**, so a
deeper fire is a darker trough, never a brighter crest: a light budget written against the mean
(Sanctuary's `WALL_LIGHT_LUMENS` divides by 0.85) stays true at the default, and a deep flicker can
never push a pool light above the lumens its host sized it for. At `FLICKER_DEPTH` this is exactly the
inherited `0.85 + 0.15 · wobble`. `flicker_depth` clamps to `0..=MAX_FLICKER_DEPTH` (so the multiplier
cannot go below 0) and reads `NaN` as the default. The flame's drawn brightness and the light's
intensity are both written from this function, so they cannot disagree.

### Is 0.15 plausible? A bound, not a validation

Mandin & Most, on a measured pool-fire flame:

> "The fluctuation amplitude AT can experimentally reach 25% of the time averaged temperature"

(the PDF's text layer garbles the word "temperature"; the next line defines `T` and `A_T = ε·T`.) That
is an **upper envelope of a measured temperature**, not a luminance. A flame's emission is
thermal radiation, and Gergely's energy balance carries it as the `−σ·T⁴` term, so a grey emitter's
radiance goes as `T⁴`. Under that law:

| `ΔT / T` | radiance multiplier |
|---|---|
| ±25 % (the measured ceiling) | 0.32 ..= 2.44 |
| ±4.4 % | 0.85 ..= 1.18, the shipped ±0.15 about a 0.85 mean (`0.15/0.85 = ±17.6 %` of the mean ≈ 4 · 4.4 %) |

So the shipped amplitude corresponds to a temperature swing about **a sixth of the largest one anyone
measured**, and a host asking for `MAX_FLICKER_DEPTH` (trough at 0) is asking for more than the
physics shows. `[INFERENCE]`: the `T⁴` conversion treats the flame as a grey body; soot emission is
at least as steep in `T`, so the conclusion that `0.15` is conservative stands, but the arithmetic is
an order-of-magnitude check, not a calibration. It does not turn `0.15` into a measured default.

## Parameters

| parameter | value | source |
|---|---|---|
| `FLICKER_DEPTH` | 0.15 | design default (inherited from Sanctuary decision 0127); now shown to sit inside the measured ceiling above |
| `MAX_FLICKER_DEPTH` | 0.5 | geometry, not art: the largest depth whose trough (`1 − 2·depth`) is still ≥ 0 |
| peak of the multiplier | 1.0 at every depth | design default, so a light budget never needs the depth |
| the calm and guttering examples in the docs (0.03, 0.3) | 0.03, 0.3 | design defaults, illustrative only |

## Validation targets

- `flicker_swing(_, d, t, p) ≤ 1` and `≥ 1 − 2d` for every `t`, `p` at `d` in `0..=MAX_FLICKER_DEPTH`
  (doctest on `flicker_swing`).
- At `FLICKER_DEPTH` the swing equals the pre-`33fd290` `0.85 + 0.15 · wobble` by algebra
  (`1 − 0.15·(1 − w) = 0.85 + 0.15·w`), so no fire in an existing host changes.
- `flicker_depth(NaN)` is the default; a negative or oversized depth cannot drive a light below 0.
- `tests/flicker.rs`, `tests/pool.rs`: a pool slot adopts its winner's depth and takes the default again
  for a candidate with none.

## What great looks like

A lantern at `0.03` reads as a steady, calm practical; a torch at `0.3` visibly breathes and still
never exceeds its base; the default reads exactly as it did before the commit.

## Deviations

- The default `0.15` is **unchanged**, so no default moves and no rendering changes.
- Not done: any change to `MAX_FLICKER_DEPTH`. The measured ceiling above argues a host should rarely
  go past ~0.3; the clamp stays at the geometric limit because a lower cap would be an art decision
  imposed on the host, which is what this API removes.

## Pending index

- Mandin & Most (IAFSS 6) is a PDF fetched for this note, not imported into the home-still corpus; the
  scribe server timed out on conversion this pass. Gergely 2020 was downloaded (`paper_download`) but
  its markdown is not yet converted or indexed.
- **The amplitude is still not *measured in luminance*.** The one paper known to do it (Maynard 2013,
  *Fire interactions and pulsation*) is unobtainable, as `fire-light.md` records. The bound above is a
  temperature bound converted through `T⁴`.
- Not indexed: `docs/research/flicker-depth.md` itself (distill was not asked to index).
