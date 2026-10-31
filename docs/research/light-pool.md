# The light pool: a dozen real lights over a thousand virtual ones

A world can carry a torch on every passer-by and a fire on every gate. A GPU cannot. The pool is the
standard answer — a fixed set of real lights, handed every frame to whichever props are nearest the
viewer — and this note records the craft behind the hand-off, the technique it comes from, and the
one property that matters more than any constant: **the hand-off must not pop**.

## Sources

| source | what it is | use |
|---|---|---|
| Sanctuary decision 0062 | carried torches are a look, not a rule | why a virtual torch lights nothing |
| Sanctuary decision 0111 | only the nearest N torches get a real light | the pool's existence |
| Sanctuary decision 0211 | a torch is lit only while the town is | the pool's gain |
| Sanctuary `src/prototype/torches.rs:524-565` | `assign_pool`: visible, in reach, sorted by distance, phase `1.3·k` | the semantics the crate reproduces |
| `10.1145/2343045.2343095` | Olsson, Billeter & Assarsson 2012, *Clustered deferred and forward shading*, HPG | the technique the pool stands in for; cited in Sanctuary `src/perf.rs:18` |
| home-still `s2025-advances-materials-9` | *Stochastic tile-based lighting* (SIGGRAPH 2025 *Advances*) | the modern alternative, and its noise cost |
| home-still `s2020-advances-pdf-slides` | SIGGRAPH 2020 *Advances*, improved culling for tiled and clustered rendering | per-tile light counts at silhouettes |
| home-still `book_real_time_rendering_4th_akenine` pp. 918-935 | RTR4, many-light management: z-binning, clustered shading | the survey |
| home-still `10.1007_978-3-319-24589-8_3` | Rodrigues & Clua 2015, real-time lighting for procedural 2D isometric game terrain | the same budget problem at a smaller scale |

## Craft principle

A gaffer's job is to make a dozen lamps look like a street. They do it by *choosing which ones read*:
the lamp the camera is near is bright and full, the ones behind are dimmer, and the ones just out of
frame do not exist. Nobody in the audience counts. The rule the light is doing, and the rule the eye
uses, is **proximity to the subject** — the light that matters is the light near what the shot is
about.

The craft corollaries that decide whether the illusion holds:

- **A hand-off is a change in a light's brightness and position at once.** If the light pops from
  one torch to the next, the viewer sees a light *teleport*, which is the one artifact this
  technique is famous for. A real pool fades the light out at the old owner and in at the new one
  over a few frames, or keeps the light count high enough that the move happens off screen.
- **The mean matters, not the frames.** A pool of 8 lights where each flickers ±15 % reads as a
  whole street flickering, because the eye integrates over about a tenth of a second. That is why
  the pool's own phases are spread (`1.3·slot`) rather than shared.
- **Motivated sources.** Every real light must have something visible at its position: a flame, a
  glowing coal, a lit window. A pool light with no source at its origin is the single most common
  giveaway in a low-budget night scene.
- **The light is not the flame.** A torch's light sits at head height and its flame a little above;
  putting the flame *inside* the light's origin makes the flame's own additive brightness fight the
  light's falloff. Sanctuary keeps them apart deliberately (`LIGHT_LIFT`, `HEAD_FLAME`).

## How shipping games do it

- **Olsson, Billeter & Assarsson 2012** is the paper the industry moved to: cluster the frustum in
  3D, bin the lights once per frame on the GPU, shade each pixel against its cluster's list. It
  removes the pool's hard cap — but it does not remove its *cost*, and on a frame budget that is
  already spent on shadow maps, a fixed pool is still the cheaper answer for a scene of hundreds of
  small lights that only matter close up.
- **Ghost of Tsushima** (Patry, SIGGRAPH 2021 *Advances*) lights its night scenes with a small set
  of hero practicals and treats the rest as emissive only; its talk is explicit that the emissive
  lantern is doing the work and the light is a look.
- **Horizon Zero Dawn**'s Decima engine (SIGGRAPH 2017 *Advances*) budgets its local lights per
  cluster and relies on the same idea: a light far from the camera can be dropped without a seam
  because it contributes sub-LSB radiance.
- **Stochastic tile-based lighting** (SIGGRAPH 2025 *Advances*) is the current research direction:
  sample a subset of a tile's lights per frame and denoise. It trades the pop for noise, which is a
  trade a stylised game usually does not want.

## Model adopted

`src/pool.rs`:

- `LightPool { size, reach, gain, light }` is the budget and the template. `LightPoolPlugin<F>`
  spawns `size` entities carrying `PoolLight { slot }`, each a `PointLight` cloned from
  `pool.light` with `intensity: 0` and a `Flicker { base: 0, phase: 1.3·slot }`.
- Every frame, `assign_pool::<F>` sorts the candidates — `PoolCandidate`s whose own `Visibility` is
  not `Hidden` and which are within `reach` of the one entity carrying `F` — by `(distance²,
  entity)`, and gives slot `k` the `k`-th of them at `translation + offset`, with
  `Flicker.base = intensity · gain`. Leftover slots are hidden.
