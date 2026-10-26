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

### The first capture, judged (V3)

`LIGHTS_FRAMES=600 LIGHTS_SHOT=… ./target/release/examples/courtyard` under Xvfb on big (RTX 3090),
1280 × 720, the default camera `(0, 9, 16)` looking at the origin. `kindled` printed **4** times, one
per brazier, with no `B0004` warning. `Exposure::ev100` stays at the plan's 3.0: its V3 rule moves it
only if the pool reads above 250 or below 60.

| region (40 × 40 px, grey DN) | DN |
|---|---|
| ground beside the near-left brazier | 198.5 |
| ground beside the near-right brazier | 190.5 |
| the shadow cross between the two near pillars | 18.7 |
| ground in a pillar's shadow, right | 31.4 |

The pool mean (194.5) is inside 120–200 and **10.4 ×** the dark patch (target: at least 4 ×). The
mean of the whole frame breathes between 75.1 and 75.6 DN over twelve consecutive GIF frames, which is
four ±15 % fires and nothing else moving the exposure.

The pool is real: the difference of the 8-light and the 2-light captures lights only the left of the
frame, where the orbiting focus is at frame 600, and is exactly the six extra torches' glow
(a grey-level difference of the two captures, amplified 4 ×: nothing else in the frame differs).

**Critique, as a hostile art director** (every item is a candidate for iteration 1):

1. *The pool is nearly invisible in the beauty shot* (major). At the plan's 8 000 lm a torch is a
   fifth of a 60 000 lm brazier, and within 12 m of one the braziers' warm wash buries it: the eye
   cannot tell 2 pooled torches from 8. The difference image proves they are there; the hero GIF does
   not show it. Either the torches are raised, the courtyard is darkened away from the braziers, or the
   camera is moved in on the focus. The plan's numbers are design defaults and were left alone here.
2. *The ground's falloff is a single broad warm plateau* (noticeable). There is no visible
   inverse-square shoulder between "lit" and "dark" within the courtyard: the four 12 m ranges overlap
   everywhere inside the pillars. The key-to-fill ratio reads well at the edges and flat in the middle.
3. *No sky and no horizon* (noticeable). The 40 × 40 m ground ends in a hard edge against a near-black
   clear colour. The default grey it replaced read as an unfinished viewport.
4. *The braziers are boxes* (cosmetic). A 0.8 m cube of dark stone with a coal slab on top is a
   motivated source, which is the point, but it reads as a cube.
5. *Hard pillar shadows* (cosmetic): `shadow_maps_enabled` on the braziers gives crisp edges where a
   real fire's large source would soften them; `PointLight::radius` is the knob and is 0.

Defects found and fixed during the capture rather than logged:

- **`kindled` printed twice per brazier.** The first frame of a fixed-step run has a delta of zero, so
  `Kindle::elapsed` was still 0 on the second frame and the same fire was announced again. The
  advance is now floored at `f32::MIN_POSITIVE`, so a heard kindle never reads exactly 0 again; the
  regression test (`a_light_present_on_the_very_first_frame_is_heard_once`) fails without it
  (`[15v0, 15v0]`). `Added<Kindle>` would have been wrong in the other direction: replacing a
  component leaves its `added` tick alone, so a re-lit fire would never be heard again.
- **A `Flame` under a parent with no `Visibility` warns (B0004)** and does not inherit its torch's
  visibility. `FlameSite` now `#[require(Visibility)]`.
- **A dark square under every brazier.** The light sat a hand's width over a 0.7 m box, so the box cast
  a hard 2 m shadow across the ground around its own fire. The light is now at the flame's centre and
  the bowl does not cast.
- **The moon was 1000 × too bright** (300 lx, a transcription slip for the plan's 0.3 lx): the whole
  courtyard read as an overcast afternoon, the brazier pool was 1.6 × the shadow, and V3's 4 × was
  unreachable.

Tooling note: **`/tmp` on big is a 16 GB tmpfs that other agents had filled to 100 %.** `LIGHTS_SHOT_DIR`
into it wrote 241 frames of which most were truncated PNGs, and `tools/gif.sh` (which `mktemp`s there)
failed with ENOSPC; the GIF came out 308 KB and mostly undecodable ("chunk too big"). Send frame
directories and `TMPDIR` to the NVMe (`/home/ladvien/tmp-lights`) and check `df -h /tmp` first.
`docs/DEMOS.md` says so.

### Not yet measured

- Hand-off popping: no consecutive-frame difference at a pool boundary has been looked at yet.
- The flicker's rate against any source: the fire-physics papers are not in the corpus.
- GPU cost of `FlameMaterial` (the bench is CPU only).
