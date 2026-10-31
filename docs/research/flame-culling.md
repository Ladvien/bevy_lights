# Flame culling: a fire off the screen costs almost nothing

A flame is drawn for every `FlameSite` in the world, on screen or not. The crate's pitch is a scene with thousands of fires and a handful of real lights; the flames are the half of that scene the crate draws itself, and until iteration 277 every one of them was drawn, extracted and re-extracted every frame, wherever the camera looked. This note decides how a flame is culled, why the box the engine would give it is wrong, and what the box must be instead.

## Sources

| Source | What it says | Used for |
|---|---|---|
| `10.58709_niujhu.v8i3.1694` — Orisayemi & Afabor, lighting in Yoruba films, citing Bordwell & Thompson | the key light "from offscreen left throws a shadow on the curtain": the light that makes the frame need not be in it | craft: culling the *flame* must never touch the *light* |
| `s2020-advances-pdf-slides` — id Tech 7, *Advances in Real-Time Rendering* 2020 | "Wanted to move to GPU culling to reduce CPU load"; "Budget for 3M visible triangles after CPU culling and LOD selection"; the GPU pass "runs in addition to all standard CPU culling" | production: culling by the CPU first is the standing practice, and the per-object CPU bill is what artists multiply ("Artists wanted to place more lights and decals") |
| `book_real_time_rendering_4th_akenine` — RTR4 §13.6 and §19.4 | "screen-aligned billboards ... The object's size is therefore view-dependent, which can complicate schemes such as frustum culling"; "some game engines do not use hierarchical BVHs, but rather just a linear list of BVs, one for each object" | technical: why a billboard's box is the engine's blind spot, and that a flat list of boxes is the shipping norm (it is Bevy's) |
| `bevy_camera-0.20.0/src/visibility/mod.rs` (`check_visibility`) and `bevy_pbr-0.20.0/src/render/mesh.rs` (`extract_meshes_for_gpu_building`) | an entity with an `Aabb` is tested as a sphere and then as an oriented box against every view's frustum, using its whole `GlobalTransform` (scale included); one that is not visible is not extracted | technical: what an `Aabb` buys, and the one way it goes wrong here |
| `docs/research/flame.md`, `flame-wind.md`, `flame-catch-size.md` | the vertex stage's geometry, the lean's `sin θ ≤ 0.94`, the catch's `2/3` power | the extents below |
| Measured, iteration 277 (`examples/bench_flames.rs`) | the cost per flame, before and after | the validation targets |

## Craft principle

A cinematographer lights from outside the frame and the audience never misses it: "a strong key light from offscreen left throws a shadow on the curtain at the right" (Orisayemi & Afabor, on *Aja Meji*). The same holds in the other direction: a source that is out of shot still lights the shot, and one that is in shot must be *there*, whole, at the frame's very edge. So the two halves of a fire are culled by different rules. The **light** is the pool's, ranked by distance and never by what the camera sees. The **flame** is the visible source, and it must not be culled until the last pixel of it has left the frame, and must be there on the first pixel that enters. A flame that pops at the screen's edge is a source that is missing for one frame while its light is still there, which is the first artifact the loop's rubric names (a light with no source).

## How shipping games do it

Culling by the CPU against a box per object is the first stage of every engine's frame, and the later GPU stages are added to it, not instead of it: id Tech 7's GPU triangle culling "runs in addition to all standard CPU culling", with a triangle budget stated "after CPU culling and LOD selection", and the reason they moved work to the GPU was "to reduce CPU load" because "artists wanted to place more lights and decals". RTR4 says the shipping shape of it: for many objects "just a linear list of BVs, one for each object" is simpler to multi-thread, which is exactly Bevy's `check_visibility` (a parallel pass over a flat list of `Aabb`s). And RTR4 names the one thing a flat list of boxes cannot know: a billboard's "size is ... view-dependent, which can complicate schemes such as frustum culling".

That last sentence is the crate's own history. Iteration 1 drew the flame as a unit quad whose vertex stage turns it to face the camera and stretches it to the flame's height, so the engine's box, computed from the unit square, was the wrong size; the fix was `NoFrustumCulling`, which is correct and costs everything: a flame behind the camera or a kilometre away is extracted, sorted and drawn like the one in front of the player.

## What it costs, measured

`examples/bench_flames.rs`: `N` `FireKind::Torch` sites on a 3 m grid, one camera looking at a corner of it (about twenty flames in view), 1280 x 720 off-screen target, no window, release build, the mean of 300 `App::update()` calls (main world, extraction and render-world CPU together). Before this note's change, with `NoFrustumCulling`:

| flames | 0 | 1 000 | 5 000 | 20 000 |
|---|---|---|---|---|
| ms per frame | 2.1 | 2.7 | 8.6 | 27.1 |

That is 1.3 µs per flame per frame, linear, with no flame in view. Taking the work apart (the flame set run, then not run after the flames were dressed; the flames visible, then `Visibility::Hidden`) puts it in the render world: with the set idle and all 20 000 flames still drawn the frame is 3.6 ms, and with the set running and the flames hidden it is 6.1 ms, against 26.5 ms when both are on. The cost is the `Changed<MeshTag>` re-extraction: `update_flames` writes a flame's brightness byte on every frame it changes, which for a whipping torch is most of them, and each write makes the render world extract that mesh again, for every flame in the world, because `NoFrustumCulling` makes every one of them visible. A town of 5 000 torches paid 6.5 ms a frame (40 % of a 60 Hz frame) for fires nobody could see.

## Model adopted

Use the engine's culling, and give it the box it needs. Three facts make a flame's box not the unit square's:

1. **The vertex stage draws a quad the CPU never sees.** Its height is the flame's own (`FlameSite::height`, packed in the tag), times the catch to the power `2/3`; its width is `0.6 +` the lean; it is shifted by half the lean; and from above it turns to face the camera (`src/flame.wesl`, `vertex`). Every corner of it lies inside a ball about the flame's foot of radius `FLAME_REACH` (1.7) of the drawn height. The worst corner is `sqrt((0.3 + 0.995)^2 + 1^2) = 1.636`, the upright quad's top corner at the shader's own lean clamp; the face-the-camera branch reaches 1.55. A ball, not a box, because the camera decides the quad's orientation and the box must hold it from every side.
2. **The transform's scale is data.** The catch is the scale's `x` and the wind's lean is the scale's `y` and `z` (`1 +` the lean's `x` and `z`), so that one shared material can still draw a different flame per instance. The engine multiplies the box by that scale, so a box built once for a full-size flame shrinks to a sliver as the flame catches or leans (`1 + lean` is as small as 0.06 at the lean limit). The extents are therefore divided by the scale, axis by axis, every time the scale changes: `flame_cull_extents(height, took, lean)`. The box the frustum test builds is then the same cube of side `2 * FLAME_REACH * size` for every catch and lean, and, because it is a cube about the foot, for every rotation of a parent.
3. **The box is the crate's, not the engine's.** The flame carries an `Aabb` and `NoAutoAabb`, so `calculate_bounds` leaves it alone; `update_flames` rewrites `half_extents` on exactly the frames it rewrites the scale (a flame in still air costs nothing, as before); `NoFrustumCulling` is gone.

The flame's *light* is untouched: it is the pool's or the host's, and a culled flame is a flame that is out of shot, not a light that is out. The embers (`FlameEmbers`) keep `NoFrustumCulling`: their quad is a separate, smaller cost (iteration 235 measured 0.1 ms for the courtyard's) and is the next item if it matters at scale.

What was tried and not done: skipping the brightness write of a flame that is not visible. It saves 0.5 ms at 20 000 flames (5.9 against 6.5 ms) and costs one frame of a stale brightness byte when a flame enters the view (`ViewVisibility` is computed in `PostUpdate`, after `Update`, so the first visible frame is drawn with the byte of the last frame it was). A flame at the frame's edge brightening by up to its depth for one frame is invisible, but 0.5 ms at 20 000 flames is not worth any frame that is wrong.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| `FLAME_REACH` | 1.7 flame heights | derived above from `src/flame.wesl`'s `FLAME_ASPECT` 0.6, `SOURCE_HEIGHT` 0.35, `OVERHEAD_HEIGHT` 1.0 and the lean's `0.995` clamp (1.636), rounded up; the test below fails at 1.6 |
| the catch's exponent | `2/3` | `docs/research/flame-catch-size.md`, mirrored from the shader's `CATCH_SIZE_EXPONENT` |
| the scale floor | `1e-4` | the shader's own floor for the catch (`max(took, 0.0001)`); a lean of `-1` would otherwise divide by zero |
| the box's centre | the flame's foot | a ball about the foot holds the quad whichever way a parent is turned |

## Validation targets

- **No culled flame is on screen.** For 40 000 random flames (height 0.04-3 m, catch 0.001-1, lean to the shader's clamp, parents turned any way) and random cameras (any elevation, aimed so the flame lands from the middle of the frame to well outside it), a flame with any of 25 sampled points of its drawn quad inside the frame is never culled by Bevy's own sphere and oriented-box test. Fails with `FLAME_REACH` below 1.64 and with the scale left undivided.
- **The box keeps its world size through a catch and a lean**: on every frame of a kindle in a 3 m/s wind, the box the frustum test builds is the cube round the drawn flame (y and z exactly; x within the flare's overshoot, never short).
- **Off-screen flames are cheap**: with about twenty flames in view, `bench_flames` at 5 000 flames costs at most a third of what it did and at 20 000 at most a quarter, and its `visible` count is the flames in view and not the flames in the world.
- **Nothing visible changes**: the courtyard's captures at the default, ground, graze, top-down, closefire and two seeded cameras, and two sweeps of the camera's aim that carry every fire across both frame edges (one in a 3 m/s gusting wind with every fire put out and re-lit), differ from the old build by GPU noise only.
- **The bench's other lines** (`bench_lights`) do not move.

### Results (iteration 277)

`bench_flames`, mean ms a frame, best of three alternating runs on a machine at load average 18-50 (so every figure is inflated and the ratios are the finding), base `e693df8` against this change:

| flames | 0 | 1 000 | 5 000 | 20 000 | 5 000, 3 m/s | 20 000, 3 m/s |
|---|---|---|---|---|---|---|
| before | 1.3 | 2.2 | 7.8 | 30.0 | 8.0 | 28.1 |
| after | 1.7 | 1.9 | 2.2 | 6.4 | 3.1 | 5.9 |
| flames the renderer counted in view, after | 0 | 15 | 15 | 15 | 15 | 15 |

The 0-flame cell is noise (six more runs with the order reversed read 2.4-5.2 for the new build and 2.5-7.4 for the old). A quiet run of the first build against the probe it came from read 2.1, 2.7, 8.6, 27.1 before and 2.3, 2.9, 6.5 after at 0, 5 000 and 20 000. All three targets are met: 3.5x and 4.7x cheaper at 5 000 and 20 000, `visible=15` of 20 000. Pixels: eight static cameras and the two sweeps (94 frames each, 1280 x 720, every third frame from 20) differ by at most 1 DN, with no pixel over 8 DN; the `flicker frame` and `pool total` log lines of five scenes are byte-identical. `bench_lights` (six alternating runs, 5 000 candidates, 2 000 lights, pool 16): `min_us` 315-396 before and 320-405 after, `p50_us` 591-703 and 576-750: the bench has no flame in it and does not move.

What is left at 20 000 flames is about 0.25 µs a flame, all of it in the main world and the visibility pass: `update_flames`' per-flame work (a parent lookup, the flicker's sines, the tilt law when there is wind) and Bevy's sphere and box test over the boxes. Skipping a hidden flame's brightness saved only 0.5 ms of the 4.3 ms and was dropped (above); a memo of the tilt law by diameter changed nothing measurable under a 3 m/s wind (three alternating runs at 5 000 flames: 2.6-3.4 ms without it, 3.0-3.4 with it) and was dropped too.

## What great looks like

Nothing: a culling change is great when it cannot be seen. A flame slides off the edge of the frame and back on without a pop, a flicker, or a frame of the wrong size, at any camera angle, in wind, and while it catches; the same scene with a thousand more fires in the world costs the frame what the fires in view cost it.

## Deviations

`NoFrustumCulling` on the flame (iteration 1, `docs/research/flame.md`, and `AGENTS.md`'s trap about it) is replaced, not removed: the trap was real (the engine's box is wrong for a billboard) and the replacement is the crate's own box. `AGENTS.md` and `flame.md` say so. No lighting default changes.

## Pending index

None. The three sources above are in the corpus and were read from its markdown; `distill_search` was not needed.
