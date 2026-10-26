# The flame's silhouette: its edges must not be one curve

A flame's silhouette should not be one analytic curve mirrored about its axis. The shader's
`body = 1.0 - abs(dx) / half_width` gives every row the same shape on both sides of the axis, so whatever
the noise does to `dx`, the *envelope* of the silhouette is a mirror: the widest point of a row and the
widest point of the row below it are equally far from the axis, and a bulge is a bulge on both sides at
once. A real buoyant flame's bulges are detached toroidal vortices, and a vortex sheds from one side of
the flame sheet before the other.

**A correction to this iteration's own first draft, recorded because it matters.** The draft claimed the
baseline silhouette was "exactly symmetric — 0.00 px left/right edge difference on every frame". That
number came from a broken metric: the axis was computed as the mean of `(left + right)`, which makes each
row's left and right half-widths equal by construction, so the statistic was identically zero for *any*
picture. Re-measured honestly (mirroring each row's luminance about a fixed axis for the whole flame),
the baseline is **already asymmetric: a per-row mirror residual of 16.5 % of the row's peak and a
half-width difference of 5.1 % of the flame's width**. The existing `n1` fbm already perturbs `dx` per
pixel, so the crate never shipped a perfect mirror; what it ships is a mirror *envelope* with a noisy
outline. The defect is real but it is a narrower one than the draft claimed, and the numbers a fix must
beat are the ones above.

The claim still needs care, because "irregular" is not a model. The physics says *what* the bumps are and
*how big*: the bumps are puffs, and the puff's spacing follows from the same law the crate already uses for
its flicker rate.

## Sources

| source | what it is | use |
|---|---|---|
| `10.7456_tojdac.1674124` — Doğaner, *The narrational role of lighting design in The Lights of the Dark City* (2025) | why a visible source has to convince | the craft rule this note rests on: the viewer reads the source to find the light |
| `10.32657_10220_48312` — *Strategies of colour and lighting design in the cinematography of Christopher Doyle* | a practical emulated from a candle | the film's fire is a single small source, and the audience is meant to accept it |
| `s2011-advances-pdf-slides` — SIGGRAPH 2011 *Advances*, Fire/Flamethrowers/Thrusters | production fire: motion from a fluid sim, colour from a blackbody | the puff motion is what production buys a sim for |
| `s2007-advances-course-notes-0-4-mb` — Mitchell, Francke & Eng 2007, *Illustrative rendering in Team Fortress 2* | legibility first | why an unambiguous silhouette matters at gameplay distance |
| GPU Gems ch. 6 §6.1 — Hubert Nguyen, *Fire in the Vulcan Demo* (NVIDIA, 2004), corpus stem `GPU Gems … Anna's Archive` | the 2D-distortion flame this crate's shader is descended from | **the crate's own ancestor, now readable**; what it does and what it admits |
| `10.1017_jfm.2018.707` — Xia & Zhang, *A vortex-dynamical scaling theory for flickering buoyant diffusion flames* (JFM 2018) | the vortex that sheds the puff, and the law for its period | the bulge/pinch-off mechanism and its scale |
| `eth-cgl-sim_anim-Kim08` — Kim, Thürey, James & Gross, *Wavelet Turbulence for Fluid Simulation* (SIGGRAPH 2008) | "enforce Navier-Stokes over low frequencies and Kolmogorov's turbulence spectrum over high frequencies" | the argument that small-scale detail may be added procedurally at a chosen band |
| `book_real_time_rendering_4th_akenine` §6.3 | the octave sum as turbulence | the fbm the shader already uses |

GPU Gems ch. 6 is the important correction to the record. `flame.md` says the noise's provenance is
"UNVERIFIED … web sources that home-still cannot index". The **chapter** is now in the library and
indexed, so the statement it was being cited for can be checked rather than remembered — and it turns
out to say something narrower than the note assumed, which is recorded under `Deviations`.

## Craft principle

A practical's job is to be believed, and Doğaner states the mechanism the viewer uses:

> "In the story world, visible sources not only manage the viewer's perception, allowing them to
> logically find where the light comes from, but also allow the film crew to overcome technical
> limitations." (`10.7456_tojdac.1674124`)

"Logically find where the light comes from" is a claim about the *source object*, not about the light:
the audience looks at the fire and accepts that the light is coming from it. Doyle's account of the same
choice is that the fire is a single small source the frame is composed around —

> "Christopher Doyle uses small single source hard lighting to emulate the practical lighting source
> from the environment such as a street lamp and candle to set the ambience and environment of the
> scene." (`10.32657_10220_48312`)

and Doğaner gives the fire's own behaviour its emotional charge:

> "the flicker of flame in a candlelit room contributes to the restless atmosphere of the story and the
> character's inner tension." (`10.7456_tojdac.1674124`)

"Restless" is doing real work here. A restlessness that is *the same on the left as on the right* is a
loop; restlessness in a silhouette means the two edges move independently. That is the craft
constraint this iteration adopts, and it is the reason a flame which is exactly symmetric cannot read
as restless however fast it is made to flicker — which is what the baseline measures: 44.5 % peak-to-
peak silhouette-area modulation at 1.97 Hz, and zero asymmetry in every frame of it.

## How shipping games do it

Production's answer to the same defect is to stop drawing one analytic shape. The SIGGRAPH 2011 fire
talk states the mechanism in one line: "Spawn lots of high velocity particles, pick up nice motion from
fluid sim." (`s2011-advances-pdf-slides`). The bumps in a production flame are not authored; they are
advected, and the sim's job is to make the edges disagree.

The crate's own ancestor is the cheap version of that, and its author is explicit about both what it
bought and what it cost. Nguyen, GPU Gems ch. 6 §6.1:

> "The 2D distortion-based flames used a GPU-generated perturbation function that altered a flame shape
> to give it a realistic motion."

and, on the trade the technique makes:

> "The screen-aligned nature of the effect made it sensitive to the camera view angle (constraints were
> needed for top and bottom views) and to motion in general (moving toward and away from the camera
> didn't work well in 2D sometimes)."

and its verdict on the class of effect, which is the one this crate's shader belongs to:

> "more memory than the particle system technique (because we had to allocate several render targets),
> the effect is perfect for creating candle-like flames."

Two things follow. First, the ancestor *did* perturb the shape with a GPU-generated function and called
that realistic motion — so "distort the flame's shape" is the technique, and the crate inherited it only
half-way: it distorts the flame's *coordinates* but leaves its *width* analytic, which is exactly why
the result is symmetric. Second, the source's own verdict is that this class suits "candle-like flames"
— a claim about scale, which fixes how far a 2D billboard may be pushed before it is the wrong tool.

The legibility stake is set by TF2's course notes, whose stated purpose is that "game players are
consistently able to visually 'read' the scene" (`s2007-advances-course-notes-0-4-mb`). A flame is read
by its silhouette at gameplay distance — a torch at 20 m is 8 px tall and has no colour to speak of —
so the silhouette is the whole fire at the distances this crate is for.

## Model adopted

**What was built.** Three rounds of the shader change, each gated by a capture:

1. `puff_wavelength(diameter)` and `flame_bulges(height, bed)` in `src/model/flame.rs` — the puff law as
   two plain functions, with tests. This part is sound and is the shape any future attempt needs.
2. In `src/flame.wesl`: the vertex stage passes the flame's world height flat to the fragment stage; the
   fragment stage takes a **per-side half-width** from one low-frequency field sampled at `±17.3` so the
   two edges are independent curves, at a vertical frequency of `size / PUFF_WAVELENGTH` (one constant,
   0.55 m — the mean of the crate's two fire sizes), so a tall flame shows more bulges than a short one.
   `EDGE_BULGE` 0.50, then 0.70. The heat ramp moved onto the per-side `body`, so the core bends with the
   edge. Cost: one `noise()` and one flat varying.
3. A second, axial sample adding an `EDGE_LEAN` wander of the silhouette's axis, 0.45 of its half-width,
   as the part a ten-pixel torch flame could show.

**What it measured.** Everything the change could have broken held: warm-pixel clipping 0.180 % → 0.177 %,
frame mean identical to 0.01 DN, silhouette-area modulation 14.9 % → 14.5 % p2p, key-to-fill 2.00 : 1,
`bench_lights` p50 inside the same noise band. What it bought was small and confined to close range:

| close camera, brazier flame | before | after (0.70) |
|---|---|---|
| width-profile variation about its own quadratic trend | 3.67 px sd (6.2 % of mean width) | **7.01 px sd (11.7 %)** |
| width-profile bulges | 1 | **2** |

At the crate's own hero camera the change is close to invisible: the torch flames are 13–15 px wide, and
measured axis wander is 4.55 px before against 4.67 px with the added lean — a flame that reads as the
same teardrop.

**Why that is a revert.** The skill's bar is GREAT, and "fine" is explicitly NOT YET. Two things were also
learned that must not be lost, because they invalidate the obvious next attempt:

- **A bigger amplitude is not the answer.** 0.22 was invisible, 0.50 gave a visible waist, 1.00 left the
  flame gaunt and 4.00 tore it. The usable window is narrow and centred near 0.70.
- **The blocker is the missing per-flame `D`.** At gameplay distance a flame is shorter than one puff
  (`flame_bulges(0.35, 0.19) = 0.88`), so a constant `PUFF_WAVELENGTH` gives it a *fraction* of a lobe,
  which no amplitude rescues. The number that has to reach the fragment stage is each fire's own fuel-bed
  diameter, and the `MeshTag`'s 32 bits are fully spent (height 16 | phase 8 | brightness 8). The honest
  next step is a wider tag or a second instance attribute — an allocation and packing decision, not a
  constant to tune.

The shader and the crate are therefore back at `578ddac` exactly, and this note stands as the research and
the blocker rather than as a shipped model.

**One correction in this note's own reasoning, kept because it invalidates a tempting shortcut.** The
first draft measured the baseline as "exactly symmetric, 0.00 px". That was a tautology: the axis was
`mean(left + right)`, which forces the two half-widths equal for *any* picture. The baseline is already
asymmetric — 16.5 % per-row mirror residual, 5.1 % half-width difference — because the existing `n1` fbm
already perturbs `dx` per pixel. What the crate ships is a mirror *envelope* with a noisy outline, which
is a much narrower defect than the draft claimed, and it is why a per-side envelope change bought so little.

## Parameters

| parameter | value | source |
|---|---|---|
| puff wavelength `λ` | `2.09 D` (0.71 m at a 0.34 m brazier bed, 0.40 m at a 0.19 m torch bed) | `λ = U₀/f` from Xia & Zhang §II, with `U₀ ≈ √(gD)` (design default: the buoyant velocity scale of a bed `D` across) and the crate's `f = 1.5/√D`. **Adopted in the model, not shipped in the shader this iteration** |
| shader puff wavelength (reverted) | 0.55 m, one constant for every flame | the mean of the two beds the example spawns; the reason it failed is `Deviations`' first item |
| edge modulation amplitude (reverted) | 0.70 of the analytic half-width × a ×2 stretch of the fbm residual, growing as `u^0.7` | calibrated in captures: 0.22 invisible, 0.50 visible, 1.00 gaunt, 4.00 torn |
| edge lateral frequency (reverted) | 1.1 cycles per flame width | design default |
| edge scroll rate (reverted) | 0.9 metres/s up the flame | design default, on the shape's clock |
| axis lean (reverted) | 0.45 of the half-width, `u^1.4` | design default, tried and rejected: 0.33× → 0.32× axis wander at gameplay distance |

## Validation targets

For `model::flame`, which is sound and would be re-adopted by any future attempt:

- `puff_wavelength(d)` is `√g · √d / 1.5` — i.e. `U₀/f` with `U₀ = √(g·d)` — within 1e-4, for beds
  0.05, 0.19, 0.34, 1.0 and 4.0 m. This test caught the note's own algebra slip: `U₀` carries `√D`.
- `puff_wavelength(d)` is finite, positive and strictly increasing over 1e-3..1e6, and a zero, negative
  or `NaN` bed gives a finite positive number.
- `flame_bulges(1.1, 0.34)` is 1.55, `flame_bulges(0.35, 0.19)` is 0.88 within 1e-2; a zero or negative
  height or bed is exactly 0, and a `NaN` or infinity in either is 0.

For a *shader* attempt, the targets this iteration set and the next one must beat:

- At the close camera, the brazier's flame: width-profile variation about its own quadratic trend rises
  from 6.2 % of the mean width to at least 9 %. Met (11.7 %).
- **At the hero camera, a torch flame's axis wander must rise above its baseline 0.33× its own width.**
  Not met (0.32×). This is the line that decides GREAT, and it is the one the missing per-flame `D`
  blocks.
- No new clipping (met: 0.177 % against 0.180 %), no loss of temporal modulation (met: 14.5 % against
  14.9 %), no CPU regression (met).

## Deviations

- **Every shader number in this note is reverted this iteration.** The research, the puff law, the two
  model functions and the blocker stand; the visual change does not, because it did not reach GREAT on
  the crate's own camera. Nothing in the note is a claim about shipped behaviour.
- **The shader's `λ` cannot be a per-flame `D` in the current tag layout.** The `MeshTag`'s 32 bits are
  fully spent (height 16 | phase 8 | brightness 8). This is the item that blocks the next attempt and it
  is an allocation decision, not a tuning one.
- **GPU Gems ch. 6 is narrower than `flame.md` claimed.** That note cites §6.1 as "the 2D-distortion
  flame, 'perfect for creating candle-like flames' … the shape and its noise". Read in full, the
  chapter's verdict on the *2D distortion* technique is that candle-scale one, and the Vulcan demo itself
  abandoned it for video-textured sprites ("Both procedural techniques have strong advantages, but they
  didn't meet our goal of creating a believable raging fire with smoke"). So the chapter is the source
  for the *technique class* and for the puff-motion principle, **not** for this shader's noise, which
  remains UNVERIFIED and is cited as such in `src/flame.wesl`'s header.
- **The metric that judged this iteration was wrong at first and is recorded.** `mean(left + right)` as a
  mirror axis is a tautology; the honest measure is a mirror about one fixed axis for the whole flame, or
  a per-row reflection of luminance about the row's lit centre. Both are in `/home/ladvien/l16/*.py`.

## Pending index

Read live from the running library on 2026-10-26 and all in `Sources`; nothing unindexed. The gaps that
would move a default to a citation:

- A measurement of **flame-surface asymmetry** in a buoyant diffusion flame — a two-edge profile or a
  wrinkling amplitude — would replace the ±22 % with a number. Xia & Zhang give the mechanism and the
  spacing, not the amplitude; Chen & Roquemore's flow visualisations (through its references) are the
  likely source and are not open access.
- A frame-by-frame study of a **candle flame's silhouette**, which is where the "smooth tongue at small
  scale" claim could be checked rather than assumed.
- `flame.md`'s long-standing gap is unchanged: Jarzynski & Olano 2020 (JCGT) for the hash, and the
  simplex-noise provenance itself, both unindexable web references.

## What great looks like

Judged at 4× on the flame and at full frame, at the close camera and at gameplay distance:

- **No mirror plane.** The left edge and the right edge are visibly different curves on every frame,
  and no frame's silhouette is the same shape reflected. At gameplay distance (a torch 8–20 px tall)
  the flame still reads as a leaning tongue, not as a symmetric candle icon.
- **Bulges that shed.** A brazier's flame shows two or three lumps up its height, each narrowing to a
  waist above it; the lumps climb and are cut off at the tip. A torch's flame shows one, or the start of
  one — a small flame is a smooth tongue.
- **A hot core that is not a stripe.** The bright part of the flame follows the bulge that is widest at
  that height, so a row across the flame can carry more than one bright patch, and the core leans with
  the silhouette instead of running up the middle.
- **An unbroken silhouette.** No torn edges, no holes, no noise islands detached from the flame, at any
  angle or distance; the base stays seated on its fire.
- **The same fire as before.** Brightness, rate, colour, catch, size and clipping are all unchanged:
  the light the flame motivates and the flame's own swing stay one number, and no pixel that was in
  gamut goes out of it.
- **Cheap.** One more `noise()` per fragment: a street of flames still costs what a street of flames
  cost.
