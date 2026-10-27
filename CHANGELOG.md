# Changelog

All notable changes to bevy_lights. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the crate is pre-1.0, so any release may break the API.

## [Unreleased]

### Changed

- **The `courtyard` example's carried flames now have something carrying them.** Each bearer's flame hung 0.42 m out from the capsule with no hand, stick or lantern under it: seen from the elevated camera it was a light with no source. A torch now has a stick ending at the flame's foot, a lantern has an iron body, and a fist and an arm join it to the figure. Example geometry only: the crate's API and its lighting are unchanged (frame mean 54.74 → 54.72 at the elevated camera, pool 4). `docs/research/carried-fire.md` has the amendment.

### Fixed

- **`tools/wesl-check`'s `View` stub was missing `world_position` and `exposure`.** `view.world_position`
  is in the real `View` (`bevy_render-0.20.0/src/view.wesl`) and the stub declared only
  `world_from_view` and `clip_from_world`, so any flame shader that read the camera's own position failed
  the validator's parse — proven both ways: a one-line probe reading the field fails on the old stub with
  `invalid accessor world_position`, and compiles on the extended one. The stub now carries both fields in
  the real struct's order. Nothing in the crate read them before; this closes the hole before something
  does. `AGENTS.md` records the rule: a stub's *bodies* are never run, but its *fields* are type-checked,
  so a subset stub rejects valid code and a superset accepts invalid code.

### Added

- **`Flicker::depth`, `FlameSite::depth`, `PoolCandidate::depth`; `flicker_swing` takes a depth** (landed in
  `33fd290`, recorded here). How far a fire dips was the crate's inherited `0.15` for every fire; a host
  now chooses it per fire (a calm lantern `0.03`, a guttering torch `0.3`). The multiplier runs
  `1 − 2·depth ..= 1`, so a fire's **peak is always its base** and a deeper flicker darkens the trough
  rather than brightening the crest; `FLICKER_DEPTH = 0.15` keeps every existing fire exactly as it was,
  `MAX_FLICKER_DEPTH = 0.5` is the largest depth whose trough stays non-negative, and `flicker_depth`
  clamps and reads `NaN` as the default. A pool slot adopts its winner's depth and returns to the default
  for a candidate that chose none. **Breaking:** `flicker_swing(diameter, t, phase)` became
  `flicker_swing(diameter, depth, t, phase)`, and `Flicker`, `FlameSite` and `PoolCandidate` gained a
  field, so struct literals without `..default()` stop compiling. `docs/research/flicker-depth.md` is the
  note: it bounds the default against Mandin & Most's measured flame fluctuation (a temperature amplitude
  up to 25 % of the mean, against the ≈4.4 % the shipped `±0.15` implies through `T⁴`). The `0.15` itself
  stays a design default. Two behaviour tests (`tests/flicker.rs`, `tests/pool.rs`) pin the
  peak-is-base and slot-adoption invariants.

- **`char_field`, `char_radiance`, `coal_radiance`, `EMBER_AT` and `blackbody_luminance`** — the
  engine-free half of a fuel bed. A bed of coals is a *surface with a temperature field on it*, not a
  uniform emitter: `char_field(u, v, seed)` is that field as a heap of overlapping coal shards (the
  bright lumps are coal, the dark channels between them the ash), `char_radiance` maps a sample to a
  colour through three stops, and `coal_radiance` is the same ramp written from three *temperatures* so
  a bed's brightness and colour cannot drift apart. `blackbody_luminance` is the missing magnitude
  `blackbody_colour` never gave: it normalises to its brightest channel, which is right for a light
  whose lumens the host sets and wrong for a surface whose `emissive` is a radiance in cd/m².
  `docs/research/fuel-bed.md` is the note. The defect it fixes is measured: `examples/courtyard`'s
  coal bed was a constant `emissive`, so it clipped to a **flat white slab** at every camera and
  reached `min channel ≥ 245` over 36 px at 85° of elevation — where it also hid the flame iteration 19
  had made visible from above.

- **`model::reach::flame_light_offset(height)`** — where a fire's own light belongs, relative to its
  `FlameSite`'s origin: `(0, height/2, 0)`, the centre of the flame billboard `src/flame.wesl` draws. A
  fire's flame and the real light it motivates are two entities, and a host that places them by different
  numbers gets both defects at once: a light with no visible source, and an illuminance the sizing never
  intended, since `E = Φ/4πr²` is measured from where the light actually is.
  `docs/research/carried-fire.md` is the note, with the craft, production and radiometry sources.