- Ties break on `Entity`, so the assignment is identical on every machine and never swaps between
  two equidistant props from frame to frame. That is a determinism requirement, not a nicety: a
  tie-break on float equality would flicker.
- When the template's `color`, `range`, `radius` or `shadow_maps_enabled` changes, the new values are
  copied onto every slot, so a host can recolour the whole pool with one write. A change to `gain`,
  `size` or `reach` does not touch the slots' looks (see Deviations).

## Iteration 12: a light's rank is its contribution, not its distance

**The defect.** The pool sorts candidates by distance alone, so a 2 m torch displaces a 3 m brazier from a
real light — even though the brazier's pool is 25× the torch's lumens and the ground under the torch reads
fine from the brazier's falloff. The craft line this note quotes is "proximity to the subject is the rule
the eye uses", and the eye is reading **how much light arrives**, which is proximity *and* output together.
Ranking by distance alone is the rule with half of it missing.

**The model.** Rank by the illuminance a candidate would put on the subject, which for a point light is
`intensity / distance²` — the same inverse square `fire_lumens` inverts, read the other way. In the pool's
own units the rank key becomes `intensity / distance²`, so:

- a candidate twice as far needs four times the lumens to rank equal (which is the inverse square, exactly);
- an infinite intensity does not give a free pass — it is divided by the squared distance like everything
  else, so the nearest light still wins when the outputs are comparable;
- the sort is still over finite numbers, and ties still break on `Entity`, so determinism is unchanged.

`LightPool::rank` chooses the rule: `Rank::Illuminance` (the default, new) or `Rank::Distance` (the old
behaviour, kept because a host whose candidates are all one intensity may prefer the simpler sort and
because a behaviour change in a released crate should be switchable).

**Validation targets.**

- With a torch at 2 m burning 8 000 lm and a brazier at 3 m burning 60 000 lm, the **brazier** ranks first
  under `Illuminance` and the torch first under `Distance`. (`tests/pool.rs`.)
- A candidate at twice the distance with four times the intensity ranks equal within 1e-3, and the tie
  then breaks on `Entity`.
- Every existing test in `tests/pool.rs` that is about *assignment* (gather, hide, gain, hysteresis, the
  hand-off, the reserve) keeps passing with the new default, because those candidates all have one
  intensity.
- `gain` still multiplies the written intensity and never the rank: a dimmer night does not reshuffle which
  lights are real.

**What the demo showed, before and after a change to make it visible.** With the courtyard's candidates all
at one intensity, `Illuminance` and `Distance` chose the *same* lights and the two captures had identical
hashes — the rule is invisible when every candidate burns alike. So one bearer in six now carries a
**lantern** (four times the lumens, a taller flame), which is the case the old rule got wrong: same
distance, very different output. With that, the two rules give different frames (RMSE 2 %, mean 58.17
against 57.44): under `Illuminance` the lanterns are the lights that get handed a real one, so the warm
pools on the bearers are where the bright sources are, and the frame's light is more evenly carried into
the middle distance.

**Deviations.** The rank is per candidate and per frame, so the pool's cost gains a division per candidate
per frame — the bench measures it. A host that wants a different weighting (size, colour, a gameplay
priority) has `Rank::Distance` plus its own candidate intensities to scale by; a third variant is not
added until someone asks.

## Parameters

| parameter | crate default | courtyard | source |
|---|---|---|---|
| pool size | 16 | 8, or `LIGHTS_POOL` | design default; Sanctuary's 14 was for its foot-scale town |
| reach | 60 m | 20 m | design default; Sanctuary's 200 ft |
| gain | 1.0 | host-driven | Sanctuary decision 0211 (lit only while the town is) |
| slot light | `PointLight { intensity: 0, range: 10, shadow_maps_enabled: false }` | range 8 | design default: the pool's lights never cast shadows |
| slot phase | `1.3 · slot` | same | Sanctuary's |
| candidate offset | host's | `Y · 1.8` for a torch | Sanctuary's `LIGHT_LIFT` is 1.525 ft |

## Validation targets

- Three slots take the three nearest candidates at `position + offset`, with `Flicker.base =
  intensity · gain`. (`tests/pool.rs`.)
- A `Visibility::Hidden` candidate and one beyond `reach` are both skipped.
- `gain == 0`, no focus, and two foci each leave every slot hidden.
- Shrinking `size` from 3 to 1 leaves exactly one `PoolLight`, slot 0.
- Two candidates at equal distance are assigned in `Entity` order, and the same one every frame.
- The slot's `color`/`range`/`shadow_maps_enabled` follow `LightPool.light` when it changes.
- Two slots never share a flicker phase.

## Deviations

- **The pool is generic over the focus marker** (`LightPoolPlugin::<F>`), the way
  `bevy_speech_bubbles` is generic over its camera marker. Sanctuary's version hard-coded its own
  `CameraFocus`; a library cannot.
