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
- The pool's `is_changed` copies `color`, `range`, `radius` and `shadow_maps_enabled` onto every
  slot, so a host can recolour the whole pool for a palette change with one write.

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
- **`gain <= 0` hides the pool** rather than multiplying to zero, for the same reason.
- **Known defect, logged:** the hand-off pops. A light that moves from one candidate to another
  jumps. Fading it over a few frames is the loop log's second item.

## Pending index

Olsson, Billeter & Assarsson 2012 is cited by DOI from Sanctuary's own comment, not read from the
corpus: `10.1145/2343045.2343095` returns nothing from `paper_get`. Everything else in *Sources* is
in the corpus and searchable by stem.
