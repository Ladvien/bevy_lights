# The flame's silhouette: it must not be a mirror

Every flame the crate draws is a mirror-symmetric wedge. That is not a tuning problem, it is arithmetic:
`src/flame.wesl`'s `body = 1.0 - abs(dx) / half_width` with a `half_width` that is a function of height
alone makes the left edge exactly the negation of the right edge. Measured on iteration 16's baseline:
the left and right edge of an isolated torch flame differ by **0.00 px, on every sampled frame**, of a
mean width of 12–83 px (`/home/ladvien/l16/findings.md`).

`docs/research/flame.md` opens by saying a shader that misses any of a flame's three qualities "reads
as a decal". This note is the fourth quality, and it is the one the crate fails completely: **a flame
has no mirror plane**. Its edges are independent curves, its bulges shed from one side, and its waist
pinches off. A symmetrical flame is not a slightly-wrong flame; it is a drawing of a flame.

The claim needs care, because "irregular" is not a model. The physics says *what* the bumps are and
*how big*: the bumps are puffs, and the puff's spacing follows from the same law the crate already uses
for its flicker rate.

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

One change, in the shader, with its one number in `src/model/flame.rs` so it is testable without a GPU
and printable by the example.

**The mechanism is the physics's, and it is already in the crate.** Xia & Zhang identify the two vortex
families in a buoyant diffusion flame:

> "Chen and Roquemore … identified two different types of vortices — the small vortices inside of the
> luminous flame developed due to the instability of the jet and the large toroidal vortices outside of
> the flame sheet caused by the buoyancy-induced Kelvin-Helmholtz instability."
> (`10.1017_jfm.2018.707`)

and the visible consequence is exactly the asymmetry and the bumps:

> "the flame oscillation to the dynamics of the toroidal vortices, which were observed to result in the
> detachment of the flame puff (flame pinch-off) as well as 'pairing' and 'merging' of the flame bulge"

A toroidal vortex is a ring outside the flame sheet, and its pairing is a *one-sided* event: the bulge
grows on the side where the ring is stronger, then detaches. So a real flame's left edge and right edge
are two different curves, and the bulges up the flame are where the puffs detached. That is the model:
**two independent edges, with a bulge whose spacing follows the puff.**

**The bulge's spacing comes from the same law as the flicker rate.** The vortex sheet is carried
downstream by `H = U₀τ` in one period (Xia & Zhang, §II), so the distance between successive puffs is
`λ = U₀ / f`. Taking the plume's characteristic rise speed as the buoyant velocity scale of a bed `D`
across, `U₀ ≈ √(gD)`, and the crate's own `f = 1.5 / √D` from `model::fire::flicker_rate`, gives

$$\lambda = \frac{\sqrt{g D}}{1.5/\sqrt{D}} = \frac{\sqrt{g}}{1.5}\,D \approx 2.09\,D .$$

So a puff's spacing is proportional to the **fuel bed**, not to the flame: a 0.34 m coal bed puts a bulge
every 0.71 m, a 0.19 m torch's bed every 0.40 m. `model::flame::puff_wavelength(diameter)` is that law,
and `model::flame::flame_bulges(height, bed)` is how many puffs fit up a flame of a given height — the
quantity the shader's constant is judged by.

**The shader.** The vertex stage already knows the flame's height in world units (`size`); it now passes
it flat to the fragment stage. The fragment stage:

1. computes the analytic profile as before, then takes **two** half-widths, one per side, each
   modulated by its own value of one low-frequency noise field sampled at a coordinate offset by the
   side (`+side · 17.3`), so the left and right edges are independent curves and no mirror plane
   survives;
2. the modulation's vertical frequency is `size / PUFF_WAVELENGTH`, so a tall flame shows several
   bulges and a short one shows a fraction of one — a small flame is a smooth tongue, a brazier's flame
   is lumpy, which the `flame_bulges` values above say it should be;
3. the noise rises with the shape's existing scroll, so a bulge climbs the flame and is shed at the top
   the way a puff is;
4. the heat ramp is computed from the per-side `body`, so the luminance is no longer a function of
   `abs(dx)` and a flame row can carry more than one lobe.

Cost: **one** `noise()` call more per fragment and one more flat varying. No new component, no new
field, no new resource, one path.

## Parameters

