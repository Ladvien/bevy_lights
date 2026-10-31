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
**a cross-fade on top of them** (added in iteration 4; see *Iteration 4's findings*). Iteration 14 added
**`preferred`** and the three **set rules** below it, and rewrote the ECS pass to decide every slot's
owner before writing any of them:

1. **`hand_off(held, challenger, hysteresis)`** — the overlap rule on distances. A slot keeps the candidate it
   holds unless a challenger is nearer by more than `1 / hysteresis`, i.e. the challenger must be
   closer by a margin before it wins. `hysteresis = 1.0` is the current behaviour (nearest wins
   outright); the crate's default is 1.15, so a challenger must be ~13 % nearer.
2. **`preferred(held, challenger, hysteresis)`** — the same overlap rule on any "smaller is better"
   quantity; `hand_off` is `preferred` on distances. A pool that ranks its candidates by the light each
   puts on the subject compares *that* quantity here, so ranking and holding are one rule rather than two
   that can disagree (see *Iteration 14's findings*, consequence 3).
3. **`fade_factor(since_hand_off, fade_seconds)`** — the intensity multiplier of a light that has just
   changed owner: `0` at the instant of the hand-off, rising smoothly to `1` over `fade_seconds`,
   with a smoothstep so the derivative is zero at both ends. The light *darkens* at the new owner and
   comes back over a third of a second; the frame's total is held by the hysteresis, not by two lights
   both being on at once (the pool has a fixed size, and two on at once would break its budget).
4. **The ECS change**: `assign_pool` remembers each slot's held candidate in a `Local` and writes
   `Flicker::base = intensity · gain · fade_factor(...)`. The slot's position still steps once (a light
   cannot be in two places), but its brightness moves through the step, which is what keeps it from
   reading as a teleport. Since iteration 14 every slot's owner for the frame is decided in **one map
   before a single slot is written**, because a decision that depended on the order the slots happen to
   be written in is how a slot came to take a candidate another slot was keeping.

The set rules, in the order they matter:

- **A candidate another slot holds is never a challenger.** It belongs to its holder; nothing else may
  compare against it.
- **A challenger displaces the worst-ranked light the pool holds**, not "the slot that happens to
  compare first". The assignment is the best *set* of lights the pool can hold, so a newcomer costs one
  light rather than re-sorting the pool.
- **At most as many lights move per frame as there are free reserve lights.** A hand-off the pool cannot
  cross-fade is a hand-off that dims its subject, and a reserve is busy for the whole fade it carries. A
  scene needing more moves gets them on the following frames; the next frame recomputes the same
  challengers, so the pool catches up without ever blacking out a street for one frame.

A slot that keeps its candidate fades nothing: a steady light is exactly as before, which is what the
benchmark and the existing tests check.

## Parameters

| parameter | value | source |
|---|---|---|
| hysteresis ratio | 1.15 (a challenger must be 13 % better) | design default: "overlapping keys", `10.58709_niujhu.v8i3.1694`; large enough to stop chatter between two props, small enough that the best torch still gets the light |
| hysteresis quantity | the rank's own value: `distance² / intensity` under `Rank::Illuminance`, the squared distance under `Rank::Distance` | one quantity for ranking and holding; the illuminance form is the inverse square `fire_lumens` inverts |
| lights moved per frame | the number of free reserve lights (unthrottled when `reserve` is 0) | design default: masking is local (Luebke §8), so a mass reshuffle is the one change the eye cannot miss; a move the pool cannot cross-fade must not happen |
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
- `preferred` is `hand_off`'s rule on a general quantity: a challenger 16 % better wins, one 10 %
  better does not. Both `NaN`s are answered before the clamp, so a broken distance never becomes a zero
  (`f32::max` returns the other operand when one is `NaN`).
- **One new candidate displaces one light and no more** (`one_new_candidate_displaces_one_light_and_no_more`):
  four steady slots and one far brighter newcomer, and exactly one slot moves while the other three keep
  their position and full brightness. Reverted to the old rule it fails, with two slots moving and one of
  them taking a candidate another slot had.
- **Four times the light at twice the distance is an exact tie**, and the tie breaks on `Entity`. The rank
  value must not be inserted before `Entity` in the sort tuple: doing so silently replaces the tie-break.
- **The hand-off budget is the free reserves** (`the_pools_hand_off_budget_is_its_free_reserves`): four
  challengers nearer than every held slot in one frame move 4 lights at `reserve: 0`, 1 at the default, and
  2 at `reserve: 2`. This rule is silent when it is wrong — it looks exactly like a pool that is merely
  slow — so it has its own test; with the throttle removed the `reserve: 1` arm reports 4 and fails.
- In a frame sequence of the courtyard, the largest single-frame change in mean luminance falls against
  the pre-change capture, and no frame loses most of the pool. Measured on the pool's own total
  (`LIGHTS_POOL_LOG=1`, frames 25+: the first 15 are the pool spawning and the braziers kindling):
  worst single frame **19.0 % → 1.2 %**, frames changing more than 2 % **25 → 0**, most lights dark in
  one frame **8 of 13 → 2 of 13** (a hand-off's own fade), at two seeds and two pool sizes.

## Deviations

- **No second light to cross-fade with.** A real cross-fade needs two lights on one prop, which doubles
  the budget. The crate fades the single light down through the hand-off instead: the frame's total
  dips by that one light's falloff rather than staying exactly constant. The hysteresis (the overlap
  rule) is what keeps the *assignment* from changing often enough for the dip to be noticed.
- **No hysteresis on the focus.** The focus moving is the host's camera; it is not this note's problem.
- **The throttle turns the reserve into a move budget, so raising it does not help.** Measured on the
  courtyard's 400-frame run with the throttle in place: `reserve` 0 gives a 4.8 % worst frame (no throttle,
  no cross-fade), 1 gives 1.2 %, 2 gives 1.4 % and 4 gives 1.4 % — while the mean count of dark lights
  *rises* (0.03, 0.54, 1.54, 3.6). So the default of 1 is the point where the curve turns, and a host that
  raises `reserve` for a busier scene pays in dark reserves and gains no smoothness. The budget exists to
  guarantee that a move which *is* made can be cross-faded, not to make more moves. A host that needs many
  moves per frame should lower `fade_seconds` (a shorter fade frees a reserve sooner) rather than raise
  `reserve`.
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