- **`PoolCandidate::offset` is documented against that helper.** A candidate spawned on the `FlameSite`
  entity needs no offset at all; one on a *parent* adds only the difference. The wrong shape — the
  flame's full height added to a candidate already at the flame — is named where a host will read it.

### Fixed

- **A flame lit with its light now grows from a point instead of appearing whole.** `FlameSite::kindle`
  was documented to grow the flame over its catch, and `update_flames` wrote the right number every
  frame — but into the `Transform`'s scale, which `src/flame.wesl` read only to tint the ember and never
  to size the billboard. Measured on a flame spawned mid-run: its bbox height across the 0.6 s catch was
  **flat at 155–174 px** (the silhouette's own noise) and only its peak luminance rose, 120 → 161 DN. The
  vertex stage now multiplies that scale into the flame's drawn size, and the height follows
  `kindle_gain` to **0.98 px RMSE** over the catch, 6 → 166 px. A settled flame is unchanged: all 13
  capture cameras are pixel-identical before and after. The unit test that was supposed to cover this
  read the CPU-side scale and passed throughout; its comment now records that only a capture can see
  the drawn size. `docs/research/flame-kindle.md` is the note.

- **A carried fire's light now sits in its flame, and is sized for where it is.** `examples/courtyard`
  put every handed-out torch pool light **1.80 m above the flame drawn on the torch** — a light with no
  source, and one that delivered **72 lx** where its 15 402 lm were sized for **250 lx** at the pool's
  edge (1.79 stops, a ratio of 0.289). Measured on the baseline, a lit bearer's own ground was *darker*
  than open ground 10 m away (45.6 DN against 72.6 DN); it is now a torch pool (143.6 → 156.0 DN at the
  brightest ground block under the figure). One entity holds the `FlameSite` and the `PoolCandidate`,
  the offset is `flame_light_offset(height)`, and the lumens are sized from the same height the light is
  placed at. The README quick start and the crate's doc example teach the same shape.

- **A drawn flame breathes with the light it motivates.** `FlameSite::diameter` sizes the flame's own
  flicker through the same puffing law the light's uses, and `FlamePlugin` rewrites each flame's
  brightness every frame from **`model::fire::flicker_swing`** — the same function `flicker_lights` writes
  the light's intensity with, so the flame and the ground under it cannot drift apart. The brightness
  rides in the flame's `MeshTag` (low byte, 8 bits) beside its phase (next byte) and height (high 16).
  Measured on `examples/courtyard`: the flame's own pixels moved ±3.5 % while its pool moved ±15-22 %,
  and the two series correlated at **+0.07**; `docs/research/flame-flicker.md` is the note.

- **`model::fire::flicker_swing`**, the sized-or-un-sized flicker choice in one place:
  `Some(d)` is `flicker_factor_of` and `None` is `flicker_factor`.

- **`model::tag::set_flame_brightness`**, a tag with only its brightness rewritten, so the per-frame
  write does not round-trip the height and phase through its quantisation.

- **The flame's brightness is written only when its quantised byte moves.** `MeshTag` is in `bevy_pbr`'s
  `Changed<MeshTag>` re-extraction set, so a write every frame re-extracts every flame from the render
  world every frame. The byte holds 1/255 of the swing, so a 2.6 Hz fire skips some frames and a fire that
  never changes costs nothing. `src/flame.rs` has a test on the change tick, which is what that set keys on.

- **`PoolCandidate::phase` and `PoolCandidate::diameter`**, so a light the pool hands to a prop flickers on
  *that prop's* fire clock. The other half of the same defect: the 48 torch-bearers' pooled lights
  flickered at the slot's `1.3 · slot` while the flames drawn on them flickered at each torch's phase.
  `None` on both is the old behaviour, so a host that models no per-prop fires is unchanged. The two are
  resolved for the `pool.size` winners each frame, not carried through the rank list's sort: in the tuple
  they cost 3.4 % of the pool's own frame (`pool_p50_us` 493 → 511) for a number only the winners read.

- **`model::pool::preferred`**, the overlap rule on any "smaller is better" quantity: `true` when a
  challenger beats what is held by the `hysteresis` margin. `hand_off` is `preferred` on distances, and
  the pool's own holding decision uses it on its rank value, so ranking and holding are one rule.

- **`bench_lights` reports each hot system set separately.** A second line,
  `bench_lights_systems flicker_p50_us=… pool_p50_us=…`, times `LightsSystems::Flicker` and
  `LightsSystems::Pool` on their own by re-running the same app with a switch that leaves one enabled — so a
  change can be attributed to the system it moved rather than to "the frame".

- **The pool ranks a candidate by the light it contributes, not by its distance.** `LightPool::rank`
  defaults to `Rank::Illuminance`, which orders candidates by `intensity / distance²` — so a brazier at
  3 m burning 60 000 lm takes a real light from a torch at 2 m burning 8 000, where the old rule gave it to
  the torch. `Rank::Distance` restores the old behaviour. (The overlap rule now compares the same rank
  value — see *Fixed* — so this entry's "unchanged" no longer applies to it.)

- **A flame is redder while its fire catches.** The shader's ramp is pulled towards an ember red as a
  fire kindles and released as it takes, so the catch reads as a young fire and not as a steady one
  switching on. Driven by the same `kindle_gain` envelope as the light and the flame's growth, and carried
  to the shader in **each flame's own instance scale** — so one fire catching never tints another.

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

- **A flame held still while the light it cast moved.** Measured on `examples/courtyard`: over 400 frames
  a brazier's pool on the ground swung ±15-22 % peak to peak while the flame's own pixels swung ±3.5 %,
  and the two correlated at **+0.07** — the source that is supposed to motivate the light was not doing
  the motivating. The flame's emission now carries the fire's own flicker, from the same function and the
  same clock the light is written with.

### Changed

- **`pack_flame_tag` / `unpack_flame_tag` take and return the brightness**, and the tag's layout is now
  height 16 | phase 8 | brightness 8 (was phase 16 | height 16). The 16-bit phase was 1e-4 rad of
  resolution a fire cannot use; the byte it keeps is 2π/256 ≈ 0.025 rad, four milliseconds of clock at
  6 Hz. A tag packed before this change reads its height correctly and its phase from a byte that used to
  be part of the old phase, so a host that persisted tags across the upgrade would see its phases shift —
  nothing in the crate persists one; `dress_flames` packs each flame's tag at spawn.

- **The flame's brightness runs on virtual `Time`, its shape on `Time<Real>`.** The two were one clock
  before. The split is the one `fire-light.md` already states for the light: a paused world's fires hold
  still, and the drawing on them keeps moving. `flame.md`'s "a paused world's fires still flicker" is now
  about the silhouette only.

- **One newcomer reshuffled the whole pool, and up to seven of eight lights went dark for a frame.** The
  hand-off compared every slot against the *same* best free candidate, so a single bright prop displaced
  all of them at once; a displaced slot's fallback could then take a candidate another slot was still
  keeping. Measured on the courtyard, the pool's total fell 17 % in one frame and the street's torch-lit
  pools went to zero and faded back over twenty frames. A slot's owner is now decided in one pass before
  any slot is written, a challenger displaces the **worst-ranked** light the pool holds rather than the
  first slot to compare, and a candidate another slot holds is never a challenger. `tests/pool.rs`'s
  `one_new_candidate_displaces_one_light_and_no_more` fails on the old rule.
- **The overlap rule compared distances while the rank compared illuminance.** A pool ranked by
  `Rank::Illuminance` still held a light for a dim near torch against a bright far lantern, so the two
  rules disagreed and the bright source could be starved. Both now compare the pool's one rank value, via
  the new `model::pool::preferred`.
- **A hand-off the pool could not cross-fade was made anyway, and cut a fade already in flight short.** A
  reserve light is busy for the whole fade it carries; the pool now moves only as many lights per frame as
  it has free reserves, so a scene needing more moves gets them over the following frames instead of
  dimming its subject.
- **`hand_off` turned a `NaN` distance into a zero** (`f32::max` returns its other operand when one is
  `NaN`), so a broken distance read as "nothing held" or "at the focus" instead of keeping what was held.
  Both `NaN`s are now answered before the clamp.

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
