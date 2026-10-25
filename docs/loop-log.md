# bevy_lights loop log

Iterations of the improvement loop (`skill://bevy-lights-build-loop`). Each entry: what changed and
why it ranked first, the sources, the critique and verdict, the numbers, and the ranked backlog that
the next iteration picks from.

## Iteration 0 — 2026-10-25 — extracted from Sanctuary

The crate does not exist yet before this entry: this is the extraction that started it, not a loop
iteration, and it is written here so the loop has a baseline.

### What this is

Sanctuary's fire lights, lifted out and generalised to carry no game knowledge:

- `Flicker` / `Swell` (now `Kindle`) / `flicker_fires` / `swell_gain` from
  `src/prototype/town.rs` → `bevy_lights::FlickerPlugin`.
- `POOL` / `LIGHT_REACH` / `PoolLight` / `spawn_pool` / `assign_pool` from
  `src/prototype/torches.rs` → `bevy_lights::LightPoolPlugin::<F>`, generic over the host's focus
  marker.
- `FlameMaterial` / `FlameSite` / `flame_tag` / `dress_fires` / `animate_flames` and
  `assets/shaders/flame.wesl` from `src/prototype/flame.rs` → `bevy_lights::FlamePlugin` and
  `src/flame.wesl`.
- `RANGE_MARGIN` / `raised_light` → `bevy_lights::model`, alongside the pure maths of the other two.

Three deliberate changes on the way out, each recorded in its note's `Deviations`:

1. **Spot lights flicker.** Sanctuary's `flicker_fires` queried only `&mut PointLight`, so the wall
   sconces that carry `Flicker` (all `SpotLight`s) never flickered, though `WALL_LIGHT_LUMENS`'
   budget divides by the mean flicker 0.85. The spot path is the fix the budget already assumed.
2. **The pool is generic over the focus marker** (`LightPoolPlugin::<F>`), and a candidate carries its
   own world-space offset instead of the crate knowing a torch's lift.
3. **The flame is drawn only where a host spawns a `FlameSite`**, so which fires have a visible flame
   stays the game's decision — a torch whose light is at +1.5 m and whose flame is on its head needs
   exactly that split.

### Findings from the extraction itself

Two real bugs, both caught by tests before the first capture:

- **`flicker_lights` froze in any app with no camera.** `PointLight` requires `Visibility`, which
  requires `ViewVisibility`; a windowless app never runs a visibility pass, so every light's
  `ViewVisibility` stayed false and the "an unseen light is not rewritten" rule skipped every write.
  A headless bench, a test and a server-side simulation all hit this. The rule now applies only when
  there is a camera to cull from, and a light the host put out with `Visibility::Hidden` is skipped
  explicitly.
- **`assign_pool`'s candidate query demanded `&Visibility`,** so a host pooling a light on an entity
  with no mesh (which has no `Visibility` unless it asks for one) silently got an empty pool. It is
  `Option<&Visibility>` now, and "not hidden" is the test.

One more, in the tooling: **`tools/wesl-check` inherited a blind spot from `bevy_elements`'.**
Every module whose shaders import `bevy_pbr` reported "not checked (module not found)", because a
Bevy import inside a module of this crate's package arrives as the sub-package path
`Package("bevy_lights/bevy_pbr")` — a spelling WESL's `join_path` produces and no source file can
write — and the router dispatches on origin equality. Each Bevy namespace is now mounted under its
own name *and* under `bevy_lights/<name>`, with the stubs keyed by `Absolute` paths and carrying the
real interface signatures, so a call with the wrong arity or a field that does not exist fails here
instead of in the GPU driver. **`bevy_elements` has the same hole and should be told** (an issue on
`Ladvien/bevy_elements`, per `AGENTS.md`).

### The bench baseline

`cargo run --release --example bench_lights` on mac_air (M4 Pro), default sizes (5 000 candidates,
2 000 flickering lights, pool 16, 600 frames):

```
bench_lights candidates=5000 lights=2000 pool=16 frames=600 mean_us=629 p50_us=403 min_us=249 max_us=13050
```

629 µs a frame for 5 000 candidates sorted and 2 000 lights modulated is 3.8 % of a 60 Hz frame's
16.7 ms, on one core, with no GPU work in it at all (the bench has no window and no render app). The
`max_us` of 13 050 is the first frame after the pool spawned its lights and the allocator warmed, not
a steady-state number; `p50` is the number to compare between iterations.

### Top 5 remaining, ranked

1. **A researched fire-flicker model with a size parameter.** Every fire today flickers at the same
   two rates (7.3 and 13.1 rad/s) whatever its size, and the literature says a larger fire puffs more
   slowly. `docs/research/corpus.md` records that the fire-physics papers (Cetegen & Ahmed 1993;
   Kitahata et al. 2005) are **not** in the corpus; importing them is part of the item.
2. **Pool hysteresis.** A light handed from one candidate to another pops. Fade it over a few frames,
   or hold a candidate's slot until another is meaningfully nearer. The craft checklist's
   "pool hand-off popping" line is the measurement.
3. **An `elements` feature** that authors practical lights against `bevy_elements`' exposure
   (`light_intensity`, `ElementsExposure`), so a host in SI metres does not hand-tune lumens.
4. **An `elements` feature** that leans the flames with `bevy_elements`' `WindState`: a flame that
   ignores the wind beside foliage that does not is the giveaway in a storm.
5. **A colour-temperature helper** (Kelvin → `LinearRgba`) for warm practicals against cool moonlight,
   which is the one number every night scene needs and every host re-derives.

### Not yet measured

- No capture has been judged. The showcase's lighting numbers (`Exposure::ev100` 3.0, 60 000 lm
  braziers, 8 000 lm torches, 300 lx moon) are design defaults, and iteration 1's first job is the
  capture and the craft checklist.
- `docs/media/courtyard.gif` and `courtyard.png` were made on mac_air; a capture from big's Xvfb is
  the loop's baseline.
