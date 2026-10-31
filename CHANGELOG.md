# Changelog

All notable changes to bevy_lights. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the crate is pre-1.0, so any release may break the API.

## [Unreleased]

No API break: the new items are additive (`FlameSmoke` is a new component, `FireKind::smoke` a new method, and `FlameLook`, `FlameSite` and `PoolCandidate` are untouched).

### Added

- **`FlameSmoke { density }`: a fire's lit plume.** A flame ended in a clean point against the dark; a wood or pitch fire goes on up as smoke, and in the dark that smoke is visible because the fire lights it. Add `FlameSmoke` (`density` `0.0..=MAX_SMOKE_DENSITY`, default 0.8) to a `FlameSite` and its flame gets one more camera-facing additive quad (`src/smoke.wesl`) holding a faint column that leaves the flame's tip, widens as a turbulent plume does (half-width `0.05 + 0.156 · (r − 0.35)` flame heights, from the entrainment coefficient 0.13 of Morton, Taylor & Turner), is lit by the flame with an inverse square, takes the fire's own hue (`FlameSite::color`) and thins into the dark. In a `FlameWind` it bends over downwind as a bent-over plume does (`z ∝ x^(2/3)`), so the wind shows from across the scene. It is a function of the shared clock (no particle state, the same on every machine), one draw per fire that asks and nothing for one that does not. The quad is the sparks' quad's geometry and carries the sparks' culling box (`ember_cull_extents`), so a fire whose smoke is off the screen draws none of it; it takes the flame's catch (a fire not yet caught smokes nothing, a snuffed one stops) and `depth_fade`. **Additive only**: a blended smoke sorts against every other transparent surface of the host, so it has no dark, occluding half and is a night effect (`docs/research/flame-smoke.md`). `docs/media/smoke.png`.
- **`FireKind::smoke()`**: a lantern's enclosed wick gives none, a candle `0.25`, a brazier `0.5`, a torch `0.8`, a campfire `0.9` and a bonfire `1.0` (design defaults), for `entity.insert(kind.smoke())` beside `kind.embers()`.
- **What it costs** (`docs/DEMOS.md`, interleaved against the build before it): a scene with no smoke is the same (`bench_flames`, 5 000 flames 2.23 → 2.23 ms a frame, 20 000 4.22 → 4.27); a smoking fire costs about 0.16 µs a frame on the CPU, the per-entity cost of the sparks' quad, and an on-screen column costs the GPU about 11 µs of the transparent pass at a 3 m close-up with 52 fires.
- **`MAX_SMOKE_DENSITY`, `FlameSmoke::problems`** (reported when a bad component is added, like the sparks').
- **`LIGHTS_SMOKE=d` for the courtyard example and `BENCH_SMOKE=1` for `examples/bench_flames.rs`**; the bench's line gains `smoke=` and `smoke_visible=`, the renderer's own count of the smoke quads it did not cull.

### Changed

- `tools/wesl-check` also compiles `src/smoke.wesl`, with and without the depth prepass.

## [0.4.0] - 2026-10-31

API break (`cargo semver-checks` against 0.3.1: 201 of 202 checks pass; `constructible_struct_adds_field` fails for `FlameSite.color`, `LightPool.focus` and `PoolCandidate.color`): a host that builds one of those three naming every field adds the new one (`color: None`, `focus: FocusRule::One`); a host that spreads `..default()` or `..FireKind::Torch.site()` is unaffected. Carries a fire's own colour, the pool for more than one viewer, the come-into-view flash fix, the pop fix for a light handed on mid-fade-in and the ember culling.

### Added

- **`FlameSite::color: Option<Color>` and `PoolCandidate::color: Option<Color>`: a fire of its own colour.** Every flame was the scene's one colour (`FlameLook::color`), its sparks a fixed orange, and every pooled light the template's, so a blue gas lantern, a green witch-fire or a violet shrine brazier among orange torches could not be drawn with its flame and its light one colour. `FlameSite::color` is the middle of that flame's ramp and the rest is the same flame in that hue: a rim darker and more saturated, a core paler, the catch's colour and the sparks' three colours too. `PoolCandidate::color` is the colour of the light the pool hands the prop: the light takes it on the frame it changes owner (it is fading in from nothing then), the light fading out keeps its owner's, and a candidate with none gets back the colour the light had (the template's, or a host's palette). `None` on both is exactly what was drawn before, to the pixel. Every distinct colour is one more material, shared by its flames and released with the last of them (`docs/research/fire-colour.md`).
- **`retint(stop, mid)`** (engine-free, `model::fire_colour`, re-exported flat) with the shipped ramp's stops (`DEFAULT_MID`, `EDGE_RED`, `CORE_YELLOW`, `EMBER_RED`, `SPARK_DULL_RED`, `SPARK_ORANGE`): the orange ramp moved to a fire's hue, with each stop's luminance kept in the orange ramp's ratio to the middle's (a rim dimmer and a core brighter than the body whatever the hue). It is the identity for every red, orange and yellow and turns all the way by 60° of hue beyond them.
- **`LIGHTS_COLOURS=1` for the courtyard example**: braziers 1 and 2 burn blue and green and some torches blue or violet, flame, sparks and light together. `docs/media/colours.png`.

- **`ember_cull_extents(height, took, lean)` and `ember_reach(lean)`** (engine-free, in `model::tag`, re-exported flat): the culling box of a flame's ember quad, the counterpart of `flame_cull_extents` and `FLAME_REACH`. The sparks' quad is wider than the flame (3.03 flame heights from the foot in still air, 5.27 at the lean limit, because it widens downwind by the drift a spark has at the top of its rise), so the flame's box cannot be reused.
- **`BENCH_EMBERS=1` for `examples/bench_flames.rs`** gives every torch `FlameEmbers`, and the printed line gains `embers=` and `embers_visible=`, the renderer's own count of the ember quads it did not cull.
- **`LightPool::focus: FocusRule`: the pool for more than one viewer.** `FocusRule::One` (the default, and what the crate did) needs exactly one entity carrying the focus marker; with two or more the pool is dark and the warning now names the setting that serves several. `FocusRule::Shared` takes any number, one per viewer (local split-screen co-op, a map or cutscene camera beside the player's): the pool's `size` lights are shared between them, each focus entitled to `ceil(size / foci)` of them, ranked by `LightPool::rank` among the candidates within `reach` of it, and a candidate's place in the one ordering is its best place among the viewers it is near, measured against each viewer's own share. A viewer among twenty fires cannot take the lights of one among three; a viewer that does not use its share leaves it to the others; a fire between two viewers holds one light. A viewer joining or leaving re-divides the lights through the pool's ordinary hand-off: with `reserve` at least the number of lights that change owner (two, when one of two viewers leaves a pool of four) the pool's total brightness holds at every frame, and with fewer the rest fade in (measured: `reserve: 1` dipped one light of four and recovered over 20 frames). With exactly one focus `Shared` is `One`, to the bit. The pool is shared and not multiplied: a host whose budget is per view raises `size` itself. `FocusRule` is `Reflect` and serialises (a file saved before the field existed loads as `One`). **Breaking for a host that builds a `LightPool` naming every field**, with no `..default()`: it must add `focus` (the crate's own round-trip test was such a literal), so the release that carries this is a minor bump. `docs/research/pool-foci.md`.
- **`focus_quota(size, foci)`, `quota_cutoff(values, quota)` and `shared_value(value, cutoff)`** (engine-free, in `model::pool`, re-exported flat): the maths of the shared pool, for a host that ranks its own candidates.
- **`LIGHTS_FOCI=n`, `LIGHTS_SPLIT=1` and `LIGHTS_FOCUS_RULE=one` for the courtyard example**: `n` foci (cyan markers) over one camera; two viewers each with its own camera on half of the window; and the default rule over the same foci, which leaves the pool dark (the comparison). `BENCH_FOCI=n` for `examples/bench_lights.rs`.

### Changed

- **A fire's sparks follow its colour.** The sparks' three colours were constants in `src/embers.wesl` and ignored `FlameLook::color`; they are now the shipped orange ramp moved to the fire's hue, which for every red, orange and yellow is the same orange as before (the courtyard's captures differ by zero pixels) and for a blue or green scene is blue or green sparks.
- **A scene whose `FlameLook::color` is a green, blue, violet or cyan now has a ramp in that hue.** The flame's rim and core were orange constants whatever the middle colour, so a blue scene had a blue body with a red rim and a yellow core; they are now the same flame in the scene's hue (`retint`). Every red, orange and yellow `FlameLook::color` keeps the shipped rim and core exactly.
- **Breaking: `FlameSite` and `PoolCandidate` each gain a `color` field**, so a host that builds either naming every field adds `color: None` (a host that spreads `..default()` or `..FireKind::Torch.site()` is unaffected). `FlameMaterial` and the embers' material gain the ramp's stops as uniforms in place of shader constants.
- **A fire whose sparks are off the screen no longer draws them.** The ember quad carried `NoFrustumCulling`, so every `FlameEmbers` fire in the world was extracted, sorted and drawn every frame, on screen or not. With every torch embered and about twenty in view, `bench_flames` read (best of three alternating runs, in milliseconds a frame, on a machine at load average 15-26) 11.5 and 37.2 at 5 000 and 20 000 flames before and 4.2 and 9.2 after (3 m/s wind, 5 000 flames: 8.8 before, 3.6 after), and the quads in view fell from every one in the world to 18-21. The quad now has a box of its own that `update_embers` rewrites on the frame the flame's scale changes. Nothing visible changes: the courtyard's captures with sparks on are bit-identical to the old build's. `docs/research/flame-culling.md` has the geometry and the numbers.

### Fixed

- **A light handed on while it is still fading in no longer pops.** A slot whose owner changes twice inside one fade-in (the arriving light at a few per cent, a nearer challenger takes it) handed the reserve that fades it out its owner's *full* intensity, so the pool's light jumped by nearly a whole light for one frame and then faded back out: in the courtyard (`LIGHTS_FOCUS_WALK=18 LIGHTS_POOL=4 LIGHTS_SEED=3 LIGHTS_HYSTERESIS=1.0`) +57 000 lm of 943 000 at frame 235, a +4.5 DN step in the whole frame's mean red and a patch on two pillars and the floor beside them, the same in the build before this change. The departing light now starts from the level its owner's light was shown at (`a_light_handed_on_while_it_is_still_fading_in_fades_out_from_the_level_it_showed`: a jump of 94 in a pool of 100 before, none after). The frame sequence's step is now +0.02 DN.
- **A light a camera turns to is drawn at its fire's brightness, not at the one it was last given.** `flicker_lights` leaves a culled light alone (nothing is extracted for it), and Bevy recomputes `ViewVisibility` in `PostUpdate`, after it: a light the camera moved over was still "culled" when `Update` ran and was extracted that same frame at the intensity it had when it was last seen. For a pool light handed to a new owner while out of view that is the old owner's: the courtyard with two moving cameras (`LIGHTS_SPLIT=1`) drew one torch's light for one frame at 75 903 lm where its fade-in was at 104, a 5.6 DN jump of a half-frame's mean luminance (frame-to-frame changes of that mean are now at most 0.9 DN over the same 180 frames); a light spawned in view takes the same path and, by reading the code, was drawn once at whatever intensity its `PointLight` was built with (not captured). `FlickerPlugin` now has a `PostUpdate` system, after the visibility passes, that writes the intensity of every light whose `ViewVisibility` has just turned true (`Changed<ViewVisibility>`): a culled light still costs nothing, and the frame it is seen it is as bright as its fire is. A light that is out of view stays untouched, as before.

## [0.3.1] - 2026-10-30

No API break (`cargo semver-checks` against 0.3.0: 202 checks pass); two public items added. Carries the fixes of issues #8, #9, #10 and #11 and flame culling.

### Added

- **`examples/bench_flames.rs`: what thousands of flames cost when few are on screen.** `BENCH_FLAMES` torches (default 5000) on a 3 m grid, one camera looking at a corner of it (about twenty in view), rendered off-screen for `BENCH_FRAMES` frames with no window; it prints `bench_flames flames=… visible=… wind=… frames=… mean_ms=… p50_ms=… min_ms=…`. `visible` is the renderer's own count of the flames it did not cull. `BENCH_WIND=x,z` makes every flame lean, which rewrites each one's scale and culling box every frame. It needs a GPU adapter (`bench_lights` has no renderer and cannot see a flame).
- **`flame_cull_extents(height, took, lean)` and `FLAME_REACH`** (engine-free, in `model::tag`, re-exported flat): the half-extents of the box, in the flame entity's own axes, that a frustum test must use to hold everything the flame's vertex stage draws. A host that draws its own flame quads with `FlameMaterial` needs the same box.

### Changed

- **A flame outside every camera's view is no longer extracted or drawn.** A flame carried `NoFrustumCulling`, because Bevy's box for a billboard is the unit square's and the vertex stage draws something else, so every `FlameSite` in the world was drawn, and re-extracted whenever its brightness byte changed (most frames), wherever the camera looked. That cost 1.3 µs a flame a frame, linear, with none in view: with about twenty of them in view `bench_flames` read, in milliseconds a frame (best of three alternating runs on a loaded machine) 2.2, 7.8 and 30.0 at 1 000, 5 000 and 20 000 flames, and now reads 1.9, 2.2 and 6.4 (`visible=15`); with a 3 m/s wind, 5 000 and 20 000 flames went from 8.0 and 28.1 ms to 3.1 and 5.9. A flame now carries its own `Aabb` and `NoAutoAabb`: a cube about its foot of side `2 · FLAME_REACH · size`, **divided by the instance scale**, because the catch and the wind's lean are the scale's `x`, `y` and `z` and Bevy multiplies a box by its entity's scale. The box is rewritten on exactly the frames the scale is. Nothing visible changes: eight cameras and two 94-frame sweeps that carry every fire across both frame edges (one with wind, gusts, and every fire put out and re-lit) differ from the old build by at most 1 DN and by no pixel over 8 DN. The test `a_flame_any_part_of_which_is_on_screen_is_never_culled` walks 40 000 random flames and cameras against Bevy's own frustum test and fails if `FLAME_REACH` is below 1.64 or the scale is left undivided. `FlameEmbers` still draws its spark quad whether or not the fire is on screen (`docs/research/flame-culling.md`).
- **A `FireKind`'s pooled light is handed to its flame's middle, not its foot** (issue #9). `FireKind::candidate` and `fire` built a `PoolCandidate` with `offset` zero and told the host to spawn it on the `FlameSite` entity, whose origin is the flame's foot, so the pool put the light at the foot; the wind's lean (`lean_light_shift`, the move of the flame's *middle*) then sank it `height / 2` straight below the flame, under the coal bed (27 cm under a brazier's foot at 3 m/s, where the flame's middle is 28 cm over it). In the scratch scene a fire spawned from a bundle lit almost nothing (the ground round it measured 1-5 DN, a candle to a bonfire), and with the light in its flame it measures 5-160 DN. The candidate's `offset` is now `flame_light_offset(height)` (0.175 m for a torch, 0.55 m for a brazier); `over` is still the light's height, which is now the flame's middle. The signature is unchanged and a host that wrote its own candidate (the courtyard, the README quick start) is untouched, but a host that spawned a `FireKind` bundle sees its light move up by half the flame's height. The `PoolCandidate::offset` sentence that said a candidate on the `FlameSite` entity "needs no offset at all" is gone, and the `FlameWind` and README sentences saying the fire's light does not move with the lean are corrected: the pool has carried it with the lean since `lean_light_shift`. `docs/research/fire-kinds.md`.

### Fixed

- **The pool no longer slows down as fires go out** (issue #11). `assign_pool` collected every finished `Snuff` into a list and searched it for every candidate, before the reach filter, so its cost was candidates × fires out: with all 5000 candidates out (a town at dawn, a saved scene whose torches were put out) 12.5 million entity comparisons a frame, and `bench_lights` read `pool_p50_us` 6.1-11.3 ms where a lit town reads 0.42 ms. It now reads each candidate's `Snuff` with the candidate, a column Bevy matches once per table: all out, `pool_p50_us` is 0.22 ms (whole frame `min_us` 5.6 ms to 0.27 ms), and with no fire out it is unchanged (417 µs best of twelve runs both ways). A fire that is lit, going out or out is assigned exactly as before. `bench_lights` takes `BENCH_SNUFFED=<n>`, which puts the first `n` candidates out, so the figure can be re-measured. `docs/research/pool-fire-state.md`, iteration 276.
- **Turning the pool's shadows off reaches the reserve lights** (issue #10). A change of the template's `shadow_maps_enabled` or of `max_shadowed` applied to the slots at once but not to a reserve already fading a shadowed light out, which kept casting until its fade ended (up to `fade_seconds`, 0.35 s of shadow cube maps the setting said were off), and a departure made on the frame the flag went off handed a new reserve the old shadow too. A reserve now casts only while the pool's shadow budget is above zero, so with the flag off, or `max_shadowed: Some(0)`, no pool light casts on that frame and the reserve's light still fades out. A cap lowered to a number above zero reaches the slots at once and leaves a reserve fading out its shadow, inside the `max_shadowed + reserve` a fade already allows. `docs/research/pool-shadows.md`, iteration 275.
- **A cross-fade's departing light stands where its light is.** The reserve light fading a light out was a snapshot of last frame's position without the wind's lean, and it was never updated, so a shadow cross-fade of a leaning torch held half a metre of the light upwind of its flame (0.51 m for a metre-tall torch in a 3 m/s wind) and one of a walking torch-bearer left the departing end a step further behind every frame (1.0 m by the end of a fade at 3 m/s); under wind every ordinary hand-off also made the departing light jump from the flame's leaned middle back to the upright axis. The reserve now re-reads its candidate's light point (position, offset and lean, as the arriving light works them out) every frame while the candidate is in the pool, so both ends of a cross-fade of one light stand on one point and a displaced owner's light keeps moving with its torch; once the candidate has left the pool it stays at the last point it followed it to (`docs/research/pool-shadows.md`, issue #8).

## [0.3.0] - 2026-10-30

The first release on crates.io since 0.2.19: 0.2.20 and 0.2.21 were tagged but crates.io refused both (too many versions in 24 hours), so their entries below ship here too. Breaking: `LightPool` gained a public field (`max_shadowed`), so a struct literal that names every field needs `..default()` or the new field.

### Added

- **The pool's shadow budget: `LightPool::max_shadowed`.** With the template's `shadow_maps_enabled` on, `Some(n)` gives a shadow to the `n` best-ranked lights the pool holds (the closest and brightest, RTR4's rule for many lights) and lets the rest light without one; `None`, the default, is every light, as before. Every shadowed point light renders six shadow views a frame, about 0.6 ms a light on an RTX 3090 in the courtyard, so the cap is what lets a sixteen-light pool afford shadows at all. A shadow is held with the pool's own `hysteresis`, and a light that earns or loses one because the ranking moved is cross-faded to itself through a free reserve: in a still scene the arriving shadow deepens over 21 frames (largest step 0.35 DN against a 4.8 DN pop) while the lit ground around it does not move. A change to the cap or the flag applies at once. A pool with shadows wants `reserve: 2`. Saved by serde. `docs/research/pool-shadows.md`.
- **`PoolBudget::shadowed` and `casts_shadow(entity)`**: which held candidates' lights throw a shadow this frame, for an editor to show beside which are real.
- **`shadow_set(holders, budget, hysteresis)`**: the engine-free rule that picks them.
- **`LightPool::problems` names a template `light.shadow_map_near_z` that is not positive**: it is the near plane of each shadow cube face, and at 0 the face writes no depth.

### Changed

- **The courtyard's figures cast shadows**, from the braziers and from the pool's three shadowed lights (`LIGHTS_SHADOWED`, default 3; the example's `LIGHTS_RESERVE` default is now 2). `LIGHTS_POOL_LOG` marks a shadowed light with `*`.
- **The courtyard's sky fill (its camera's `AmbientLight`) is 20 cd/m², from 2**, so a figure's shadow thrown by a fire onto a wall the moon misses reads navy (`srgb(4,9,32)`) instead of black (`srgb(1,1,5)`); other shadows rise 2-3 DN. `docs/research/lighting-look.md`.
- **A saved `LightPool` keeps every setting of its template light but `intensity`**: the shadow's biases and near plane, `contact_shadows_enabled`, `affects_lightmapped_mesh_diffuse` and, where Bevy's `experimental_pbr_pcss` feature gives the light one, `soft_shadows_enabled`, besides the colour, range, radius and shadow flag it saved before. A file that names none of them takes Bevy's defaults.

### Fixed

- **A cross-fade no longer dips mid-fade.** The departing light's fade was read one frame ahead of the arriving light's, so every cross-faded hand-off sagged by up to 7 % of a light at its steepest; the two now run on one clock and sum to the light exactly.
- **A reserve light is put out when its fade ends.** It stayed visible at zero intensity for the rest of the run, a light the renderer still clustered and shaded.
- **A light shown this frame is written this frame.** `FlickerPlugin` skipped a light whose `ViewVisibility` was false, and on the frame a light is switched on that is still last frame's value, so it rendered one frame at a stale intensity. A pool slot re-lit after standing empty did this; once reserve lights went out at the end of their fade (above), so did every cross-fade, opening with a one-frame 6 % drop of the pool's total (a reserve at 112 lm instead of 70 000) until this fix.
- **A reserve fading a light out keeps that light's flicker rate and depth**, not just its phase, so the two ends of a cross-fade of a sized fire breathe together.
- **Every setting of the pool's template light reaches every pool light live.** A change to `LightPool::light` copied only its colour, range and radius onto the lights already burning, while a light spawned later took the whole template, so a host that changed the shadow bias, the near plane or `contact_shadows_enabled` at runtime saw nothing until the pool grew, and then two kinds of light side by side. Now every field but `intensity` and `shadow_maps_enabled` (the pool's own, per light) is copied onto every light, reserves included, on the next frame (`docs/research/pool-template.md`).

## [0.2.21] - 2026-10-30

### Added

- **`ground_ramp(lumens, over, dim_lux, bright_lux)`**: the `(peak, reach)` of a linear ramp `peak * (1 - d/reach)` that crosses the true ground illuminance exactly at the `dim_lux` and `bright_lux` circles (`ground_reach`). For a host's coarse illuminance grid (a stealth or AI tile map baking each fire as a disc that fades linearly): a ramp fitted from the spike's peak over-reads the pool's middle several-fold, one fitted at the two thresholds a grid is read at draws the circles the ground shows. Plain data in world units and lux; no grid is named. `docs/research/fire-reach.md`.

- **The pool's light follows a flame's lean.** With a `FlameWind`, the pool moves the light of a candidate on a `FlameSite` entity to the leaning flame's middle (downwind and slightly lower) by the new `lean_light_shift(height, speed, diameter, toward)`; still air, no wind resource or no `FlameSite` change nothing. `docs/research/flame-lean-light.md`.

- **`FlameEmbers`: sparks that leave a flame.** Add the component (`rate`, sparks a second, `0..=MAX_EMBER_RATE`, default 3) to a `FlameSite` and its flame gets one more child, a camera-facing additive quad (`src/embers.wesl`) on which sixteen hashed slots each emit a spark per cycle: a capsule along its own velocity that climbs under drag, weaves, drifts downwind with the flame's lean and cools yellow to orange to red (Newton cooling). No particle state, no buffers; the quad inherits the flame's catch and lean, honours `depth_fade`, and sites without the component are unchanged. `FlameEmbers::problems()`, serde under the `serde` feature, `LIGHTS_EMBERS=r` in the courtyard. `docs/research/flame-embers.md`.

- **`FireKind::embers()`**: the sparks a kind of fire sheds, `None` for the candle and the lantern, rising from the torch (2 a second) through the brazier (the default 3) and campfire (5) to the bonfire (`MAX_EMBER_RATE`). A design default; `docs/research/fire-kinds.md`.

- **`FireKind::fire(lux, over)`**: the flame and its light as one bundle, `(FlameSite, PoolCandidate)`, so a picker spawns both from one call and a flame never goes without the light the pool hands it (the sparks stay `embers()`, absent for a candle). No new numbers. `docs/research/fire-kinds.md`.

- **A bad setting is reported when it arrives.** `Flicker`, `Snuff`, `FlameSite`, `FlameEmbers` and `PoolCandidate` call their own `problems()` as they are added and log one `warn!` per type (the first bad one, with the entity and each field), so a scene file or BRP call that writes a `NaN` phase no longer just goes dark in silence. Nothing is refused or clamped. `docs/research/settings-report.md`.

- **A bad settings resource is reported when it is written.** `LightPool`, `FlameLook`, `FlameWind` and `FlameGust`, the resources a panel edits live, are checked on every frame they change and warn once per distinct set of problems, so a slider dragged through `NaN` is one line, not one a frame, and a fix followed by a new mistake warns again. Nothing is refused or clamped; a missing optional resource is silent. `docs/research/settings-report-resources.md`.

### Fixed

- **The flame seen from straight above has a body, not a flat ring.** The overhead cloud's heat was its core alone, so the rest of the disc was one deep orange inside a red rim; it now has an orange-yellow body heat mottled by the flowing billows, red only at the rim. Visible from about 2-3 m above a fire; at 45 and 66 degrees every pixel is as before. Design default, judged in captures; `docs/research/flame-overhead.md`, round 23.

- **The flame seen from above stands on its coals.** Between about 66 and 85 degrees of camera elevation the overhead cloud hovered half a flame above a bed that was lit and crisp below it. The cloud now has a second, smaller billow at the flame's foot, placed by the same screen offset the tongue's seat uses and faded in with the overhead blend; at or below 50 degrees every pixel is as before (AE 0.003 over 1280x720 at 30 and 45 degrees). Design defaults, judged in captures; `docs/research/flame-overhead.md`, round 17.

- **A bad `Kindle` is reported when it arrives.** `Kindle` had `problems()` but was the one settings component the add-time warning (`Flicker`, `Snuff`, `FlameSite`, `FlameEmbers`, `PoolCandidate`) did not cover, so a saved scene's `NaN` or negative `elapsed` wrote a bad gain in silence. `FlickerPlugin` now checks it like the rest. Test `a_kindle_with_a_bad_elapsed_is_named_and_a_fresh_one_is_not`.

- **`PoolBudget` is flagged changed only when the schedule changes.** The pool wrote it through `ResMut` every frame, so `resource_changed::<PoolBudget>` (an editor's panel, a game's "pool too small" warning) fired every frame. It now compares and writes only when a slot changes owner or the count of wanting candidates moves. Test `the_budget_is_flagged_changed_only_when_the_schedule_changes`.

- **`Flame` is `Reflect`.** The drawn flame was the one public component the `AGENTS.md` rule ("everything public is `Reflect`") missed, so a BRP client or an editor panel could not read its catch (`kindled`) or its torch (`source`). It now derives `Reflect`, registers as a component and is registered by `FlamePlugin`.

- **A light linked by `FlameSite::fire` leans with its flame.** The pool's lean moved only a candidate that was itself a `FlameSite` entity; a torch whose light is a sibling of its flame, joined by `FlameSite::fire`, kept its light upright beside a bent flame. The site that names the winner now supplies the lean. `docs/research/flame-lean-linked.md`.

### Changed
- **The overhead flame parts into masses** (`src/flame.wesl`): seen from above at close range the cloud is a few lit masses with darker clefts that show the coal bed, not one even mottle; the core stays whole. Overhead branch only; 45° and 66° are unchanged.

- **The flame from straight above is as wide as its bowl's fire.** The overhead cross-section's mean radius is `1.2` of the flame's half-width (was `0.85`): from above the fire fills about two thirds of a brazier bowl instead of a third, and the spiky burst is a round billow. Cameras at or below 50° are unchanged. `docs/research/flame-overhead.md`, round 20.

- **The flame from above flows outward instead of drifting.** Over a brazier seen from above, the disc's brightness was one noise dragged in a single direction (a measured 0.21 px a frame, which reads as wind or a conveyor); it is now two layers whose features swell about the flame's axis, cross-faded half a cycle apart (`OVERHEAD_FLOW_RATE`, `OVERHEAD_FLOW_GROWTH` in `src/flame.wesl`), and the disc is mottled from its middle instead of only its rim. Frames at and below 50 degrees are unchanged. `docs/research/flame-overhead.md`, round 18.

- **A catching or dying flame is taller for its share of the light.** The flame's (and its sparks') drawn size was its catch, `took`, in a straight line, so its area was `took²` against a light that goes with `took`: a third of the way through a catch the light was a third up and the flame a sliver. The size is now `took^(2/3)`, the intermediate-regime law for flame height against heat release rate (Delichatsios 2007), so a settled flame (`took` 1) and a put-out one (0) are exactly as before and the middle moves up (0.3 draws 0.45, 0.5 draws 0.63). On the courtyard's relight the flame shows about one saved frame (33 ms) earlier. `docs/research/flame-catch-size.md`.

### Documentation

- The courtyard example's `LIGHTS_GPU_TIMING=1` prints mean GPU time per render pass; with it the ember quad measures about 0.1 ms a frame (transparent pass 11 µs without embers, 107 to 137 µs with, RTX 3090, 52 flames).
- `docs/media/relight.gif`: the courtyard put out and lit again (`LIGHTS_SNUFF`, `LIGHTS_RELIGHT`), the first README media that shows a flame catching with its light. The README's note that a flame's catch cannot be shown is narrowed to the opening clip.

## [0.2.20] - 2026-10-30

### Fixed

- **The flame from above is a billowy cloud, not a starburst.** The overhead cross-section's brightness and heat were functions of the direction from the flame's axis, which drew rays out of the core. They now follow position, the edge ramp is softer and the core is round and mottled. Frames at or below 50° are unchanged. `docs/research/flame-overhead.md`, round 15.

## [0.2.19] - 2026-10-30

### Fixed

- **No hard line across the coal bed from 60° up.** The tongue's heat stopped dead at its seat while its mask was ramped, so where the overhead disc overlaps the seat the colour stepped from cream to orange in two pixels. The heat now leaves with the same foot ramp, blended in with the overhead factor; frames at or below 50° are unchanged. `docs/research/flame-overhead.md`, round 14.

## [0.2.18] - 2026-10-30

### Changed

- **The flame's core burns hotter as the camera rises.** From about 50° of elevation the hot core's emission is raised by up to 90 % (`OVERHEAD_CORE`, `src/flame.wesl`), so a fire seen from above out-shines the ground it lit and the core reads cream-yellow instead of grey-green. Frames at or below 50° are unchanged. `docs/research/flame-overhead.md`, round 13.

## [0.2.17] - 2026-10-30

### Changed

- **The flame holds its brightness through the 60-70° camera elevations.** The upright tongue's overhead dimming now follows the fifth power of the overhead blend instead of the blend itself, so the flame no longer goes pale before the cross-section has arrived; frames at or below 55° and at 76° and past are unchanged (`docs/research/flame-overhead.md`, round 12).

## [0.2.16] - 2026-10-30

### Added

- **`Kindle` and `Snuff` on a `PoolCandidate`: a pooled torch can be lit and put out.** A candidate entity (one with no `Flicker` of its own) that carries `Kindle` or `Snuff` now has them advanced by the crate, announced as `Kindled` / `Snuffed` for the candidate, and the light the pool lends is `intensity · kindle_gain · snuff_gain`, the product a `Flicker` light uses. A `FlameSite` whose `fire` names the candidate follows through the existing link. A candidate whose snuff has finished wants no light: its slot is freed (`PoolBudget::wanting` drops). Ranking keeps using the plain `intensity`, so a dying fire keeps its slot until it is out. A pool whose candidates carry neither pays nothing extra. The courtyard's `O` / `K` / `LIGHTS_SNUFF` / `LIGHTS_RELIGHT` now reach the 48 bearers' torches too. Design defaults, `docs/research/pool-fire-state.md`. Nothing existing changed.

## [0.2.15] - 2026-10-30

### Added

- **`Snuff` and `Snuffed`: a fire that goes out.** Insert `Snuff::default()` on a `Flicker` light and its intensity falls to exactly 0 over `SNUFF_SECONDS` (1.2 s) on `snuff_gain` (a smoothstep down, no flare); the flame of any `FlameSite` whose `fire` names that light shrinks on the same curve, so light and flame leave together. `Snuffed { entity }` is written once when it is out, for the hiss and the smoke. Remove `Snuff` and insert `Kindle::default()` to light it again; `Snuff` wins over `Kindle`. `Snuff::problems` validates it like `Kindle`. The courtyard example puts its braziers out with `O` or `LIGHTS_SNUFF=<seconds>`. Design defaults, `docs/research/flame-snuff.md`. Nothing existing changed.

## [0.2.14] - 2026-10-29

### Added

- **`ground_lux(lumens, distance, over)` and `ground_reach(lumens, lux, over)`**: the illuminance on flat ground and the radius of its iso-lux circle, with the cosine term `fire_lux` and `fire_reach` leave out (they are the square-on law a light is sized by). Use these for an overlay or gizmo painting the ground; at 4 m out and 0.9 m up the ground gets 0.22 of the square-on number. No existing number changed; the docs of `fire_lux` and `fire_reach` now say which surface they mean.

## [0.2.13] - 2026-10-29

### Added

- **`FireKind::candidate(lux, over)`**: the `PoolCandidate` for a kind, sized like `FireKind::light` and carrying the kind's diameter and depth, so a placed fire's three components (`flicker`/`light`, `site`, `candidate`) all come from one name. No numbers changed.

## [0.2.12] - 2026-10-29

### Added

- **`PoolCandidate::problems()` and `Kindle::problems()`**: the same field-by-field report `Flicker` and `LightPool` give, for the two components an editor saves with a scene (`offset.x`, `intensity`, `phase`, `diameter`, `depth`; `elapsed`). The crate still never refuses a value. No numbers changed.

## [0.2.11] - 2026-10-29

### Added

- **`fire_lux(lumens, reach, over)` and `fire_reach(lumens, lux, over)`**: `fire_lumens` read back, the ground illuminance a placed fire gives and the radius of its iso-lux circle, for an editor overlay or a gizmo. No numbers changed.

## [0.2.10] - 2026-10-29

### Added

- **`FireKind` implements `Display` and `FromStr`** (case-insensitive, trimmed), with `UnknownFireKind` listing the valid names, so a text field, command line or environment variable picks a kind. No numbers changed.

## [0.2.9] - 2026-10-29

### Added

- **`FireKind::reach()`, `lumens(lux, over)`, `light(lux, over)`.** Each kind now carries the metres of ground it lights to its pool's edge (Candle 1, Lantern 1.5, Torch 2, Brazier 4, Campfire 5, Bonfire 9), so a picker places a fire with no lumen number typed: `lumens` is `fire_lumens` at that reach and `light` a `Flicker` averaging it. Brazier and Torch are the courtyard's own pool constants (which now read them); the rest are design defaults (`docs/research/fire-reach.md`).

## [0.2.8] - 2026-10-29

### Added

- **`FireKind`.** Six named fires (`Candle`, `Lantern`, `Torch`, `Brazier`, `Campfire`, `Bonfire`; `FireKind::ALL`, `Reflect`, serde as the name) that bundle a fuel-bed `diameter()`, a flicker `depth()` and a flame `height()`, with `rate()`, `name()`, `flicker(mean_lumens)` (a `Flicker` via `at_mean`) and `site()` (a `FlameSite`) built from the same numbers so a light and its flame agree. Made for an editor's fire picker; the brazier row is the courtyard's existing numbers, the rest design defaults (`docs/research/fire-kinds.md`). The courtyard takes `LIGHTS_KIND=<name>`.

## [0.2.7] - 2026-10-29

### Added

- **Gusts.** A new `FlameGust { mean, intensity, timescale, seed }` resource (`problems()` and serde like `FlameWind`) makes `FlamePlugin` write `FlameWind` each frame from `model::gust_velocity`: the mean wind times `1 + intensity · x(t)` with the heading turned by `intensity · y(t)` radians, `x` and `y` independent zero-mean unit-variance stationary processes (`model::gust_unit`, a stateless sum of 12 sinusoids under a one-pole spectrum, deterministic, computed in `f64` so it stays smooth after hours). Defaults `GUST_INTENSITY` 0.25 and `GUST_TIMESCALE` 2 s are design defaults; the Gaussian model and the meaning of intensity come from Hannesdóttir et al. 2019 (`docs/research/flame-gust.md`). Without a `FlameGust` the plugin leaves `FlameWind` alone. `courtyard` takes `LIGHTS_GUST=I`.

## [0.2.6] - 2026-10-29

### Added

- **Flames lean in the wind.** A new `FlameWind { velocity }` resource (metres per second on the XZ plane, `problems()` and serde like `FlameLook`) tilts every flame downwind by `flame_tilt(speed, diameter)`: Welker & Sliepcevich's pool-fire tilt law with Pritchard & Binding's constants (`docs/research/flame-wind.md`), using each fire's own `diameter` (`DEFAULT_FIRE_DIAMETER`, 0.2 m, when it has none). The flame turns about its foot, is capped at `FLAME_TILT_LIMIT` (70°, a design default) and is drawn upright from straight overhead. The lean rides on each flame's instance scale (`y`/`z` carry `1 + sin θ · direction`), so the shared material and the one-draw-per-flame cost are unchanged. The `courtyard` example takes `LIGHTS_WIND=x,z`. With no `FlameWind`, or a zero one, every flame is drawn as before (the 1920x1080 captures differ by under 3 pixels, the GPU's own noise).

## [0.2.5] - 2026-10-29

### Changed

- **`Flicker`, `Kindle` and `FlameLook` load from a partial file**: a saved field that is absent takes its default, as `LightPool` already did, so a project written before a field was added still loads. No visual or default change.

## [0.2.4] - 2026-10-29

### Added

- **`Flicker::intensity_at(t, kindle)`**: the lumens a fire gives at a virtual time and kindle age, the same function `flicker_lights` writes each frame (which now calls it). An editor draws a fire's curve, or scrubs a timeline, with no app running. No visual or default change.

## [0.2.3] - 2026-10-29

### Added

- **`problems()` on `Flicker`, `FlameSite`, `FlameLook` and `LightPool`**, returning a `Vec<Problem>` (`field`, `kind: ProblemKind`; `Display` reads "`reach` must be greater than zero"). An editor that loads settings from a project file shows the bad field before it reaches a light. The crate still never refuses a value. No visual or default change.

## [0.2.2] - 2026-10-29

### Added

- **`LightPool` is `Serialize`/`Deserialize`** under the `serde` feature, so an editor saves the pool's budget with a project. The slot template `LightPool::light` is written as `color`, `range`, `radius` and `shadow_maps_enabled` (the fields the pool copies; `intensity` is ignored and reads back as 0), and any missing field takes its default. No visual or default change.

## [0.2.1] - 2026-10-29

### Added

- **`PoolBudget`**: a resource the pool rewrites every frame with the candidate each real slot holds (`holders`, `holder_of`, `holds`) and how many candidates asked for a light (`wanting`, `granted`, `starved`). An editor shows which torches are real lights right now; a game sees its pool is too small when `starved()` is above 0. No visual or default change.

## [0.2.0] - 2026-10-29

### Migration from 0.1

- **`Flicker { base, phase }` becomes `Flicker { base, phase, ..default() }`.** `Flicker` gained `diameter` and `depth`; spelled with `..default()` a literal absorbs this and any later field. `diameter: None` keeps 0.1's rate exactly. `Flame { kindled }` gained `source`, which a host never constructs.
- **`LightPool::rank` defaults to `Rank::Illuminance`.** A host whose candidates all burn the same lumens gets the same order as `Rank::Distance`; a host that mixes fires (a brazier and torches) will see the pool's choice change. Set `rank: Rank::Distance` for 0.1's order.
- New in 0.2: `Flicker::diameter`, `FlameSite::kindle`, `LightPool::{rank, reserve, hysteresis, fade_seconds}` and `model::cinematography`.

### Added

- `examples/courtyard` gains `LIGHTS_EV100=x`, the camera's exposure (default 6). At 9 the ground is no longer saturated salmon around a brazier, so a flame's own contrast can be judged. Example only; the crate is unchanged.

### Changed

- `docs/media/showcase.gif` re-encoded at 400 × 225 (was 480 × 270): 1.8 MB, under the README's 2 MB budget (was 2.4 MB). Same scene, same frames, caption shortened to fit.

### Fixed

- **No edge-on slab poking past the flame seen from straight above (`docs/research/flame-overhead.md`, round 10).** At 85°+ the upright tongue, edge-on, collapsed to a one-pixel horizontal line of its widest row that stuck out past the cross-section's ellipse beside the core. Its brightness now goes to zero between sin(elevation) 0.970 and 0.996; below 76° every term is unchanged (identical frames). Shader only.

- **A fire is still a fire from above (`docs/research/flame-overhead.md`).** An upright billboard presents `cos(elevation)` of its area, so a brazier's flame was gone by 75° of camera elevation and a tactical or top-down camera saw a bare coal bed. Past 50° the flame's quad now faces the camera through the plume's centre, the upright flame is drawn on it as the image it has on screen, and a rippled cross-section of the plume (an ellipse `R` wide and `R·sin(elevation)` tall) fades in by 76°, so the tongue turns into a round, hot-cored glow with no step. At or below 50° the frame is unchanged. Shader only: `src/flame.wesl`; no API, default or CPU cost changed.
- **`FlameSite::depth_fade` works under an orthographic camera (issue #6).** The occlusion margin was 3 % of reverse-Z depth, which is 3 % of the distance for a perspective camera but 135 ft for an orthographic one with `far` 6000, so a flame behind a wall never faded. The shader now converts the source's and the scene's depth to linear view z (`depth_ndc_to_view_z`) and hides the source when the scene is nearer by more than 15 % of the flame's drawn height, so any projection at any far plane agrees. Perspective results change slightly (the margin is now a fraction of the flame's height rather than of the distance); a torch flush on a wall still does not fade. `examples/occlusion.rs` gains `LIGHTS_ORTHO=1` / `LIGHTS_FAR`.

### Changed

- **The flame seen from above has licks, not a soft halo (`docs/research/flame-overhead.md`, round 7).** Past 50° of camera elevation the plume's cross-section's radius is a scrolling noise of the direction from the fire's axis and its brightness is streaked along the radius, with a crisper edge (`smoothstep(0, 0.26)` for `0.35`), so the outline has tongues that lean and part. Frames below 50° are unchanged (1 channel of 1 pixel). Design defaults, judged in captures.
- **`examples/courtyard`'s braziers are iron bowls, not boxes.** A revolved bowl on a stem with a bright lip, a soot lining that glows from the coals up to the rim, and a round coal bed sunk 0.05 m below the lip, so the showcase's central prop reads as a fire basket. Example only: no library code or default changed; `docs/media/showcase.gif` is re-encoded. `docs/research/brazier-bowl.md`.

### Added

- **A flame linked with `FlameSite::fire` catches with its light's `Kindle`.** Re-lighting a fire (inserting `Kindle::default()` over the light) flared the light from nothing while its flame, which had caught once at spawn, burned on at full height: a source that precedes its own light. While the named entity's `Kindle` runs (`elapsed < KINDLE_SECONDS`) the flame's drawn scale is `kindle_gain(elapsed)`, the light's own curve, floored at 0.001, and it is exactly 1 afterwards, even if the `Kindle` is removed mid-way. The site's own `kindle` seconds are not used while a link's kindle runs. A site with `fire: None`, a link to an entity with no `Kindle` or a settled one draws as before. `docs/research/flame-relight.md`.

- **`FlameSite::fire: Option<Entity>` makes a flame read its breath from its light (issue #4 criterion 3, issue #3).** A site carried its own `diameter`, `depth` and `phase`, so a flame and the light it motivates could be given two values and nothing objected; Sanctuary shipped that twice. With `fire: Some(light)` the flame's brightness uses that entity's `Flicker` (`diameter`, `depth`, `phase`) or, for a prop the pool lights, its `PoolCandidate` (`depth` defaulting to `FLICKER_DEPTH`, `phase` to the site's), read every frame, so `Flicker::set_depth` moves the flame too. The light need not be the site's parent. `None`, a despawned entity, or an entity with neither component keeps the site's own values, so a host that does not set it draws the same bytes. Breaking for code that builds a `FlameSite` literal: add `fire: None`. The silhouette's noise phase and the catch (`kindle`) do not follow the link. `docs/research/flame-fire-link.md`.

- **`FlameSite::depth_fade` fades a flame whose torch the camera cannot see (issue #2, ask 1).** A flame is taller than the thing that holds it, so a torch behind a wall edge left its flame hanging over the wall with nothing under it ("is that a torch on the other side of the wall?"). With `depth_fade: true` and a `DepthPrepass` on the camera, the flame shader reads the prepass depth at the flame's source (a third of the way up) over a 16-tap disc and multiplies the flame's emission by the share of taps the camera can see, so a wall edge crossing the source dims the flame over a few pixels instead of popping it. A site that leaves it `false` draws with the unchanged material and does no depth read; a camera with no `DepthPrepass` compiles the read out and draws every flame whole. `FlameSite` gains a field, so every struct literal needs `depth_fade: false`. Two shared materials (the faded one carries `params.z == 1`), moved between by `update_flames` when a site's flag changes. `docs/research/flame-occlusion.md`. The new `occlusion` example is the test scene; `courtyard` takes `LIGHTS_DEPTH_FADE=1`. `tools/wesl-check` now compiles the shader with and without `DEPTH_PREPASS`. Not done: `FlameSite::visible_from`, and a time envelope (the fade is soft in screen space instead).

- **`Flicker::at_mean`, `set_mean`, `set_depth` and `mean`, and `PoolCandidate::at_mean` (issue #5).** `base` is the fire's *peak*, so changing `depth` silently moved a fire's average (`1 − depth`). A fire built with `at_mean(mean, depth)` solves `base = mean / flicker_mean(depth)`; `set_mean` re-solves it for a light whose lumens are rewritten every frame; `set_depth` clamps like `flicker_depth` and keeps the mean (a put-out `base` of 0 stays 0). `base` stays the documented peak. The README quick start now builds its light with `at_mean`.

- **Documented, and tested, that a `FlameSite` is independent of any light (issue #3).** A fire whose light sits elsewhere (a sconce's pool held out over the wall, a window glow) is a light in one place and a site parented to the prop in another; sharing `phase`, `diameter` and `depth` keeps them breathing together. No `FlameAnchor` type was added: the site's own `Transform` and parent already are the host-declared origin, and a light with no site draws no flame. A regression test pins that the flame stands at the site's origin and leaves with its prop.

- **`Flame::source: Option<Entity>` names the torch a flame is the fire of (issue #2, ask 2).** It is the parent of the flame's `FlameSite`, so a host diagnostic answers "whose flame is that" with one component read instead of walking the hierarchy. `None` for a root site or a site nested under another site. `update_flames` keeps it current each frame (written only on change), so a reparented site is followed; a despawned parent takes its site and flame with it, so it never resolves to a reused slot. Breaking for anyone constructing `Flame { kindled }` by hand (the crate says a host never does). Occlusion fading and `visible_from` (asks 1 and 3) are not done: they need a depth-buffer or raycast design and GPU captures.

### Changed

- **A coal bed's lumps have a hot middle (`char_field`'s `PLATEAU` 0.92 → 0.75, `FIELD_MEAN` lower bound 0.42 → 0.38), and the `courtyard` example scales the bed's radiance by `BED_GAIN` 0.2.** At 2.2 m the braziers' bed read as a pale pink plate (mean RGB 223/145/113, 59 % of pixels at R 255): the full `coal_radiance` ramp is ~35 times the display's white at the example's exposure and Bevy's tonemapper rolls it toward white, which is where the blue comes from. The bed now reads as orange embers with dark char between them (measured 191/79/41, 14 % at R ≥ 250), each lump brighter in the middle than at its rim. `coal_radiance` is unchanged, so a host that sizes the lining's glow from it is unaffected; a host that draws the bed from it should apply its own gain at its own exposure. `docs/research/fuel-bed.md`, round 2.

- **The `courtyard` example's braziers link their flame to their coals (`fire: Some(coals)`), and `LIGHTS_RELIGHT=<seconds>` re-lights them once.** `K` re-lit a brazier's light but its flame burned on at full height, because the site had `fire: None`; with the link the flame catches with its light again (captured at 1280x720: flame absent at 0.1 s after the re-light, growing over the kindle, full by ~1.4 s). Example only; the default scene is unchanged.

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

- **`flicker_mean`, `CALM_FLICKER_DEPTH` and `WHIPPING_FLICKER_DEPTH`** (issue #4). `flicker_mean(depth)`
  is the mean of a fire's multiplier (`1 − depth`, read through `flicker_depth`), so a host writes a lumen
  budget against the fire's mean and solves `Flicker::base = budget / flicker_mean(depth)` instead of
  dividing by a literal `0.85` that goes stale when a fire chooses a depth. The two constants (`0.05`,
  `0.3`, design defaults) name the calm and gusting ends of `docs/research/flicker-depth.md`. Nothing
  existing changes. Not done: a `FlameSite` that follows its light's depth, which needs a site-to-light
  link the crate does not have (see issue #3).
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