## Iteration 14's findings: the reshuffle

The rule "every slot keeps its candidate unless a challenger is better by the hysteresis" was applied
per slot, each against one global best free candidate. Three consequences, all measured:

1. **One newcomer displaced every slot at once.** In the courtyard's `LIGHTS_POOL_LOG` the pool's total
   fell 17 % in a single frame (mean 951 678 lm; −162 899 lm at frame 94) and up to **7 of 8 slots read
   zero** in that frame. `before/pop/frame0007.png` shows it: every torch-lit pool gone, only the
   braziers burning, then a twenty-frame fade back.
2. **A displaced slot's fallback stole a held candidate.** The second pass took "the best candidate no
   slot has been *written* yet", so a slot written earlier could claim a candidate a later slot was
   still keeper of. Instrumented: `DBG STOLEN slot 6 took Some(1057v0) held by another keeper`.
3. **The overlap rule and the rank disagreed.** Iteration 12 ranked by illuminance but held by distance,
   so a bright far lantern could be starved by a dim near torch the rank would never have chosen.

Fixed by the three set rules above and by `preferred`. Same capture, same camera, 400 frames:

| | before | after |
|---|---|---|
| mean pool total | 951 678 lm | 1 032 425 lm (+8.5 %) |
| mean per-frame change | 10 396 lm (1.09 %) | 2 492 lm (0.24 %) |
| largest per-frame change | 162 899 lm (17.1 %) | 14 902 lm (1.4 %) |
| frames changing more than 2 % | 13 of 259 | **0** |
| most slots dark in one frame | 7 of 8 | 2 of 8 (a hand-off's own fade) |
| mean frame luminance, frames 88–110 | two 0.69 DN drops, each recovering over ~10 frames | smooth; largest step 0.07 DN |

The "+8.5 % mean total" is consequence 3 fixed: the bright lanterns now hold their lights instead of
being starved by dim near torches.

## Iteration 271's findings: one clock, and a reserve that goes out

Measured while building the shadow budget (`pool-shadows.md`), on a still scene where only the pool
changed, three defects of the cross-fade itself:

- **The two ends ran one frame apart.** The reserve aged its fade before reading it, the arriving slot
  read 0 on the frame of the hand-off, so the pair summed to `fade(t) + 1 - fade(t + 1/60)`: a dip of up
  to 7 % of the light at the smoothstep's steepest, visible as a 2.5 % sag of the lit ground over a
  third of a second once the flicker was turned off. The reserve now reads its fade before ageing it, and
  the pair sums to the light exactly (`a_light_that_earns_a_shadow_gains_it_by_a_cross_fade` holds the sum
  to 1e-3; at the old offset it reads 93).
- **A finished reserve stayed on.** Its test was `out < 1.0`, true for a fade that had ended (out 0), so a
  reserve, once used, stayed visible at zero intensity for the rest of the run. It is now hidden the frame
  its fade ends, and drops any shadow it carried.
- **A light switched on drew one stale frame.** `FlickerPlugin` skips a light whose `ViewVisibility` is
  false, and on the frame a light is shown that is still last frame's value, so the light rendered once
  at whatever it was left at. Hiding finished reserves exposed it on every cross-fade (a 6 % one-frame
  drop of the pool's total); the flicker now writes a light whose `Visibility` changed this frame.

After all three, on the courtyard's 620-frame run (frames 26+, default camera, the example's new
`reserve: 2` and three shadows), the pool's total changes by at most 0.72 % between frames against
0.87 % at the iteration's start; with `LIGHTS_FOCUS_WALK=14`, where torches stream across the reach,
1.02 % against 2.2 %, and no frame over 2 % (one before).

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
- **One move is one move.** A newcomer takes one light from the pool, not a share of every light: the
  other slots keep their candidates, and no frame changes by more than the flicker's own swing. (Added in
  iteration 14, where the pool was measured changing 17 % in a frame.)
- **The best set, not the best slot.** The lights the pool holds are the best *set* it can afford, so a
  bright far lantern keeps a light that a dim near torch would starve under a per-slot rule.
- **No light is ever sourceless and no source has two lights.** A frame sequence shows a flame at the
  foot of every pool and a pool under every flame, in every frame.
- **A light handed on mid-fade-in fades out from where it was.** (Iteration 280.) The cross-fade's two lights sum to one only if the departing light starts at the level it was *shown* at. A slot whose owner changes twice inside one fade-in has its arriving light at a few per cent; the reserve was started from the owner's full intensity, so the pool's light jumped by nearly a whole light for a frame (+57 000 lm of 943 000 in the courtyard with `LIGHTS_FOCUS_WALK=18 LIGHTS_POOL=4 LIGHTS_SEED=3 LIGHTS_HYSTERESIS=1.0`, found as a +4.5 DN step in a frame sequence's mean red, and in the build before the fix too). `LastLight::intensity` is now the level after the fade. A test hands a slot on twice inside one fade-in and bounds the pool's frame-to-frame rise.
