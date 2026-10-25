# Changelog

All notable changes to bevy_lights. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the crate is pre-1.0, so any release may break the API.

## [Unreleased]

## [0.1.0] - 2026-10-25

First release: the practical lights extracted from Sanctuary, generalised to carry no game knowledge.

### Added

- **`FlickerPlugin`: fires that move, and fires that take hold.**
  - `Flicker { base, phase }` modulates a `PointLight` or a `SpotLight` around its mean by a summed
    sine, `0.85 ± 0.15` of base, so a light's budget can be written against the mean. Two fires at
    different phases never fall into step.
  - `Kindle { elapsed }` makes a light flare up over `KINDLE_SECONDS` (0.6 s) instead of switching on;
    re-inserting `Kindle::default()` re-lights it.
  - `Kindled { entity }` is written once per kindle on the frame it starts, so a host can play the
    sound of a fire catching.
  - A light the camera cannot see is not rewritten (nothing is extracted for it) while its kindle still
    advances, so it has already flared when it comes into view.
  - Spot lights are included: a wall sconce is a fire too.
- **`LightPoolPlugin::<Focus>`: a fixed set of real lights over thousands of virtual ones.**
  - `LightPool { size, reach, gain, light }` is the budget and the template. `PoolCandidate { offset,
    intensity }` is a prop that would like one. `PoolLight { slot }` marks the lights the crate owns.
  - The `size` nearest visible candidates within `reach` of the one entity carrying the host's focus
    marker get the lights, sorted by distance and then by `Entity`, so the assignment is the same on
    every machine and never swaps between two equidistant props.
  - `gain` dims or puts out the whole pool in one write; a `LightPool` change recolours or re-ranges
    every slot. Two slots never share a flicker phase (`1.3 · slot`).
  - A missing `FlickerPlugin` is reported with an `error!` in `finish()`, not fixed by adding the
    plugin (which would panic a host that adds it afterwards).
- **`FlamePlugin`: the billboarded shader flame.**
  - `FlameSite { height, phase }` draws an additive, billboarded flame (`src/flame.wesl`) standing on
    the site's origin, `height` world units tall, at its own clock phase.
  - One mesh, one material and one clock for every flame in the scene; each fire's phase and height
    ride on its `MeshTag`, so a street of braziers costs one draw each and nothing else.
  - `FlameLook { color, strength, max_height }` is the scene's fire colour, ember strength and height
    scale, written in place on change rather than rebuilding the material.
  - The clock is `Time<Real>`, so a paused world's flames keep moving.
- **`bevy_lights::model`: the engine-free half.** `flicker_factor`, `kindle_gain`, `KINDLE_SECONDS`,
  `raised_light`, `RANGE_MARGIN`, `pack_flame_tag` and `unpack_flame_tag`, as plain `f32` maths with no
  `bevy` in it, so a host may reuse them in its own systems.
- **`LightsSystems::{Pool, Flicker, Flame}`**, chained, so a light handed to a new candidate flickers at
  its new base in the same frame and a flame is drawn on the frame its site appears.
- **`examples/courtyard.rs`**: four shadow-casting braziers, 48 torch-bearers, a moon fill and an
  orbiting focus, with `LIGHTS_FRAMES` / `LIGHTS_SHOT` / `LIGHTS_SHOT_DIR` / `LIGHTS_EYE` /
  `LIGHTS_LOOK` / `LIGHTS_SEED` / `LIGHTS_POOL` for unattended captures.
- **`examples/bench_lights.rs`**: the CPU cost of the pool and the flicker at `BENCH_CANDIDATES` and
  `BENCH_LIGHTS`, one line of output, no window and no GPU.
- **`tools/wesl-check`**: compiles `src/flame.wesl` to WGSL and parses it with naga, in a second and
  without a GPU. A WESL error is otherwise silent at build time and shows only as a dark surface in a
  capture.
- **`tests/leaf.rs`**: the boundary ratchet. The dependency list is closed to `bevy` and `serde`; `src/`
  may not name a game's camera marker, player, unit scale or palette resource; `src/model/` may not name
  `bevy`.
- **`serde` feature**: `Serialize`/`Deserialize` on every public component and resource.

### Known limits

- **The pool's hand-off pops.** A light that moves from one candidate to another jumps in one frame.
  Fading it over a few frames is the top of `docs/loop-log.md`'s backlog.
- **Every fire flickers at one rate.** The literature says a larger fire puffs more slowly; there is no
  size parameter yet, and the fire-physics papers that would set one are not in the corpus.
- **`FlameLook::max_height` is set once**, before any site spawns: a site's height is packed as a
  fraction of it.
