# The pool's hand-off: a light that moves must not pop

The pool hands its real lights to whichever candidates are nearest the focus. When the ordering
changes, slot `k` jumps from one prop to another: its position teleports and its intensity steps. This
note is about that instant, and about the two ways a real pool avoids it.

## Sources

| source | what it is | use |
|---|---|---|
| `10.58709_niujhu.v8i3.1694` — Orisayemi & Afabor, *A critical scrutiny of cinematic lighting technique* | Bordwell & Thompson on lamps rearranged per camera position, and overlapping keys as actors move | why a moving key is masked, and the overlap rule |
| `10.4324_9781003671220` — Lazar, *Cinematography Beyond Technique* ch. 9 | motivated movement: a camera moves because something in the story did | the "moves happen off screen" principle |
| `10.1109_tvcg.2019.2898741` — Denes, Maruszczyk, Ash & Mantiuk, *Temporal resolution multiplexing* | the eye's finite integration time; the Talbot-Plateau law | how long a fade has to be to be invisible |
| `book_fundamentals_computer_graphics_marschner` ch. 22 | flicker fusion, apparent motion above ~10 Hz, flicker invisible above 60–80 Hz | the floor a fade must stay well under |
| `s2025-advances-materials-9` — *Stochastic tile-based lighting in HypeHype* (SIGGRAPH 2025) | sampling a subset of a tile's lights; TAA and denoising | the production alternative: trade the pop for noise |
| `s2020-advances-pdf-slides` — SIGGRAPH 2020 *Advances*, improved culling | per-tile light counts | why a light can appear and vanish by frustum |
| `book_level_of_detail_3d_graphics_luebke` ch. 9 | visual masking; a large stimulus masks a smaller one in the same place at the same time | why a fade is better than a cut |
| `book_real_time_rendering_4th_akenine` pp. 918-935 | many-light management | the survey |

## Craft principle

Film's answer to "the light must move" is not to hide the move but to make it *unwatchable*. Bordwell
& Thompson, quoted in the corpus, on the cost a production pays for it:

> "three-point lighting system demands that the lamps be rearranged virtually every time the camera
> shifts to a new framing of the scene. In spite of the great cost involved, most Hollywood films will
> have a different lighting arrangement for each camera position."

and on what happens instead when the actors move rather than the camera:

> "when the actors move, the director must decide whether to alter the lighting system. By overlapping
> different key lights, the filmmaker can maintain a constant intensity as actors move around the set
> … it will prevent distracting shadows and highlights from moving across other actors."

**Overlapping keys** is the rule this note adopts: two sources covering one subject, so the hand from
one to the other never changes the total. That is hysteresis by another name, and it is what a pool
implements when it holds a slot on a prop that is still *close enough* rather than swapping the moment
another prop is a millimetre nearer.

The second rule is the perceptual one. A fade is invisible if it is shorter than the eye can resolve as
a change. Denes et al.: the visual system has a "finite integration time that results in fusion of
rapid temporal changes", and under the Talbot-Plateau law "the perceived brightness of such a
flickering display will match the brightness of the steady light that has the same time-average
luminance". Marschner gives the practical floor: apparent motion is compelling above ~10 Hz, and
flicker is undetectable above 60–80 Hz in good light. A fade of a third of a second is 20 frames at
60 Hz, i.e. 3 Hz — far below the fusion threshold, so the eye follows it as a smooth change rather than
a step, which is exactly what is wanted.

Masking (Luebke) is the mechanism: a larger stimulus in the same place at the same time raises the
threshold for a smaller one, so a light's own falloff masks the last frames of its fade.

## How shipping games do it

- **The pool that holds its count high** (the film rule above, mechanised): keep more lights than the
  frame needs and let the change happen where nothing is being looked at. Cheap, and the frame's
  brightness never steps.
- **Hysteresis**: a slot keeps its candidate until another is meaningfully nearer — a ratio, not a
  distance. This is "overlapping keys" as an algorithm, and it is what most production culling systems
  do for shadow casters, where a pop is worse than a slightly suboptimal assignment.
- **A temporal fade**: the light's intensity ramps down at the old owner and up at the new over a few
  frames while its position steps once. This is the technique the crate's own note has named since
  iteration 0 as the defect to fix.