- **The candidate carries its offset in world axes.** Sanctuary placed a pool light at `at +
  Vec3::Y * LIGHT_LIFT` because its torch stayed upright whatever the arm did. Making the offset a
  field makes that a decision the host states rather than a fact it has to know.
- **The crate's lights are `Visibility::Hidden` when unused**, not `intensity = 0`. A hidden light
  is not extracted at all; an intensity-0 one is.
- **The template is applied when *its* shared fields change, not whenever the resource
  does.** The first draft copied them on `pool.is_changed()`; Sanctuary's night glide writes `gain`
  every frame, and its palette tints every `FireLight` itself, so the first draft would have reset
  every slot to the template's colour on every frame and fought the palette. Found by reading
  Sanctuary's `apply_palette` before publishing; `a_change_of_gain_leaves_a_slots_own_colour_alone`
  is the test. Since iteration 272 the shared fields are every field but `intensity` and
  `shadow_maps_enabled`, not four (`pool-template.md`).
- **`gain <= 0` hides the pool** rather than multiplying to zero, for the same reason.
- **Known defect, logged:** the hand-off pops. A light that moves from one candidate to another
  jumps. Fading it over a few frames is the loop log's second item.

## Pending index

Olsson, Billeter & Assarsson 2012 is cited by DOI from Sanctuary's own comment, not read from the
corpus: `10.1145/2343045.2343095` returns nothing from `paper_get`. Everything else in *Sources* is
in the corpus and searchable by stem.

## Iteration 189: the pool's budget, readable (`PoolBudget`)

A level editor that places torches has to show the author **which of them are real lights right now**: the pool's whole point is that most are not, and an author who cannot see that cannot size a scene. The assignment lived in a `Local` inside `assign_pool`, so the only way to find a holder was to read every `PoolLight`'s transform and match it back to a candidate by position. This section records the query that replaces that; it adds no tuned number, so nothing here is a default and nothing changes visually.

**Craft.** A gaffer works from a lamp schedule: every unit on the show, whether it is hot, and how many circuits are spare. The schedule is the working tool; the lamps are the result. Light-board practice (the *Cinematography Beyond Technique* and low-key lighting entries in `corpus.md`) treats "which units are on" as information the operator must always have in front of them; the crate's pool is the same board, with an automatic operator.

**Production.** Many-light budgets in shipping engines are always exposed as counters (lights requested, lights granted, lights culled): the clustered-shading and tiled-culling talks listed above (`s2020-advances-pdf-slides`, `s2025-advances-materials-9`) report per-tile and per-frame light counts as the number a content author tunes against, and RTR4 pp. 918-935 frames the budget as the first thing a lighting artist checks.

**Technical.** The pool already computes the full list: the visible, in-reach candidates ranked by `LightPool::rank`, and the slot each winner holds. `PoolBudget` publishes exactly those, at the end of `assign_pool`, as a `Resource` (`Reflect`, so `bevy_fairy` reads it). It is rewritten every frame in place, reusing its buffers, so it allocates nothing in steady state.

**Model adopted.** `PoolBudget { holders: Vec<Option<Entity>>, wanting: usize }`. `holders[slot]` is the candidate that slot holds (indexed `0..LightPool::size`, reserves excluded: a reserve only fades a departure out). `wanting` is how many candidates asked for a light this frame: visible, within reach, and the pool not dark (a pool with no focus, or gain 0, wants nothing and holds nothing). Derived: `granted()`, `starved()` (`wanting - granted`, the candidates that are virtual right now), `holder_of(slot)`, `holds(entity)`.

**Validation.** A test fixes the answer: with 5 candidates at known distances and 3 slots, `granted == 3`, `starved == 2`, and `holds` is true for exactly the three nearest; a hidden or out-of-reach candidate is not `wanting`; a dark pool reports nothing.

**Deviations.** None: no default changes. Design default: the budget excludes reserve lights.

**Pending index.** None; no new source was read.

## Iteration 190: a saveable pool budget

An editor that configures the pool live saves its settings with the project (bevy_grimoire issue #103). `LightPool` was the one settings resource without serde because `PointLight` has none. No tuned number is involved, so nothing here is a default and nothing changes visually.

**Craft.** A lamp schedule is a document that travels with the show: the same rig recalled on the next shoot day. **Production.** Engines save light budgets as project data next to the scene (the counters in `PoolBudget`'s section above are the same numbers read back). **Technical.** The pool copies exactly four fields of `light` onto its slots (`color`, `range`, `radius`, `shadow_maps_enabled`) and ignores `intensity`, so those four are the template's whole state; a `serde(with)` module writes them and rebuilds a `PointLight` from them with `intensity` 0. `#[serde(default)]` on the pool and the template means a file naming a few fields is complete. *(Wrong, found in iteration 272: a light the pool spawned took the whole template, biases and contact-shadow flag included, so a project lost them on reload. The template now saves every field but `intensity`; `pool-template.md`.)*

**Validation.** A round trip keeps every field and zeroes the template's intensity; a partial file takes the defaults. **Deviations.** None. **Pending index.** None; no new source was read.