| parameter | value | source |
|---|---|---|
| puff wavelength `λ` | `2.09 D` (0.71 m at a 0.34 m brazier bed, 0.40 m at a 0.19 m torch bed) | `λ = U₀/f` from Xia & Zhang §II, with `U₀ ≈ √(gD)` (design default: the buoyant velocity scale of a bed `D` across) and the crate's `f = 1.5/√D` |
| the shader's puff wavelength | 0.55 m, one constant for every flame | the mean of the two beds the example spawns (0.71 and 0.40 m); see `Deviations` |
| bulge count, brazier (1.1 m flame, 0.34 m bed) | 1.55 at the law's own λ; 2.00 at the shader's 0.55 m | `flame_bulges(1.1, 0.34)` |
| bulge count, torch (0.35 m flame, 0.19 m bed) | 0.88 at the law's own λ; 0.64 at the shader's 0.55 m | `flame_bulges(0.35, 0.19)` |
| edge modulation amplitude | ±22 % of the analytic half-width, growing with height as `u^0.7` | design default: enough that the widest bulge and the narrowest waist differ by 44 % (a puff's visible size) and the base stays clean |
| edge noise frequency, across the flame | 1.1 cycles per flame width | design default: one lateral feature per flame, so the silhouette reads as a leaning tongue and not as a torn rag |
| edge scroll rate | 0.9 units/s up the flame | design default: the shape's own clock, the same choice the crate already makes for its noise (1.7 and 2.4 units/s); the *brightness* stays on the puffing law, unchanged |

## Validation targets

- `puff_wavelength(d)` is `√g / 1.5` within 1e-3 times `d`, checked against the two beds the example
  spawns: a 0.34 m brazier bed gives 0.71 m ± 0.02, a 0.19 m torch bed 0.40 m ± 0.02.
- `flame_bulges(1.1, 0.34)` is 1.55 ± 0.02, `flame_bulges(0.35, 0.19)` is 0.88 ± 0.02; a zero or
  negative height or bed is 0, and a `NaN` in either is 0.
- `puff_wavelength(d)` is finite, positive and strictly increasing for `d` in 1e-3..1e6, and a `NaN` or
  a non-positive bed gives a finite positive number.
- In a capture at the close camera: an isolated torch flame's **left/right edge difference is at least
  8 % of its mean width** on every sampled frame (baseline: 0.00 %), and the same flame's width profile
  is no longer mirror-symmetric about its own centroid.
- A flame's silhouette **area** still modulates (the flicker is not lost): 30–60 % peak-to-peak over 40
  frames, as the baseline's 44.5 %.
- A flame row at mid-height carries **two or more luminance lobes** where the baseline carried one.
- `tools/wesl-check` still compiles and naga still parses the module — a WESL error is silent at build
  time and loud only as a black flame in a capture.
- No new clipping: warm pixels at ≥ 254 DN stay at or below the baseline's 0.18 % on the default camera.
- The CPU bench does not regress (the change is GPU-side; the bench cannot see it and is reported to
  say so).

## Deviations

- **The shader's `λ` is a constant, not a per-flame `D`.** The right number is each fire's own puff
  wavelength, which needs its fuel-bed diameter in the fragment stage; the `MeshTag`'s 32 bits are fully
  spent (height 16 | phase 8 | brightness 8) and no free channel exists that is not a hack. So all flames
  share one puff *spacing in metres*, which is right for the crate's own sizes (0.40–0.71 m) and wrong
  for a scene mixing a candle with a bonfire. Recorded here rather than hidden; the fix, if a host needs
  it, is a wider tag or a second instance attribute, and it is not this iteration's.
- **The bulge's *rate* is the shape's clock, not the puffing frequency.** The crate already animates the
  shape on its own visual clock (1.7 and 2.4 units/s) while the brightness and the light follow the law;
  this note keeps that split, because changing it needs the same per-flame `D` the point above lacks.
  The brightness — the number a light budget is written against — is untouched.
- **GPU Gems ch. 6 is narrower than `flame.md` claimed.** That note cites §6.1 as "the 2D-distortion
  flame, 'perfect for creating candle-like flames' … the shape and its noise". Read in full, the
  chapter's verdict on the *2D distortion* technique is the candle-scale one quoted above, and the
  Vulcan demo itself abandoned it for video-textured sprites ("Both procedural techniques have strong
  advantages, but they didn't meet our goal of creating a believable raging fire with smoke"). So the
  chapter is the source for the *technique class* and for the puff-motion principle, **not** for this
  shader's noise, which remains UNVERIFIED and is cited as such in `src/flame.wesl`'s header.
  `flame.md`'s `Pending index` records this correction.
- **The 22 % edge amplitude and the 1.1-cycle lateral frequency are design defaults** with no citation,
  like every other shape number in this system. They are the two numbers a hostile critique attacks
  first, and the "What great looks like" section says how they fail.

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