- **Stochastic sampling** (*SIGGRAPH 2025*, HypeHype): pick a random subset of a tile's lights per
  frame and denoise with TAA. It trades the pop for noise, and it needs a temporal denoiser the crate
  does not and should not own.
- **Reprojection**: the light's contribution is resampled from the previous frame, so a moving light
  has a velocity and blurs with the rest of the frame. Needs a history buffer; out of scope here.

## Model adopted

Two engine-free functions in `src/model/pool.rs` (the pool's own model module) plus one ECS change, and
**a cross-fade on top of them** (added in iteration 4; see *Iteration 4's findings*):

1. **`hand_off(held, challenger, hysteresis)`** — the assignment rule. A slot keeps the candidate it
   holds unless a challenger is nearer by more than `1 / hysteresis`, i.e. the challenger must be
   closer by a margin before it wins. `hysteresis = 1.0` is the current behaviour (nearest wins
   outright); the crate's default is 1.15, so a challenger must be ~13 % nearer.
2. **`fade_factor(since_hand_off, fade_seconds)`** — the intensity multiplier of a light that has just
   changed owner: `0` at the instant of the hand-off, rising smoothly to `1` over `fade_seconds`,
   with a smoothstep so the derivative is zero at both ends. The light *darkens* at the new owner and
   comes back over a third of a second; the frame's total is held by the hysteresis, not by two lights
   both being on at once (the pool has a fixed size, and two on at once would break its budget).
3. **The ECS change**: `assign_pool` remembers each slot's held candidate in a `Local`, applies
   `hand_off` against it, and writes `Flicker::base = intensity · gain · fade_factor(...)`. The slot's
   position still steps once (a light cannot be in two places), but its brightness dips through the
   step, which is what makes the step read as a change of brightness rather than a teleport.

A slot that keeps its candidate fades nothing: a steady light is exactly as before, which is what the
benchmark and the existing tests check.

## Parameters

| parameter | value | source |
|---|---|---|
| hysteresis ratio | 1.15 (a challenger must be 13 % nearer) | design default: "overlapping keys", `10.58709_niujhu.v8i3.1694`; large enough to stop chatter between two props, small enough that the nearest torch still gets the light |
| fade | 0.35 s, smoothstep | design default: below the eye's fusion threshold (`10.1109_tvcg.2019.2898741`), long enough to read as a change rather than a cut (20 frames at 60 Hz); masked by the light's own falloff (Luebke) |
| fade curve | smoothstep | design default: zero derivative at both ends, so no flicker at either boundary |
| reserve lights | 1 | design default: one cross-faded hand-off per frame; two lights on one subject for 0.35 s is the craft's overlap rule, and one is the cheapest form of it |
| who fades | only the light that changed owner | a light that keeps its candidate is untouched, so the frame is unchanged when nothing moves |

## Validation targets

- `hand_off` keeps the held candidate when the challenger is nearer by less than the hysteresis ratio,
  and swaps when it is nearer by more; at `hysteresis = 1.0` it always takes the nearest, which is the
  old behaviour and is a test.
- A held candidate that is no longer a candidate (out of reach, hidden, despawned) is always dropped,
  whatever its distance.
- `fade_factor(0) == 0`, `fade_factor(0.35) == 1`, monotone in between, and exactly 1 after.
- `fade_out_factor(since, fade) + fade_factor(since, fade) == 1` at every point of the fade, so a
  cross-faded hand-off neither brightens nor darkens the subject.
- With a reserve, a hand-off leaves the subject's total at what it was, within the flicker's own swing:
  the departing light's base plus the arriving light's, summed over the fade, is continuous.
- In `tests/pool.rs`: a slot that keeps its candidate has the same `Flicker::base` for two frames; a
  slot that changes owner has a smaller base on the frame it changes and recovers within the fade; and
  a slot whose new owner is the same entity (re-inserted) does not fade.
- In a frame sequence of the courtyard, the largest single-frame change in mean luminance falls against
  the pre-change capture (the measurement the loop log has wanted since iteration 0).

## Deviations

- **No second light to cross-fade with.** A real cross-fade needs two lights on one prop, which doubles
  the budget. The crate fades the single light down through the hand-off instead: the frame's total
  dips by that one light's falloff rather than staying exactly constant. The hysteresis (the overlap
  rule) is what keeps the *assignment* from changing often enough for the dip to be noticed.
- **No hysteresis on the focus.** The focus moving is the host's camera; it is not this note's problem.
- **The fade is on `Time` (virtual), like the flicker**, so a paused world's hand-off holds still
  rather than resolving while nobody is looking.

## Iteration 4's findings: the cross-fade

The craft's own rule — "by overlapping different key lights, the filmmaker can maintain a constant
intensity as actors move around the set" — was only half done: iteration 2's fade *dips* the total by
one light's share for a third of a second, which is a soft pop rather than a pop. A cross-fade needs
two lights on the subject for the length of the fade, which a fixed pool cannot afford per slot.

Adopted: a small **reserve** (`LightPool::reserve`, default 1) of lights the pool spawns and owns but
never assigns. When a slot changes owner the pool takes a free reserve light, puts it at the *old*
owner's last position and intensity, and fades it out over `fade_seconds` while the slot fades in. The
frame's total is then continuous across the hand-off, which is the overlap rule done literally.

- **The reserve is a budget, not a promise.** With one reserve light, the first hand-off in a frame
  cross-fades and any other that frame falls back to the dip. A host that hands over often can raise
  `reserve`; a host that cannot afford the lights keeps the dip.
- **`fade_out_factor(since, fade) = 1 - fade_factor(since, fade)`**, so the departing and arriving
  lights sum to exactly one for the whole fade: no brightness is invented at either end.
- **The reserve light's own flicker is kept.** It carries a `Flicker` at the old owner's base and
  phase, so the departing light still moves as a fire while it leaves; a light that stopped flickering
  to fade out would read as a lamp being turned down.

## Iteration 2's findings

Adopted as written, with four things the implementation taught:

- **The pool had been reading a stale position.** It used the candidate's `GlobalTransform`, which is
  propagated in `PostUpdate`, so on the frame a host moved a prop the pool compared last frame's
  distance. That reads as an extra hand-off, and with hysteresis it can hold a light on a prop that has
  gone. It now reads the candidate's own `Transform` when it has one, and falls back to
  `GlobalTransform` for a child (a torch in a hand) that only a parent places. This is a *fix*, not a
  deviation: the note's "steady is steady" line could not hold without it.
- **A slot's first light is not a hand-off.** The first draft faded a slot in from nothing whenever it
  gained a candidate, so a pool filling for the first time flashed every light up from zero. Only a
  change of owner fades.
- **Slots are claimed in a first pass.** A one-pass assignment lets slot 0 see slot 1's still-held
  candidate as free for one frame, because slot 1 has not re-claimed it yet.
- **The fade is measured, not asserted.** `a_hand_off_costs_less_than_a_pop_and_recovers_within_the_fade`
  sums the slots' bases: two full slots are 200, a hand-off dips the total and it is whole again by
  the end of the fade; with `fade_seconds: 0.0` the same test fails at `200 -> 200`, which is the old
  pop.
- **In a capture, the hysteresis is what does the work here, not the fade.** On the courtyard's
  240-frame sequence (pool 4, seed 3), excluding the frames in which the pool fills, the worst frame
  pair's change fell 0.697 -> 0.441 and the pairs over 0.45 fell from 2 to 0, with the median
  unchanged. Setting `LIGHTS_HYSTERESIS=1.0 LIGHTS_FADE=0.0` reproduces the old sequence exactly,
  which is how the first, wrong measurement (it read the pool's first filling frame) was caught: a
  switch that turns a feature off has to change what you measured, or you measured something else.

## Pending index

Nothing in this note is unindexed: every stem in *Sources* was read live from the running library on
2026-10-26. One gap, unchanged from iteration 0: Olsson, Billeter & Assarsson 2012 (cited by DOI from
Sanctuary) is still not in the corpus, so the clustered-shading comparison rests on the *Advances*
notes and RTR4 rather than the original paper.

## What great looks like

- **Nothing ever teleports.** Watch a captured frame sequence frame by frame: no light's pool jumps
  from one piece of ground to another between two frames; the transitions are a dip, not a cut.
- **The total holds.** A hand-off changes the frame by less than one light's worth of luminance, and
  the mean frame luminance across a run varies by less than the flicker's own amplitude.
- **The nearest torch still wins.** Hysteresis must not make the pool stupid: a torch half the
  distance of the held one must take the light, within one frame.
- **Steady is steady.** With nothing moving near the focus, `Flicker::base` is unchanged frame to frame
  (the pool must not introduce its own chatter).
