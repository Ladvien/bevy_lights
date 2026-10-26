# Changelog

All notable changes to bevy_lights. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the crate is pre-1.0, so any release may break the API.

## [Unreleased]

### Added

- **A flame is redder while its fire catches.** The shader's ramp is pulled towards an ember red as a
  fire kindles and released as it takes, so the catch reads as a young fire and not as a steady one
  switching on. Driven by the same `kindle_gain` envelope as the light and the flame's growth, carried to
  the shader in the material's spare uniform slot.

- **A flame catches with its light.** `FlameSite::kindle` (seconds) makes a flame grow from nothing to
  its full height and width over that time, on the same `kindle_gain` envelope the fire's *light* uses —
  so a fire that is being lit arrives as one event instead of a full-size flame over a light still at
  zero. `KINDLE_SECONDS` is the usual value; `0.0` (the default) is a fire that has been burning all
  along and costs nothing.

- **A cross-faded pool hand-off.** `LightPool::reserve` (default 1) holds lights the pool owns but never
  assigns; when a candidate leaves the pool, a reserve light takes its last position and intensity and
  fades out while the slot's new owner fades in, so the subject's light is continuous across the
  hand-off instead of dipping. `model::pool::fade_out_factor` is the exact complement of
  `fade_factor`, and a test asserts the two sum to one for the whole fade. `reserve: 0` restores the
  dip.

- **A fire's flicker sized by its fuel bed**, from the measured puffing law.
  - `Flicker { diameter: Option<f32> }`: a fire's fuel-bed diameter in the host's world units. A
    brazier's 0.34 m coals flicker at 2.6 Hz, a 4 m pyre at 0.75 Hz.
  - `model::fire::flicker_rate(diameter)` = `1.5/√D` Hz, and `flicker_factor_of(diameter, t, phase)`
    running the same two-sine shape at that rate. The window (`0.70..1.00`), the mean (0.85) and the
    amplitude are unchanged, so a lumen budget written against the mean still holds.
  - The law is Cetegen & Ahmed 1993's `f ∝ D^-1/2` with the factor 1.5, as restated and fitted by
    Xia & Zhang 2018 (*J. Fluid Mech.*) to six experimental studies: `f = 0.48√(g/D)`. `flicker_factor`
    and the old rates stay for a host with no size to give.

- **The pool's hand-off: an overlap rule and a fade**, so a light that changes owner does not pop.
  - `LightPool::hysteresis` (default `HYSTERESIS` = 1.15): a slot keeps the candidate it holds until a
    challenger is about 13 % nearer. `1.0` is the old nearest-wins rule.
  - `LightPool::fade_seconds` (default `FADE_SECONDS` = 0.35): a slot that changes owner comes back up
    from nothing over that time instead of snapping on. `0.0` restores the old behaviour.
  - `model::pool::hand_off` and `model::pool::fade_factor`: the rule and the curve, engine-free and
    re-exported flat, so a host can reason about a hand-off without an app.

- **`model::cinematography`: a scene's lighting as the numbers a cinematographer sets**, so a host can
  size and colour its practicals instead of picking them by eye.
  - `key_to_fill(key_lux, fill_lux)` and `stops(ratio)`: the ratio of a scene's key to its fill, with the
    craft's three conventional ratios as constants — `LOW_KEY` (8:1, a night exterior), `MOON_KEY`
    (4:1) and `HIGH_KEY` (2:1, a lit interior).
  - `fill_for(key_lux, ratio)`: the inverse, so a host that knows what its fire puts on the ground asks
    for the sky that makes a night of it.
  - `fire_lumens(lux, reach, over)`: the lumens a practical needs for the ground `reach` out and `over`
    below it to receive `lux`. Bevy maps a point light's lumens to Φ/4π candela, so this is the inverse
    square solved for the light: a pool gets a *size* instead of an arbitrary number.
  - `blackbody_colour(kelvin, strength)`: a light's linear RGB from its colour temperature (Planck's law
    against the CIE 1931 observer), so a fire is coloured by the temperature it burns at.
  - `white_balanced(kelvin, balance_kelvin, strength)`: the same colour as a camera balanced to
    `balance_kelvin` records it, which is how one balance turns a fire orange and the sky blue.
  - All five are engine-free (in `src/model/`, covered by `tests/leaf.rs`) and re-exported flat from the
    crate root. Their sources and parameters are in `docs/research/lighting-look.md`.

### Fixed

- **A child candidate sat at its own local offset, not at its parent's place.** The pool preferred a
  candidate's own `Transform` (to read a root prop the frame the host moves it), but for a child that is
  the constant local offset — so every torch, which is a child of its bearer, collapsed to one point. The
  assignment therefore never changed: the same lights held forever and no candidate ever left the pool.
  A local `Transform` is now read only for a candidate with no `ChildOf`.
- **The pool's cross-fade only fired when an owner left the pool.** The common hand-off is a slot
  changing owner while the old one is still a candidate, displaced by a nearer prop; that case kept the
  dip. It now cross-fades on any change of owner.

- **Both ends of a pool cross-fade now keep the same flicker.** A reserve light that was fading a
  departing owner out carried its own flicker phase, so the two ends of the cross-fade beat against
  each other and their sum wobbled. It now keeps the phase of the light that left.


- **The pool read a candidate's `GlobalTransform`, which lags a frame.** A host that moved a prop had
  the pool decide against the previous frame's position, which read as an extra hand-off (and, with
  hysteresis, could hold a light on a prop that had already gone). The pool now reads the candidate's
  own `Transform` when it has one and falls back to `GlobalTransform` for a child placed by its parent.

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
  - `gain` dims or puts out the whole pool in one write. A change to the template light's colour,
    range, radius or shadow setting recolours or re-ranges every slot; a change to `gain`, `size` or
    `reach` leaves the slots' looks alone, so a host that tints its own fire lights keeps its tint
    through a night glide that writes `gain` every frame. Two slots never share a flicker phase
    (`1.3 · slot`).
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
