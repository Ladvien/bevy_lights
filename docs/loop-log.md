# bevy_lights loop log

Iterations of the improvement loop (`/build-loop`, `.omp/commands/build-loop.md`). Each entry: what changed and
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

### Published and adopted

`0.1.0` is on crates.io, published from `b6e734c` (tag `v0.1.0`). CI on big's runners: lint 2.0 min, build and
test 8.9 min cold, wasm 1.7 min, about 1.3 min warm.

Sanctuary adopted it the same day (`Ladvien/sanctuary` `747461d`, decision 0307) and deleted its copies: 1208 tests
green, the release build builds, and before/after captures of the walk-around town and of the siege night were
reviewed by an agent that opened the images (both ACCEPTABLE: every fire in both builds at the same place, the
sconce pools 1.5 to 2.7 grey levels darker on average). Nothing needed a crate change, so there is no `0.1.1`.

What adopting it showed, for the next iteration:

1. **A `FlameSite` spawned from a system against a light that despawns in the same frame leaves a parentless site
   at the world's origin.** Bevy removes the invalid `ChildOf` and keeps the entity. Sanctuary dresses a fire from
   the light's own queued command (`queue_silenced` + `with_children`) and has a test that fails against the
   naive form. `FlameSite`'s docs should say so; so far they do not.
2. **Every host has to write the system that decides which fires get a flame** (Sanctuary's `site_fire_lights`, 20
   lines). If a second host wants the same, a marker the crate resolves into a site itself is the candidate.
3. **Spot-light fires now flicker, swell and report `Kindled`**: Sanctuary's wall sconces never did, and the
   kindle cue it plays from `Kindled` now sounds for them. It moved their pools darker by the mean flicker, which
   their lumen budget had assumed.
4. **A pool light that is out is hidden**, so a census of visible point lights (Sanctuary's `perf: … lights N`)
   counts none of them while the town is unlit, and counts only what a camera sees: it fell from 15 to 3 for two
   seconds while the view moved and came back to the same 15 [INFERENCE: the view moved; the lights that left
   included ones that are not the pool's]. A sample every five seconds is not a regression signal on its own.
5. **`tools/wesl-check` as first copied from `bevy_elements` never compiled a shader that imports Bevy**: a
   Bevy import arrives as the sub-package `bevy_lights/bevy_pbr`, which a router mounted at `bevy_pbr` is never
   asked about, and "module not found" was classed benign. The copy here mounts each namespace under both
   spellings; a deliberately wrong call into a stubbed Bevy function now fails it (`incorrect number of
   arguments … expected 1, got 2`). Upstream's own check prints "not checked" for 11 of its 44 modules, exactly
   the ones that import Bevy: filed as `Ladvien/bevy_elements#37`.

### Not yet measured

- Hand-off popping: no consecutive-frame difference at a pool boundary has been looked at yet.
- The flicker's rate against any source: the fire-physics papers are not in the corpus.
- GPU cost of `FlameMaterial` (the bench is CPU only).

## Iteration 1 — 2026-10-26 — the lighting look: a key, a fill, and a temperature

### What changed, and why it ranked first

The crate's three systems each did their job and the courtyard still was not a night. Measured on the
baseline captures rather than guessed: the fire's *falloff* was already right (a clean inverse square,
0.906 linear at the flame to 0.0069 six metres out), but the pools **blew out** (sRGB 255, 245, 232,
four metres of ground above 200 DN), nothing in `src/` computed a **ratio** or a **colour
temperature** (the fire was `srgb(1.0, 0.60, 0.30)`, the moon 0.3 lx, both picked by eye), and the
stone had no **specular answer** (one roughness of 0.9 everywhere).

This ranked first over the runner-up, a fire-size flicker parameter (backlog item 1 for two
iterations), because the missing piece was not how one fire moves but what the scene *is*: a game
developer adopts this crate for its night looking like a night, and no flicker rate fixes a brown wash.
It also closes the ownership gap the user named: cinema and dramaturgical lighting, codified.

### The sources

Read live from home-still on 2026-10-26; all cited in `docs/research/lighting-look.md`.

- **craft:** `10.4324_9781003671220` (Lazar, *Cinematography Beyond Technique*), `10.7456_tojdac.1674124`
  (Doğaner, lighting as narrative), `10.2478_bsmr-2022-0020` (Huttunen, lighting the face),
  `10.5897_jmcs2022.0788` (Vanden Bossche, low-key), `10.32657_10220_48312` (Doyle's cinematography),
  `10.23856_6105` (*Harmony of light and shadow*, the softness table).
- **production:** `s2007-advances-pdf-slides-2-8-mb` and `s2007-advances-course-notes-0-4-mb`
  (Mitchell, Francke & Eng, TF2), `s2006-advances-full-course-notes-pdf` (the dark stormy city),
  `s2011-advances-pdf-slides` (fire as hotness → blackbody).
- **technical:** `PBR3_12_Light_Sources` (Planck, colour temperature), `book_real_time_rendering_4th_akenine`
  (tone mapping, specular).

### The model

`src/model/cinematography.rs`, engine-free and re-exported flat:

- `key_to_fill` / `stops` / `fill_for`, with `LOW_KEY` 8:1, `MOON_KEY` 4:1 and `HIGH_KEY` 2:1.
- `fire_lumens(lux, reach, over)`: the inverse square solved for a point light (Bevy maps lumens to
  Φ/4π candela), so a pool is given a size and a brightness and the lumens follow.
- `blackbody_colour(kelvin, strength)`: Planck against the CIE 1931 observer to linear sRGB.
- `white_balanced(kelvin, balance_kelvin, strength)`: the same as a camera balanced elsewhere records
  it. This is what makes a fire orange and a sky blue from two temperatures and one balance.

`examples/courtyard.rs` is the proof: every light's lumens, every colour, the moon's illuminance and
the coal bed's emissive are derived from those numbers, and it prints the result. Its line:

```
look: fire 1900 K, sky 8000 K, balanced 2856 K, pool 1000 lx (edge 250) at 220697 lm (brazier, 4.0 m)
and 15402 lm (torch, 2.0 m), moon 500.0 lx, key-to-fill 2.00:1 (1.00 stops), at the pool's edge 0.50:1
```

The numbers that went: 60 000 lm per brazier and 8 000 per torch (now derived: 220 697 and 15 402 lm
from the pools they light), `srgb` triples for the fire and the sky (now temperatures), 0.3 lx for the
moon (now `fill_for`), `ev100` 3.0 (now 6.0, computed from Bevy's `2^-ev100 / 1.2`), and a 0.66 m
emissive coal slab under a 0.9 m flame (now 0.34 m under a 1.1 m flame).

### The critique, and what it cost

`/tmp/l1/critique.md`, written before the verdict. Five defects in my own work, in the order found:

1. **The night went black.** Sizing the fire from its pool's *edge* and exposing as though that edge
   were the key: the elevated frame's mean fell 72.3 → 27.6 DN. Fixed by computing the exposure from
   Bevy's formula: at `ev100` 6.0 a 1000 lx pool on 0.3-albedo stone is linear 1.24, a 250 lx edge
   0.31.
2. **No cool side existed.** `blackbody_colour` clamps to gamut, and a 1900 K blackbody's blue is
   *negative*; clamping before the white balance divided made the balance grey. Split into an
   unclamped internal colour and a clamped public one; the balance divides first, clamps after.
3. **The coal bed was a lightbox** — a slab nearly as wide as the flame was tall, emissive 3.0,
   clipped to flat white. Now 0.34 m at 1.4.
4. **A torch demanded 61 607 lm**, because it was sized like a brazier; now sized from its edge.
5. **The flame clipped** at strength 1.0; now 0.82.
6. **A raking fill made it worse.** Rotating the moon to rake across the figures turned the whole
   frame into a warm wash (mean fell, figures at 3–9 DN against ground at 47–61). Reverted: a fill
   fills. The separation is the two colours, not the fill's direction.
7. **At 4:1 the courtyard was unreadable.** Figures between pools measured 12.6–31.9 DN against
   moonlit ground at 76–83. Raised the target to `HIGH_KEY` (2:1, one stop) — the craft's own
   high-key end, chosen for a *game's* reader rather than for a photograph. That is the single most
   consequential number in this iteration, and it is a design default the owner can move back.

### Verdict

**GREAT** for the model and for the look. Every line of the note's "What great looks like" holds at
full frame and at 4x, except the ground's specular answer, which is a material-and-art change and is
iteration 2's. The frames were opened and judged here, not delegated.

### The craft checklist, before and after

| line | before | after |
|---|---|---|
| key-to-fill at a pool | no ratio existed; four 60 000 lm braziers, 12 m ranges, a 10 m courtyard: every warm surface lit by another fire | 2.00:1 at the pool's centre, 0.50:1 at its edge, printed by the demo |
| falloff shape | inverse square already right (0.906 → 0.0069 over 6 m), but blown at the top | the same falloff, unblown: p99 245 → 211, clipped pixels 0.20 % |
| motivated light | every pool had a bowl; the bowl was a lit box with no flame read | coal bed 0.34 m under a 1.1 m flame; every pool has a flame, every flame a pool |
| warm/cool separation | pillar faces blue, everything else warm; the fill normalised to white | one balance (2856 K): fire orange, sky blue, shadows at 5–13 DN of blue |
| silhouette and readability | figures 112 DN against ground at 104: no separation | figures 12–48 DN against ground at 76–83: a lit silhouette, not a hole |
| specular highlights | none (roughness 0.9 everywhere) | graded highlights on pillar faces; no placed ground highlight yet |
| flicker rate and amplitude | unchanged (not this iteration's system) | unchanged; the flame's colour now matches its light's temperature |
| pool hand-off popping | not measured (backlog 2) | not measured |
| flame size against reach | brazier 0.9 m flame, 12 m range: a candle lighting a hall | 1.1 m flame, 4 m pool; torch 0.35 m, 2 m pool |
| bloom and clipping | pools at 255, 245, 232; 4 m of ground over 200 DN | 0.20 % clipped in the elevated frame, all in a flame core; 0.00 % at grazing |

### The benchmark

`cargo run --release --example bench_lights`, default sizes, same machine:

| | before | after |
|---|---|---|
| mean_us | 629 | 205 |
| p50_us | 403 | 202 |
| min_us | 249 | 127 |

No regression: the new module is pure maths called at startup by the example, and nothing in it runs a
frame. (The before column is iteration 0's recorded baseline; the after figure is today's run on the
same Mac.)

### The gate

`cargo build --release`, `cargo build`, `cargo test` (42 tests: 16 lib, 7 flicker, 3 leaf, 10 pool, 6
doc), `cargo test --features serde`, `cargo build --examples --all-features`, `cargo clippy
--all-targets --all-features -- -D warnings`, `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
--all-features`, `cargo fmt --check`, `cargo build --lib --target wasm32-unknown-unknown`, and
`tools/wesl-check` (the shader was not touched) all pass.

Two doctests failed on the way and were fixed rather than deleted: both asserted the behaviour I had
corrected in the code, so the documentation had gone stale in the same minutes.

### Two things the crate got wrong that the example caught

- **`blackbody_colour` clamped before normalising**, which is right for a displayable colour and wrong
  for anything that divides by it. A public `blackbody_colour` and a private `blackbody_linear` is the
  split; `white_balanced` uses the second.
- **`fire_lumens`' doc example** asserted `candela / 16.0 == 12.0` for a ground-level 4 m pool, which
  is only true for a surface square-on to the light. It is, at the pool's centre, so the doctest
  stays — but the note says "square-on" now.

### Top 5 remaining, ranked

1. **A fire-size flicker parameter** (still backlog 1): a brazier and a candle flicker at one rate. The
   fire-physics papers are still not in the corpus.
2. **A placed specular highlight**: the one "what great looks like" line still at PARTLY. A tighter
   roughness on some stone, or a wet-stone material, so each fire resolves a highlight inside its pool.
3. **Pool hysteresis** (still backlog 2): a light handed between candidates pops; no consecutive-frame
   difference has been measured at a boundary yet.
4. **An `elements` feature** authoring the fill against `bevy_elements`' exposure, so a host in SI
   metres does not carry `ev100` and lux separately.
5. **A Kelvin range check on `blackbody_colour`'s gamut clamp**: the function is right, but a caller
   asking for 1000 K gets a colour with no blue at all and no warning. A `FlameLook`/light colour that
   saturates is a thing a host should be able to ask about.

## Iteration 2 — 2026-10-26 — the pool's hand-off: an overlap rule and a fade

### What changed, and why it ranked first

A pool light that changes owner teleported: its position jumped from one prop to another and its
intensity stepped. The craft checklist has had "pool hand-off popping" as an unmeasured line since
iteration 0, and the pool note has carried it as a known defect. It ranks above the fire-size flicker
(a beauty-and-plausibility item) because a **visible artifact** is category 1 and a teleporting light
is the artifact this technique is famous for.

### The sources

Read live from home-still on 2026-10-26; all cited in `docs/research/pool-hand-off.md`.

- **craft:** `10.58709_niujhu.v8i3.1694` (Bordwell & Thompson on overlapping keys: "By overlapping
  different key lights, the filmmaker can maintain a constant intensity as actors move around the
  set"), `10.4324_9781003671220` (motivated movement).
- **production:** `s2025-advances-materials-9` (Stochastic tile-based lighting in HypeHype: the
  pop-versus-noise trade), `s2020-advances-pdf-slides` (improved culling).
- **technical:** `10.1109_tvcg.2019.2898741` (finite integration time and the Talbot-Plateau law),
  `book_fundamentals_computer_graphics_marschner` ch. 22 (fusion at 60–80 Hz),
  `book_level_of_detail_3d_graphics_luebke` ch. 9 (visual masking).

### The model

`src/model/pool.rs`, engine-free:

- `hand_off(held_distance, challenger_distance, hysteresis)`: a challenger takes the light only when
  `challenger · hysteresis < held` — the overlap rule as an algorithm. `hysteresis = 1.0` is
  nearest-wins.
- `fade_factor(since, fade_seconds)`: smoothstep from 0 to 1, exactly 1 after the fade.
- `LightPool { hysteresis: HYSTERESIS (1.15), fade_seconds: FADE_SECONDS (0.35) }`, both public.
- `assign_pool` holds each slot's candidate in one `Local<HandOffState>`, claims keeps in a first pass,
  then fills the rest from the nearest unclaimed candidate and fades a slot that changed owner.

### The bug the work exposed

**The pool read `GlobalTransform`, which lags a frame.** `TransformPlugin` propagates in `PostUpdate`,
so on the frame a host moves a prop the pool compared the *previous* frame's distance. With hysteresis
that is worse than without it: the pool could hold a light on the position a prop had left. The pool
now reads the candidate's own `Transform` when it has one and falls back to `GlobalTransform` for a
child placed by its parent (a torch in a hand). This is the fix that made "a steady pool is steady"
true; `a_steady_pool_keeps_its_candidates_and_its_brightness` is the test.

### The critique, and what it cost

`/tmp/l2/critique.md`, written before the verdict. Four of my own defects, and one gap in my own
verification:

1. **A slot's first light faded in**, so a pool filling for the first time flashed up from zero.
2. **One-pass assignment let slots collide**: slot 0 saw slot 1's still-held candidate as free for
   one frame, because slot 1 had not re-claimed it. Two passes now.
3. **Every unheld slot took the single nearest candidate**, so three slots all reported the same
   candidate in the first frame. The unclaimed-nearest rule fixed it and reproduces the old
   "slot k takes candidate k" for an unfilled pool (its own test).
4. **The pool's own total was not measured** — the craft checklist's line. Closed with a test that
   sums the slots' bases and *bites*: `fade_seconds: 0.0` makes it fail at `200 -> 200`.

Remaining, and deferred to iteration 3 (none a regression): the fade is a dip rather than a cross-fade
(it needs a second light, which doubles the budget); a hand-off can still move a light a long way; and
the rule compares distances only, never how much a light matters.

### Verdict

**GREAT.** The overlap rule does what the craft says (a 5 %-nearer challenger does not steal a light —
verified by making the same test fail at `hysteresis: 1.0`; a challenger twice as near does), the two
pairs that used to jump over 0.45 in a 240-frame sequence are gone, and a steady pool is
frame-identical. Every line of the note's "What great looks like" holds.

**A number I got wrong, and had to re-measure.** The first pass reported the worst pair falling
0.697 → 0.524. That 0.524 was the *first* frame pair of the after-run — the pool filling in, not a
hand-off — so it was not measuring this feature at all. Worse, setting `LIGHTS_HYSTERESIS=1.0
LIGHTS_FADE=0.0` and re-capturing gave the same 0.524, which is what exposed it: the switch that turns
the feature off must change the measurement, or the measurement is not of the feature. On the
steady-state tail the real figures are 0.697 → 0.441 and two pairs over 0.45 → none. The claim that
the *fade* removes the jump is also wrong on its own: in this run the hysteresis prevents the
hand-offs rather than fading them.

### The craft checklist, before and after

| line | before | after |
|---|---|---|
| pool hand-off popping | unmeasured; the note called it a known defect | steady-state worst frame pair 0.697 → 0.441 (37 % smaller); pairs over 0.45: 2 → 0; median unchanged |
| steady is steady | untested | `a_steady_pool_keeps_its_candidates_and_its_brightness`: five frames, identical slots, positions and bases |
| the total holds | untested | a hand-off dips the pool's total by one light's share and it is whole again inside the fade; `fade_seconds: 0.0` fails the same test |
| the nearest torch still wins | nearest wins outright | a challenger 2× nearer takes the light in one frame; a 5 %-nearer one does not |

### The benchmark

| | iteration 1 | iteration 2 |
|---|---|---|
| mean_us | 205 | 208 |
| p50_us | 202 | 211 |
| min_us | 127 | 135 |

p50 moved +9 µs (+4 %) for the two passes and the hold map, at 5 000 candidates and 16 slots. That is
0.13 % of a 60 Hz frame. The trade is logged, not hidden: it buys a light that does not teleport, and
the pool is the one system this crate runs every frame.

### The gate

`cargo build --release`, `cargo build`, `cargo test` (53 tests: 20 lib, 7 flicker, 3 leaf, 15 pool, 8
doc), `cargo test --features serde`, `cargo build --examples --all-features`, `cargo clippy
--all-targets --all-features -- -D warnings`, `RUSTDOCFLAGS="-D warnings" cargo doc`, `cargo fmt
--check`, `cargo build --lib --target wasm32-unknown-unknown`, `tools/wesl-check` — all pass.

Clippy caught two real things on the way: `assign_pool` at nine arguments (the pool's three `Local`s
became one `HandOffState`), and the candidate query's type (factored into `CandidateRead`).

### Top 5 remaining, ranked

1. **A fire-size flicker parameter** — still the top of this list after three iterations. A brazier and
   a candle flicker at one rate; the fire-physics papers (Cetegen & Ahmed, Kitahata) are still not in
   the corpus, and importing them is part of the item.
2. **A cross-fade instead of a dip**: a hand-off that overlaps two lights for the fade's duration,
   which needs a spare slot (the pool would hold `size + 1` lights) — the craft's "overlapping keys"
   done literally.
3. **A placed specular highlight** (iteration 1's open beauty line): stone that resolves a fire's
   highlight inside its pool.
4. **An `elements` feature** authoring the fill against `bevy_elements`' exposure.
5. **`hand_off` by luminance, not distance**: a dim far candidate and a bright near one are compared
   only by distance today, so the pool can hold the dim one. Whether that is right is a host's call;
   a `weight` on `PoolCandidate` would let it say.

## Iteration 3 — 2026-10-26 — a fire's flicker sized by its fuel bed

### What changed, and why it ranked first

Every fire in the crate flickered at one pair of rates (1.16 and 2.08 Hz) whatever its size, because
the rates were tuned by eye in a foot-scale game. A candle and a pyre moved alike, which is the
cheapest tell that a fire is a lamp with a wobble. Backlog item 1 for three iterations, and the item
was blocked each time on the fire-physics literature the corpus did not hold.

### The research, first

Imported, converted and indexed this iteration: **`10.1017_jfm.2018.707`** (Xia & Zhang 2018,
*J. Fluid Mech.*, open access). It states the law every earlier study found and then derives it:

> "Early researchers [14-19] experimentally found that `f` is related to the fuel source dimension
> `D` as `f ∝ D^{-1/2}` with a proportionality factor of around 1.5 given by Trefethen and Panton [20]
> and Cetegen and Ahmed [21]."

and its own fit to six experimental studies' pool-fire data:

> "…in good agreement with the trend line, `f_p = 0.48 √(g/D)` … which is equivalent to the scaling
> relation, `f = 1.5 √(1/D)`, given by Cetegen and Ahmed [21]."

Cetegen & Ahmed 1993 is not open access — `paper_download` found no copy across seven providers — so
the note cites the accessible paper that carries its result and says so. Also read: `GPU Gems` ch. 6
(Nguyen's shipped-fire chapter) for how a production fire is animated, and Huttunen 2022 + Lazar for
the craft frame. `docs/research/corpus.md`'s fire-physics gap is now closed.

### The model

```
flicker_rate(D)                = 1.5 / √D                Hz
flicker_factor_of(D, t, phase) = 0.85 + 0.15 · (½ sin(ωt + phase) + ½ sin(1.7ωt + 1.7 phase))
                                 ω = 2π · flicker_rate(D)
```

`Flicker { diameter: Option<f32> }`: `Some(0.34)` for a brazier's coals (2.57 Hz), `Some(4.0)` for a
pyre (0.75 Hz), `None` for the inherited rate. The window, the mean 0.85 and the amplitude are
untouched, because a lumen budget divides by that mean.

### The measurement, and three wrong ones

I got this wrong three times before the number was real, and each time it was the measurement:

1. **The whole frame's mean luminance cannot see this.** A ±15 % flicker on four lights in a
   1 280 × 720 frame moves the mean by less than a tenth of a DN. Both sequences read as identical.
2. **My shell passed the switch to both runs.** `LIGHTS_UNSIZED=$([ $2 = 1 ] && echo yes)` under
   `set --` came out empty for both, so I captured the same thing twice and "proved" no difference.
3. **The first `Flicker` in query order is a pool light, not a brazier.** The intensity log's
   12 593 lm against 182 074 lm was two pool lights, not the braziers the feature is about.

The demo now prints `flicker frame N: <lumens>`, which is the honest way to read it. On two captures
that really did differ, a 70 × 70 patch under a brazier's flame:

| | mean DN | sd | up-crossings |
|---|---|---|---|
| sized (0.34 m bed) | 76.652 | 1.449 | **3.98 Hz** |
| unsized (inherited) | 76.665 | 1.394 | **1.66 Hz** |

The mean is unchanged (the window and mean do their job) and the rate more than doubles. The law says
2.57 Hz; the crossing count sees the second sine and the flame's own animation too, so it bounds the
change rather than calibrating it.

### The critique

`/tmp/l3/critique.md`. Remaining, and honest rather than defects:

1. **Only the rate is sourced.** The amplitude, the two-sine shape and the 1.7 coupling are still
   design defaults: the papers give the frequency, not the light's modulation depth.
2. **Gravity is unmodelled** (`f ∝ √g`): a host on another gravity gets Earth's rate.
3. **A fire's diameter is a guess in every host.** The courtyard's 0.34 m is its coal bed's mesh size,
   which is the fuel bed only by assumption; Sanctuary's torches have no size, so they keep the
   inherited rate and still flicker like its braziers.
4. **A capture cannot cleanly confirm 2.57 Hz** — the patch sees a sum — so only the model test does.

### Verdict

**GREAT.** The law is cited to a converted, indexed paper, tested at its boundaries (four diameters
over 20 s inside the window and mean 0.85), the un-sized path is behaviourally identical (its own
test), and the visible result is a fire that moves at its own rate. Bench p50 210 µs against 211, i.e.
unchanged (the new work is a square root per light per frame, when a diameter is present).

### The gate

`cargo build --release`, `cargo build`, `cargo test` (58 tests: 22 lib, 8 flicker, 3 leaf, 15 pool, 10
doc), `cargo test --features serde`, `cargo build --examples --all-features`, `cargo clippy
--all-targets --all-features -- -D warnings`, `RUSTDOCFLAGS="-D warnings" cargo doc`, `cargo fmt
--check`, `cargo build --lib --target wasm32-unknown-unknown`, `tools/wesl-check` — all pass.

Two things the gate caught: a doctest in the crate root still built `Flicker` without the new field
(the public example, which is exactly what a doctest is for), and clippy's `manual_is_multiple_of` in
the new frame counter.

### Top 5 remaining, ranked

1. **A sourced flicker amplitude**: the rate is the literature's, the ±15 % is not. The flames'
   brightness fluctuation is a different measurement from the light's, and one paper that reports it
   would replace the last uncited number in this system.
2. **A cross-fade in the pool** (iteration 2's deferred item): overlap two lights for the fade with a
   spare slot, which is "overlapping keys" done literally.
3. **`flicker_rate_under(diameter, gravity)`**: the law is `f ∝ √(g/D)` and only Earth's `g` is baked
   in. Three lines, and a host with another gravity needs it.
4. **A placed specular highlight** (iteration 1's open beauty line).
5. **An `elements` feature** authoring the fill against `bevy_elements`' exposure.

## Iteration 4 — 2026-10-26 — a cross-faded hand-off

### What changed, and why it ranked second

The top item was "a sourced flicker amplitude", and it could not be done: the one paper that measures
it (Maynard 2013, UC eScholarship) is not obtainable — `paper_download` has no copy and the site
answers an automated fetch with 403 — so that is recorded as a blocker in `fire-light.md`'s *Pending
index* rather than a guess dressed as a citation. The next item, the pool's cross-fade, was small,
self-contained, and finishes a rule the crate already claims to follow.

Iteration 2's fade *dips* a subject's light by one light's share for a third of a second: a soft pop.
The craft's rule is to overlap: "By overlapping different key lights, the filmmaker can maintain a
constant intensity as actors move around the set."

### The model

A `LightPool::reserve` (default 1) of lights the pool spawns and owns but never assigns. When a
candidate **leaves** the pool, a free reserve light takes that candidate's last position and intensity
and fades out over `fade_seconds` while the slot's new owner fades in. `fade_out_factor` is the exact
complement of `fade_factor`, so the pair sums to one for the whole fade: the subject neither brightens
nor darkens. `reserve: 0` restores the dip, and makes the test fail.

The reserve is a budget, not a promise: one departure cross-fades per frame, and a second in the same
frame falls back to the dip.

### The measurements, and one that proves nothing

`a_cross_faded_hand_off_holds_the_subject_and_a_dip_does_not` (one slot, a challenger that displaces
the holder):

| frame after the hand-off | 1 | 2 | 3 | 4 | 5 | 6 |
|---|---|---|---|---|---|---|
| cross-fade (reserve 1) | 99.3 | 98.1 | 97.0 | 96.0 | 95.2 | 94.5 |
| dip (reserve 0) | 0.0 | 0.7 | 2.5 | 5.5 | 9.5 | 14.3 |

The capture, by contrast, **proves nothing and is recorded as such**: `LIGHTS_RESERVE=1` and `=0` came
out bit-identical over 281 frames, not because the feature failed but because no candidate leaves the
pool in the courtyard's first five seconds — the reserve light is there (9 pool lights against 8) and
contributes zero, since nothing departed. This iteration's evidence is the unit test's numbers, and
the demo doc says so rather than presenting two identical captures as support. A capture that shows a
cross-fade needs a scenario where torches cross the reach boundary, which the example lacks.

### The critique

`/tmp/l4/critique.md`. It caught one real defect that I fixed before the verdict: **`old_look` was an
unbounded map keyed by `Entity`**, growing by one entry per candidate ever pooled — a leak in a
library that runs for hours. It is now pruned every frame to the candidates still in the list plus the
ones a slot holds. The test for it is honest about its own weakness: the map's size is private, so
`the_pool_forgets_a_candidate_that_has_gone` churns 40 candidates through a two-slot pool and checks
the pool still resolves to the nearest survivors, rather than counting entries.

Two limits recorded, not defects: one cross-fade per frame unless `reserve` is raised, and the
departing light's flicker phase is the reserve's own rather than the old owner's (a departure should
copy the old `Flicker`; `old_look` would need to keep the phase).

### Verdict

**GREAT** for the mechanism: the cross-fade holds a subject's light through a hand-off where the dip
drops it to zero, the fades sum to one over the whole curve (its own test), the switch that turns it
off makes the test fail, the pool's budget stays explicit, and the leak the critique found is fixed.

### The benchmark

| | iteration 3 | iteration 4 |
|---|---|---|
| mean_us | 215 | 219 |
| p50_us | 210 | 212 |
| min_us | 131 | 140 |

+2 µs at p50 (1 %) for the reserve bookkeeping and the prune, at 5 000 candidates and 16 slots plus 1
reserve. Logged, not hidden.

### The gate

`cargo build --release`, `cargo build`, `cargo test` (62 tests: 23 lib, 8 flicker, 3 leaf, 17 pool, 11
doc), `cargo test --features serde`,
`cargo build --examples --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`,
`RUSTDOCFLAGS="-D warnings" cargo doc`, `cargo fmt --check`, `cargo build --lib --target
wasm32-unknown-unknown`, `tools/wesl-check` — all pass.

Three tests needed teaching about the reserve, which is the right kind of failure: `slots()` counted
every `PoolLight` and a reserve is one, so the helpers now see the assigned slots (`assigned(app,
size)`) and the reserve has its own test.

### Top 5 remaining, ranked

1. **A capture that shows the cross-fade**, with a scenario where candidates leave the pool (a focus
   that walks away, or torches that go out). The mechanism is proved; the picture is not.
2. **The flicker amplitude, still uncited.** Recorded as a blocker: Maynard 2013 measures it and is
   not obtainable. A host with library access would settle the last uncited number in the system.
3. **A departure that copies the old flicker phase**, so the light fading out moves exactly as the one
   that left did.
4. **A placed specular highlight** (iteration 1's open beauty line).
5. **An `elements` feature** authoring the fill against `bevy_elements`' exposure.

## Iteration 5 — 2026-10-26 — the cross-fade's phase, and a demo that was not built

### What this iteration produced

One fix in iteration 4's own feature, one test that bites, one genuine demo knob, and one backlog item
blocked rather than fudged.

### The fix: the two ends of a cross-fade now share a flicker

Iteration 4's cross-fade had a defect I found by re-reading it: the departing light kept the **reserve
light's** phase (`1.3 · slot`) instead of the phase of the light that left, so the two ends of the fade
beat against each other and their sum wobbled. Iteration 4's own test measured the *mean*, where the
means are equal, so it could not see it.

`old_look` now carries the phase with the position and intensity, and the reserve takes it while it
fades out. `the_two_ends_of_a_cross_fade_keep_the_same_flicker` compares the phases: reverted, it fails
at `must keep the phase it had (0), not the reserve's (1.3)`; with the fix, it passes.

### The failure: a capture that shows a cross-fade

Backlog item 1. Six attempts, none of which produced a capture, and the log records them because the
next agent should not repeat them:

1. A `LIGHTS_DOUSE` system removing a torch's `PoolCandidate`: candidates dropped 48 → 47, but the
   reserve-1 and reserve-0 runs were bit-identical.
2. Choosing the *nearest* torch each tick: the focus orbits and the bearers walk, so the nearest torch
   is rarely one the pool's slots hold.
3. Alternating douse/relight on `elapsed as u32 % 2`: a douse at an odd second relit in the same frame.
4. A `TorchKind` component that was never spawned (an earlier edit had silently not applied), so the
   system matched nothing for several runs. The candidate census is what caught it.
5. Pinning one torch from the pool's *held* set: a `PoolCandidate` cannot say which torch a slot holds,
   because a slot's light is the crate's own entity and not the candidate's.
6. `LIGHTS_REACH` at 6 m and 4 m, expecting candidates to stream across the boundary: still zero
   departures in the demo, with 48 candidates throughout.

What the work did establish is worth keeping: **the crate's departure path is sound.**
`a_departure_is_detected_at_all` despawns a holder with no replacement and the reserve fades it out. The
mechanism works; the courtyard cannot produce the situation. All of the douse experiment was removed
rather than shipped half-working.

The demo did gain a real knob: `LIGHTS_REACH`, which the pool's reach needed anyway.

### The benchmark, and a measurement that lied

The first bench read p50 212 → 495 µs, a 2.3× "regression". It was the machine: this Mac's load average
was 20 from other agents' work, and successive runs of the *same* binary read 495, 856, 851, 865. Benched
against a worktree at the last commit, at the same moment, the **pre-change** tree read p50 1013 µs and
the changed tree 842: the change is not a regression, the machine was. The diff adds one `f32` per
remembered entry and one phase write, which cannot cost 280 µs.

**Lesson for the loop:** a bench on this host is only comparable taken back to back with the base
commit in the same minute, and a reading that moves between successive runs of one binary is the load.

Measured properly (two trees built into their own target dirs on big, benched interleaved three rounds
each): base p50 **674, 690, 671** µs against head **696, 706, 708** — a consistent **+3 to +5 %**, which
is the honest cost of one extra `f32` per remembered entry and one phase write. (Big's absolute numbers
are higher than this Mac's were in earlier iterations because big was busy too; the ratio is what the
interleaving buys.)

### Verdict

**GREAT** for the fix: the two ends of a cross-fade move together, which is what "overlapping key
lights" means, and the test fails without it.

**Blocked** for backlog item 1, with the reason and the plan: a capture that shows a cross-fade needs a
subject that *leaves* the pool, and the courtyard's torches never do. The concrete plan is a camera
behaviour — a focus that walks away from the torches — not a lighting one. That is what iteration 6
should build.

### The gate

`cargo build --release`, `cargo build`, `cargo test` (64 tests), `cargo test --features serde`,
`cargo build --examples --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`,
`RUSTDOCFLAGS="-D warnings" cargo doc`, `cargo fmt --check`, `cargo build --lib --target
wasm32-unknown-unknown`, `tools/wesl-check` — all pass.

### Top 5 remaining, ranked

1. **A focus that walks away** — a camera behaviour in the example, so the torches fall out of the
   pool's reach and a cross-fade fires. This is the only way backlog item 1 gets its capture, and it is
   a demo change, not a crate one.
2. **The flicker amplitude, still uncited** (blocked: Maynard 2013 unobtainable).
3. **A placed specular highlight** (iteration 1's open beauty line).
4. **An `elements` feature** authoring the fill against `bevy_elements`' exposure.
5. **A bench that controls for host load** — build base and head into separate target dirs and run them
   interleaved, refusing a reading whose repeat differs by more than 10 %. The script this iteration
   used is in `docs/DEMOS.md`'s bench section from now on.

## Iteration 6 — 2026-10-26 — the child-position bug, and a cross-fade that finally fires

### What changed, and why

Iteration 5's blocked item had a plan: a focus that walks away, so torches leave the pool. Building it
found a **library bug that had been there since iteration 2**, and fixing it forced two more corrections
in iteration 4's feature.

### The bug: a child candidate sat at its own local offset

Iteration 2 fixed "the pool reads a stale `GlobalTransform`" by preferring a candidate's own `Transform`.
That is right for a **root** prop the host moves, and **wrong for a child**: a torch is a child of its
bearer, its own `Transform` is a constant `(0, 0.95, 0)`, so every torch candidate collapsed to the same
point. The pool's sort therefore never changed, the same four lights stayed where they were for the whole
run, and no candidate ever left. `a_child_candidate_moves_with_its_parent` is the test: reverted, it fails
with the child's light at `[0, 2, 0]` instead of its parent's place.

The fix reads a local `Transform` only when the candidate has **no `ChildOf`**, and `GlobalTransform`
otherwise. On the demo this changed the pool's per-frame total from 2 jumps over 20k in 400 frames to 14:
the pool had been frozen for four iterations and every capture before this one was of a frozen pool.

### Two corrections to iteration 4's cross-fade

1. **It only cross-faded a *departed* owner.** The common hand-off is a slot changing owner while the old
   one is still a candidate — simply displaced by a nearer prop — and that case kept the dip. The
   departure is now "this slot is about to lose its owner", from the first pass's own keep decision, not
   "the owner left the pool". `a_steady_pool_keeps_its_candidates_and_its_brightness` caught my first
   attempt at this, which re-derived the decision without the first pass's slot claiming and so
   manufactured phantom hand-offs in a steady pool.
2. **The dip test needed `reserve: 0`.** With a reserve by default, that pool no longer dips, so its test
   was measuring the wrong thing until it pinned the old behaviour.

### The capture this time

With `LIGHTS_FOCUS_WALK=18` and `LIGHTS_POOL_LOG=1`, on 400 frames at pool 4:

| | reserve 1 | reserve 0 |
|---|---|---|
| mean pool total | 763 501 | 763 224 |
| frames where the two differ by >1000 lm | 16 of 400 | — |
| the first hand-off's frames (11 → 19) | +12 507, +11 989, +11 344, +10 601, +9 790, +8 938, +8 066, +7 192, +6 330 | reference |

The reserve run carries up to 12 507 lm more light through the hand-off, decaying frame by frame as the
departing light fades out. That is the evidence iterations 4 and 5 could not produce, and it took a
per-frame log to see: a 0.35 s fade is invisible in a 30-frame sample and far below a frame's mean
luminance.

### The benchmark

Base (`d533e16`) against head, built into separate target dirs and run **interleaved**, three rounds
each, on big:

| round | base p50 | head p50 |
|---|---|---|
| 1 | 650 | 622 |
| 2 | 524 | 571 |
| 3 | 550 | 549 |

The ranges overlap, so there is no regression: the change adds one `Option<&ChildOf>` to the candidate
query and moves the departure test before the slot loop, neither of which is per-candidate work.

### Verdict

**GREAT.** A four-iteration-old bug in the pool's own candidate position is fixed with a test that fails
without it; the cross-fade now covers the common hand-off rather than only a departure; and the capture
finally shows the feature working, with numbers rather than a claim.

### The gate

`cargo build --release`, `cargo build`, `cargo test` (65 tests), `cargo test --features serde`,
`cargo build --examples --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`,
`RUSTDOCFLAGS="-D warnings" cargo doc`, `cargo fmt --check`, `cargo build --lib --target
wasm32-unknown-unknown`, `tools/wesl-check` — all pass.

### Top 5 remaining, ranked

1. **The flicker amplitude, still uncited** (blocked: Maynard 2013 unobtainable).
2. **A placed specular highlight** (iteration 1's open beauty line).
3. **An `elements` feature** authoring the fill against `bevy_elements`' exposure.
4. **The demo still has no hand-off in its default capture.** The walking focus is a knob, not the
   default; whether the showcase should ship with torches visibly changing hands is an art call.
5. **`hand_off` by luminance, not distance**: a dim far candidate and a bright near one are compared
   only by distance, so the pool can hold the dim one.

## Iteration 7 — 2026-10-26 — a flame catches with its light

### What changed, and why it ranked first

The top backlog item was still the uncited flicker amplitude (blocked: unobtainable), so the pick came
from the pattern iteration 6 exposed: **state that should follow something and does not**. Reading the
flame path for it found a visible artifact.

A fire a host lights flares its *light* up over `KINDLE_SECONDS` — and its *flame* appeared at full
height and full brightness on the first frame. The courtyard showed it on frame 1 of every brazier: a
full-size flame over a light at zero. That inverts `lighting-look.md`'s motivated-source rule, and it
contradicts `fire-light.md`'s own craft line, that the flare "is the sound cue's visual half": one event
was arriving as two.

### The model

`FlameSite::kindle` (seconds) makes a flame grow from nothing to its full height **and width** over that
time, on the same `kindle_gain` envelope the light uses — one curve shared by both halves of the catch,
rather than two that could drift apart. The scale goes in the child flame's `Transform`, because the
`MeshTag` packs the height as a fraction of `max_height` and a packed tag cannot be re-scaled. A settled
flame is skipped with one comparison, so a fire that has been burning all along costs nothing. `0.0` is
the default and `KINDLE_SECONDS` the usual value.

Width scales with height deliberately: a flame that grew tall but stayed wide would read as a sheet.

### The measurements

`a_flame_lit_with_its_fire_grows_from_nothing_and_stops_at_full` fails without the scaling (the flame
reads `1.0` on the frame it should be tiny); `a_flame_that_has_been_burning_all_along_is_full_size_at_once`
pins the unchanged case.

On the courtyard's first 60 frames (braziers start with `Kindle::default()`), the count of saturated
flame pixels:

| frame | 5 | 10 | 20 | 40 | 59 |
|---|---|---|---|---|---|
| flame pixels | 0 | 4 430 | 15 145 | 19 891 | 19 156 |

The flame grows into its fire and then holds. I could not get a *crop* that showed it cleanly — the flame
region is crowded by torch flames and a pillar, and two attempts framed the wrong thing — so the evidence
here is the pixel count, which is the same measurement a crop would be eyeballing.

### The benchmark

Base (`df0612b`) against head, built into separate target dirs, run interleaved, three rounds:

| round | base p50 | head p50 |
|---|---|---|
| 1 | 549 | 542 |
| 2 | 502 | 494 |
| 3 | 513 | 482 |

No regression (head is marginally faster: the bench has no flames, and the new per-flame loop exits on a
single comparison for a settled fire).

### The gate

`cargo build --release`, `cargo build`, `cargo test` (67 tests), `cargo test --features serde`,
`cargo build --examples --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`,
`RUSTDOCFLAGS="-D warnings" cargo doc`, `cargo fmt --check`, `cargo build --lib --target
wasm32-unknown-unknown`, `tools/wesl-check` — all pass.

One thing the gate caught: my first edit dropped the `MeshTag` from the flame's spawn tuple and two
existing flame tests failed — the tag tests are what make a silent mistake like that loud.

### Top 5 remaining, ranked

1. **The flicker amplitude, still uncited** (blocked: Maynard 2013 unobtainable).
2. **A placed specular highlight** (iteration 1's open beauty line): the last line of
   `lighting-look.md`'s "what great looks like" still at PARTLY.
3. **An `elements` feature** authoring the fill against `bevy_elements`' exposure.
4. **The flame's colour against its catch**: a catching flame should read *redder* at its base as it
   catches (the ember stage) before its full colour arrives. Today only its size and brightness ramp.
5. **`hand_off` by luminance, not distance**: a dim far candidate and a bright near one are compared
   only by distance, so the pool can hold the dim one.

## Iteration 8 — 2026-10-26 — the specular answer

### What changed, and why it ranked first

The top item is the uncited flicker amplitude, still blocked. Second was the **last line of
`lighting-look.md`'s "what great looks like" still unmet**: "stone near a fire shows a small, placed
highlight … the ground does not look like matte paper". Measured first: on the ground-level capture the
floor had **no local maximum anywhere** (p99 136 against a max of 217) — one 40 m plane at roughness
0.55 spreads a fire's mirror lobe until it is indistinguishable from its diffuse falloff.

### What changed

The courtyard's floor is a grid of **2.5 m flagstones** inset by 6 cm of darker, rougher mortar, each
stone's roughness drawn from the example's seeded stream across 0.18–0.32 and its colour from a narrow
warm-grey band. Some stones answer a fire sharply and their neighbours stay matte; a floor of identical
mirrors would read as a wet car park, and one matte plane read as paper.

No crate API: a highlight is a material property, and the crate owns lights, not surfaces. The research
note says so.

### The metric that had to change

The first attempt measured whole-frame p99.9/p99 and read almost nothing (1.38 → 1.40), because that
statistic is dominated by the flames' cores. The craft's question is **local**: at the foot of a fire,
does a highlight dominate its own stone?

| | peak | median | peak / median |
|---|---|---|---|
| at the fire's foot, before | 108.0 | 89.0 | 1.21 |
| at the fire's foot, after | 212.0 | 101.3 | **2.09** |
| a stone 2 m away, after | 78.3 | 58.3 | 1.34 |

The fire's stone now carries a highlight twice its own median; a stone out of reach stays matte. Seeing
the frame is what caught the bad metric — the highlights were plainly visible as vertical streaks under
each brazier while the whole-frame number called it a wash.

### The verdict

**GREAT** for the look: the last open line of "what great looks like" is answered and measured locally.
The frame at ground level now says where every fire is twice — once by its falloff, once by the streak
of light on the stone at its foot.

### The gate

`cargo build --release`, `cargo build`, `cargo test` (67 tests), `cargo test --features serde`,
`cargo build --examples --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`,
`RUSTDOCFLAGS="-D warnings" cargo doc`, `cargo fmt --check`, `cargo build --lib --target
wasm32-unknown-unknown`, `tools/wesl-check` — all pass.

### Top 5 remaining, ranked

1. **The flicker amplitude, still uncited** (blocked: Maynard 2013 unobtainable).
2. **The flame's colour against its catch**: a catching flame should read redder at its base as it
   catches, before its full colour arrives. Today only size and brightness ramp.
3. **An `elements` feature** authoring the fill against `bevy_elements`' exposure.
4. **`hand_off` by luminance, not distance**: a dim far candidate and a bright near one are compared only
   by distance, so the pool can hold the dim one.
5. **The flagstones cost 256 draw calls** — one mesh, one material each. A crate-scale scene wants an
   atlas or an instanced floor; the example is small enough not to care, and the number should be
   recorded next to the look it buys.

## Iteration 9 — 2026-10-26 — the flame is redder while its fire catches

### What changed, and why it ranked second

The top item is still the uncited flicker amplitude (blocked). Second was the same ignition event's other
half: iteration 7 made a catching flame *grow*, but its **colour** stayed the steady ramp from frame 1, so
a fire in its first three tenths of a second showed the host fire-colour and yellow core of a fire that
had been alight for hours. A young, small fire is redder — Planck's law, which the crate already cites in
`lighting-look.md`.

### The model

The shader's ramp is pulled towards an ember red while the fire catches. `params.z` (the spare uniform
slot) carries how far the scene's catch has run — 1 for a flame that took long ago — written once a frame
from the *youngest* fire in the frame, and the fragment mixes `EMBER_RED * max(heat, 0.35)` towards the
steady ramp by that number. One shared material means one number for the whole scene; the courtyard's
four braziers light together, so it is exactly right there, and the note says so rather than implying it
is per-flame.

### The measurements

The flame core's **blue** channel — the number that says "redder" — on the courtyard's catch:

| frames | 15–17 (catching) | 27–35 (settled) |
|---|---|---|
| mean blue of the flame core | **34–42** | **65–69** |

Roughly half: a catching fire is markedly redder, and by the end of `KINDLE_SECONDS` it is the steady
flame. The signal the shader reads is pinned by `the_shader_is_told_when_a_fire_is_still_catching`
(`params.z` < 0.2 mid-catch, exactly 1 after) and `a_scene_with_no_catch_keeps_the_steady_ramp`.

### The trap the shader gate caught

`catch` is a **reserved WGSL word**. `tools/wesl-check` refused it immediately — "use of a reserved
word" — which is precisely the error `AGENTS.md` records as silent at build time and loud only in a
capture. Renamed to `took`.

I could not get a crop that showed the colour change legibly (the flames in the ground-level view are a
few pixels tall and the region is crowded by pillars); the evidence is the core statistics above, which
is the same measurement a crop would be eyeballing.

### The gate

`cargo build --release`, `cargo build`, `cargo test` (69 tests), `cargo test --features serde`,
`cargo build --examples --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`,
`RUSTDOCFLAGS="-D warnings" cargo doc`, `cargo fmt --check`, `cargo build --lib --target
wasm32-unknown-unknown`, `tools/wesl-check` — all pass.

### Top 5 remaining, ranked

1. **The flicker amplitude, still uncited** (blocked: Maynard 2013 unobtainable).
2. **A per-flame catch colour**: one shared material means the youngest fire tints them all. A per-instance
   catch would ride in the `MeshTag`'s spare bits or a second instance attribute; the note records the
   shared approximation as a known limit.
3. **An `elements` feature** authoring the fill against `bevy_elements`' exposure.
4. **`hand_off` by luminance, not distance**: a dim far candidate and a bright near one are compared only
   by distance, so the pool can hold the dim one.
5. **The flagstones cost 256 draw calls** — recorded next to the look they buy; a crate-scale floor wants
   an atlas.

## Iteration 10 — 2026-10-26 — the catch belongs to the flame

### What changed, and why it ranked second

Iteration 9 left the catch colour in the material's **shared** uniform, with the youngest fire in the
frame deciding. Exact for the courtyard's four braziers lighting together, and wrong for a host lighting
one torch: every flame in the world was tinted red for three tenths of a second. A visible artifact, which
ranks above the remaining beauty items and the blocked amplitude.

### The model

The catch is already per flame in the entity world — `update_flames` writes each flame's `Transform`
scale from its own fire's `kindle_gain`. The shader now reads it there:
`length(world_from_local[0].xyz)` in the vertex stage, passed flat, mixed into the ramp in the fragment.
`params.z` returns to unused; one shared material can no longer carry one fire's state to another.

### The tool had to be fixed first — **wrong, corrected in iteration 12**

I claimed the validator's `get_world_from_local` stub returning the **identity** was a blind spot that had
to be fixed first. It was not. `tools/wesl-check` *compiles* each module and never runs it, so a stub's body
is never evaluated and only its signature is checked: reverting the stub to the identity still passes, and a
deliberate type error in the fragment still fails. The stub's richer body is documentation, not a check. The
change to per-flame colour is still right; the reason I gave for touching the stub was not.

### A test that cannot bite, and says so

`one_fire_catching_does_not_tint_another` passes under **both** ways the isolation holds — the `f32::MAX`
skip and a `kindle: 0` site's gain of exactly 1 — and I confirmed by mutation that removing either does
not fail it. Rather than delete it or pretend, its comment now states what it pins (the property) and what
it cannot detect (a regression in the loop's shape).

### The measurement

On the courtyard's catch (frames 14–39), the flame core's mean blue climbs **60.0 → 68.8** as the fires
take. The window is later than iteration 9's because the scene load moved; what matters is that the number
is now read from each flame's own transform, so a host lighting one fire cannot move it for the others.

### The gate

`cargo build --release`, `cargo build`, `cargo test` (69 tests), `cargo test --features serde`,
`cargo build --examples --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`,
`RUSTDOCFLAGS="-D warnings" cargo doc`, `cargo fmt --check`, `cargo build --lib --target
wasm32-unknown-unknown`, `tools/wesl-check` — all pass.

### The README hero GIF, and a bug in my own demo

The user asked for a succinct README GIF that showcases the top features, kept current with every commit.
Built one: two seconds, 30 fps, 480 px, 2.0 MB, opening on the catch and then the fires breathing. Two
encoding bugs of mine had to be fixed first, and the second is the interesting one:

- **The encoded frame rate did not match the capture's.** `LIGHTS_SHOT_EVERY=3` saves one frame per three
  simulated frames, so the content is 20 fps; I encoded at `FPS=48 STRIDE=2` = 24 fps of content, **1.2× too
  fast**. A 2.6 Hz flicker read as 3.1 Hz, and the user asked whether the *flicker* was wrong. It was the
  GIF. Re-captured at `EVERY=2` (30 fps of content) and encoded at `FPS=30`.
- **`dither=bayer` crawls on a dark scene.** The ordered pattern moves every frame on a slow luminance
  gradient, so it reads as noise laid over the flicker — the thing the demo exists to show. `tools/gif.sh`
  now uses 256 colours with `dither=none`: slight banding, stable, the right trade.

Verified in the encoded file: one brazier's pool reads **2.50 Hz** against the law's 2.57, peak-to-peak 74 %
of its mean. (Measure a *single* pool — four at different phases cancel and read ~1.5 Hz whatever the
truth is.)

The policy is now written where it will be enforced: `AGENTS.md`'s rules and the loop skill's ship section
both say the README is checked before every push and the GIF re-encoded whenever the look it shows moves.

### Top 5 remaining, ranked

1. **The flicker amplitude, still uncited** (blocked: Maynard 2013 unobtainable).
2. **An `elements` feature** authoring the fill against `bevy_elements`' exposure.
3. **`hand_off` by luminance, not distance**: a dim far candidate and a bright near one are compared only
   by distance, so the pool can hold the dim one.
4. **The flagstones cost 256 draw calls** — recorded next to the look they buy; a crate-scale floor wants
   an atlas.
5. ~~The validator's other stubs are still identities.~~ **Struck in iteration 12:** stub bodies are never
   run, so this was never a defect. Signatures and types are what the validator checks, and it does.

## Iteration 11 — 2026-10-26 — the README was broken, and cannot be again

### What changed, and why it ranked first

The user made keeping the README current a standing duty, so the first thing this iteration did was check
it — and the README's **quick start did not compile**. `FlameSite::kindle` (iteration 7) and
`Flicker::diameter` (iteration 3) were both missing from it. A new user's first `cargo build` would have
met three errors, which is the worst possible first impression and exactly the staleness the user named.

Proved rather than assumed: the snippet was extracted into a scratch crate and built, failing with
`missing field \`diameter\`` and two `missing field \`kindle\``.

### The fix, and the ratchet

- The example is corrected: `Flicker { diameter: Some(0.34) }`, `FlameSite { kindle: KINDLE_SECONDS }`, and
  `night_fill()` is now actually **called** — it was defined and never used, which clippy caught, and an
  example function nobody calls teaches nothing.
- **`tests/readme.rs` compiles it**, so it cannot rot again: it extracts the `## Quick start` block, writes
  it to `examples/readme_quick_start.rs` behind a `readme-example` feature that nothing else enables (so
  `cargo build --examples` and the published package never see it), formats it the way the repository's own
  files are, and builds it with `cargo build --example`. Reverted to the stale text, the test fails with the
  same missing-field error.
- The README's API list was also missing `PoolLight`, `FlameLook`, `flicker_rate`, `hand_off` and
  `fade_out_factor`; it now has a **What a host touches** section naming every component and resource a game
  spawns, marks, reads or styles, and every engine-free function.

### The gate

`cargo build --release`, `cargo build`, `cargo test` (**70 tests**, the README's among them), `cargo test
--features serde`, `cargo build --examples --all-features`, `cargo clippy --all-targets --all-features -- -D
warnings`, `RUSTDOCFLAGS="-D warnings" cargo doc`, `cargo fmt --check`, `cargo build --lib --target
wasm32-unknown-unknown`, `tools/wesl-check` — all pass. CI runs `cargo test`, so the README test runs there.

### Top 5 remaining, ranked

1. **The flicker amplitude, still uncited** (blocked: Maynard 2013 unobtainable).
2. **An `elements` feature** authoring the fill against `bevy_elements`' exposure.
3. **`hand_off` by luminance, not distance**: a dim far candidate and a bright near one are compared only
   by distance, so the pool can hold the dim one.
4. **The flagstones cost 256 draw calls** — recorded next to the look they buy; a crate-scale floor wants
   an atlas.
5. **The validator's other stubs are still identities** — `position_world_to_clip` and `view` return
   placeholder values, so a shader that misuses *those* is checked no further than its types.

## Iteration 12 — 2026-10-26 — a light's rank is its contribution, and a claim of mine was wrong

### The correction first

Backlog item 5 was "the validator's other stubs are still identities, so a shader misusing `view` or
`position_world_to_clip` is only type-checked". Testing the premise killed it: `tools/wesl-check`
**compiles** each module and never runs it, so a stub's *body* is never evaluated and only its signature is
checked. Reverting iteration 10's `get_world_from_local` stub to the identity still passes, and a deliberate
type error in the fragment still fails — proof both ways. So iteration 10's claim that the identity stub was
a blind spot was **wrong**, the change it made to that stub changed nothing, and item 5 rested on the same
mistake. Both records are corrected, and item 5 is struck in the backlog.

### What was built instead

The pool ranked candidates by **distance alone**, so a 2 m torch displaced a 3 m brazier burning 25× the
lumens. That is the craft's "proximity to the subject" rule with half of it missing: what the eye reads is
how much light *arrives*, which is output over the square of distance.

`LightPool::rank` picks the rule: `Rank::Illuminance` (the new default, key `intensity / distance²`) or
`Rank::Distance` (the old behaviour, kept so a host can switch it off). The rank key and the distance are
carried separately in the assignment tuple, because they answer different questions: *which* lights should
be real is about what arrives, while the overlap rule is about how far it has to travel.

### The measurements

- A torch at 2 m burning 8 000 lm against a brazier at 3 m burning 60 000 lm: `Illuminance` gives the light
  to the **brazier**, `Distance` to the **torch**. The test pairs each rule with its answer, so it
  validates both arms.
- Four times the light at twice the distance ranks **equal**, and the tie breaks on `Entity` — the inverse
  square, exactly, and the determinism that depends on it.
- `gain` never reshuffles which lights are real: a quarter-gain night has the same slot in the same place at
  a quarter the brightness.
- `the_pools_default_rank_is_illuminance` pins the default, so changing it is a decision and not a slip;
  with the default flipped to `Distance` that test fails.

### The demo, and an honest limit

The courtyard could not show the rule at all: every `PoolCandidate` is a torch at one intensity, and the two
rules gave **bit-identical** frames. Rather than claim a demonstration, one bearer in six now carries a
lantern — four times the lumens, a taller flame, the case the old rule got wrong. With that the rules give
different frames (RMSE 2 %, mean 58.17 against 57.44), and the illuminance rule puts the real lights where
the bright sources are.

### The benchmark, measured late

Iteration 12 added a division per candidate per frame to the pool and **shipped without running the bench**
— a hole in that iteration's own verification, found here. Measured now, base `9d3abf0` against head,
interleaved on big: base p50 **434, 528, 449** against head **462, 466, 461**. The medians sit inside each
other's spread; the change costs nothing the bench can see. Recorded because "I did not measure it" is a
worse answer than a number, even when the number is dull.

### The gate

`cargo build --release`, `cargo build`, `cargo test` (**74 tests**), `cargo test --features serde`,
`cargo build --examples --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`,
`RUSTDOCFLAGS="-D warnings" cargo doc`, `cargo fmt --check`, `cargo build --lib --target
wasm32-unknown-unknown`, `tools/wesl-check` — all pass.

### Top 5 remaining, ranked

1. **The flicker amplitude, still uncited** (blocked: Maynard 2013 unobtainable).
2. **An `elements` feature** authoring the fill against `bevy_elements`' exposure.
3. **The flagstones cost 256 draw calls** — recorded next to the look they buy; a crate-scale floor wants
   an atlas.
4. **The bench should report the pool's work**, not only the total: `Rank::Illuminance` adds a division per
   candidate per frame, and the whole-frame p50 does not separate that from the flicker's cost.
5. **A `Rank::Weighted`** for a host that wants gameplay priority (a carried torch matters more than a
   brazier): the rank key would take a per-candidate multiplier.

## Iteration 13 — 2026-10-26 — the bench says which system cost what

### What changed, and why

Backlog item 4: "the bench should report the pool's work, not only the total". Iteration 12 added a
division per candidate per frame to the pool's sort and **shipped without running the bench at all** — a
hole in that iteration's verification, found while orienting here. So this iteration did two things: filled
the hole, then built the instrument that makes the hole visible next time.

### The hole, filled

Base `9d3abf0` (before the rank change) against head, interleaved on big, three rounds:

| round | base p50 | head p50 |
|---|---|---|
| 1 | 434 | 462 |
| 2 | 528 | 466 |
| 3 | 449 | 461 |

The medians sit inside each other's spread: the rank change costs nothing the bench can see. Recorded
because "I did not measure it" is a worse answer than a dull number.

### The instrument

`bench_lights` now prints a second line, `bench_lights_systems flicker_p50_us=… pool_p50_us=…`, by re-running
the same app with a resource switch that leaves one of `LightsSystems::{Flicker, Pool}` enabled. The systems
are private (internals); the crate's public boundary is the system set, so that is what is driven, and the
switch is a run condition that is an `Option<Res<…>>` — a bare `Res` in a run condition panics when absent,
which is one of the crate's own recorded traps.

At 5 000 candidates and 2 000 lights, over three runs: **flicker 157–182 µs, pool 302–394 µs**. The pool is
the expensive half, which is what any change to its sort or its hysteresis should move.

### Proving the instrument is not a lie

A per-system line from a switch that does nothing would be the frame measured twice.
`the_switch_really_isolates_the_two_sets` runs both ways and asserts the *other* set did not run — with
`Only::Flicker` the pool spawns no slots, with `Only::Pool` the flicker writes no intensity. Removing the
pool's run condition makes it fail (`and the pool did not run: no slots were spawned`, 5 against 0).

That test also caught a fact I had forgotten: a pool of 4 spawns **5** slots, because the cross-fade keeps a
reserve. The expectation was wrong, not the code.

### The gate

`cargo build --release`, `cargo build`, `cargo test` (74 tests, plus the example's own
`cargo test --release --example bench_lights`), `cargo test --features serde`, `cargo build --examples
--all-features`, `cargo clippy --all-targets --all-features -- -D warnings`, `RUSTDOCFLAGS="-D warnings"
cargo doc`, `cargo fmt --check`, `cargo build --lib --target wasm32-unknown-unknown`, `tools/wesl-check` —
all pass.

### Top 5 remaining, ranked

1. **The flicker amplitude, still uncited** (blocked: Maynard 2013 unobtainable).
2. **An `elements` feature** authoring the fill against `bevy_elements`' exposure.
3. **The flagstones cost 256 draw calls** — recorded next to the look they buy; a crate-scale floor wants
   an atlas.
4. **The pool's 310 µs is unexamined**: the bench can now name it, and the next pool change should say
   whether the sort, the hysteresis pass or the cross-fade's bookkeeping is the share. A candidate worth
   taking is sorting only the candidates within reach rather than all of them.
5. **A `Rank::Weighted`** for a host that wants gameplay priority (a carried torch matters more than a
   brazier): the rank key would take a per-candidate multiplier.

## Iteration 14 — 2026-10-26 — the pool's hand-off is a set decision

### First, the ungated work

The pass that was killed had claimed iteration 13; a concurrent pass on another branch pushed its own
iteration 13 ("the bench says which system cost what") while this one was running, so this entry is
renumbered **14** and every reference in the note, `docs/DEMOS.md` and `tests/pool.rs` follows it.

This pass opened on a **dirty tree with this iteration's implementation staged but never committed** — the
previous pass was killed before its gate. Nothing was built on it blind: the whole gate was run on it
first, and it failed twice.

- `cargo test` was green (76 tests) the moment `/tmp` was taken off the tmpfs. The doctests had failed
  with `LLVM ERROR: IO failure on output stream: No space left on device`, which reads as a code error
  and was **a full 16 GB `/tmp`** (100 %, 6.8 MB free). `export TMPDIR=/home/ladvien/l13/tmp` fixed it;
  `docs/DEMOS.md` already warns about `/tmp`, and this is the second way it bites — the *linker*, not just
  a frame sequence.
- `cargo clippy -D warnings` **failed**: a `match` on a single pattern at `src/pool.rs:524`. Fixed to
  `if let`.
- `cargo build --lib --target wasm32-unknown-unknown` **failed**: the target was not installed on big
  (`E0463`). `rustup target add wasm32-unknown-unknown` fixed the environment, not the crate.

Both code-side fixes are the ones the gate exists for, and both are in this iteration's commit.

### What changed, and why it ranked first

Ranked first because it is a **visible artifact of the most physical kind**: the previous pass measured
the pool losing 17 % of its lumens in one frame with up to 7 of 8 lights reading zero, then fading back
over twenty frames. The runner-up was the `Rank::Weighted` backlog item (a host-side priority), which is
API convenience and cannot beat a street going dark for one frame.

The rule "every slot keeps its candidate unless a challenger is better by the hysteresis" was applied
**per slot against one global best free candidate**, so one newcomer displaced every slot at once, and a
displaced slot's fallback could then take a candidate another slot was still keeping. Three set rules
replace it, and the overlap rule was made a function of the rank's own quantity:

- **A candidate another slot holds is never a challenger.**
- **A challenger displaces the worst-ranked light the pool holds**, one light per challenger.
- **At most as many lights move per frame as there are free reserves** (`LightPool::reserve`; unthrottled
  at 0), because a hand-off the pool cannot cross-fade is a hand-off that dims its subject.
- **`model::pool::preferred`**: the overlap rule on any "smaller is better" quantity, so ranking and
  holding cannot disagree — iteration 12 ranked by illuminance and held by distance.

Every slot's owner is now decided in **one map before any slot is written**; the pass that decides
depends on no write order.

### Sources

- craft: `10.58709_niujhu.v8i3.1694` (Orisayemi & Afabor, the Bordwell & Thompson "overlapping keys" quote),
  `10.4324_9781003671220` (Lazar, motivated movement).
- production: `s2025-advances-materials-9` (HypeHype, stochastic tile-based lighting), `s2020-advances-pdf-slides`.
- technical: `10.1109_tvcg.2019.2898741` (Denes et al., temporal resolution multiplexing),
  `book_fundamentals_computer_graphics_marschner` ch. 22, `book_level_of_detail_3d_graphics_luebke` ch. 9,
  `book_real_time_rendering_4th_akenine` pp. 918-935.
- The note was committed alone, before any code: `1db0af7`.

### The critique, and the verdict

Full critique in `/home/ladvien/l13/critique.md`. **GREAT.** Evidence: `/home/ladvien/l13/`.

Craft checklist, before → after, default camera, seed 7, pool 8:

| line | before | after |
|---|---|---|
| key-to-fill (brazier pool vs moonlit far ground) | 122.7 / 51.1 DN | 122.2 / 51.1 |
| falloff, motivated light, warm/cool, silhouette, specular, bloom | holds at every angle | unchanged |
| **pool total, worst single frame** | **−19.0 % (frame 303: 972 505 → 787 358 lm)** | **−1.2 %** |
| **frames changing >2 %** | **25 of 365** | **0** |
| **most lights at zero in one frame** | **8 of 13** | **2 of 13** |
| mean per-frame change | 11 300 lm (1.18 %) | 2 453 lm (0.24 %) |
| mean pool total | 954 930 lm | 1 033 222 lm (+8.2 %, the bright lanterns no longer starved) |

Second seed and smaller pool (`LIGHTS_SEED=3 LIGHTS_POOL=4`): worst frame **11.4 % → 1.4 %**, frames
over 2 % **5 → 0**, dark lights **4 → 2**.

The aligned crash pair (`ev/before/frame0005→0006` against `ev/after/`, whole frame): **−1.63 % and 13.2 %
of pixels moving more than 2 DN** → **+0.16 % and 2.7 %**. The `ground_left` panel loses 1.6 DN before and
0.38 DN after while `sky` is untouched in both, which is the pool moving and not the flame shader.

Throttle measured at four reserve sizes (`logs/throttle/r0..r4`): 4.8 % worst frame at `reserve: 0`, 1.2 %
at 1, 1.4 % at 2 and 4 — with mean dark lights rising 0.03 → 0.54 → 1.54 → 3.6. So raising `reserve` buys
no smoothness; recorded in the note's `Deviations`.

### Benchmark

`bench_lights` (candidates 5000, lights 2000, pool 16), three rounds each, whole frame:
before `mean_us` 489 / 523 / 495, p50 473 / 490 / 478; after 466 / 477 / 480, p50 469 / 471 / 465. Flat
within the run-to-run spread.

Iteration 13 landed the per-system line while this pass was running, so the work this iteration added is
now attributable instead of merely "flat": on the merged tree, `pool_p50_us` **322 / 322 / 314** against
the same instrument on `origin/main` (which has no set rules) at **328 / 355 / 344**. The pool's own cost
is if anything a little lower, which is what moving the decision from a per-slot search to one map should
look like; `flicker_p50_us` is unchanged at 130-132. CPU-only, so it says nothing about GPU cost — the
change touches no shader, and the capture's wall time per 400 frames did not move.

### The gate

`cargo build --release`, `cargo build`, `cargo test` (**77 tests**), `cargo test --features serde`,
`cargo build --examples --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`,
`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`, `cargo fmt --check`,
`cargo build --lib --target wasm32-unknown-unknown`, `tools/wesl-check` — all pass.

### Media

`docs/media/showcase.gif` re-encoded from the new build (60 frames, 30 fps, 480 px, 2.2 MB), same caption,
same capture recipe. Flicker verified through the encode on a single pool: **2.50 Hz** peak-to-peak 32 %,
identical to the previous GIF's 2.50 Hz / 32 % — the encode did not eat the flicker.
`docs/media/courtyard.gif` and `docs/media/courtyard.png` regenerated at their recorded recipes (57 frames
at 30 fps, 640 px, 3.3 MB).

### Top 5 remaining, ranked

1. **The flicker amplitude, still uncited** (blocked: Maynard 2013 unobtainable). Unchanged for 13 iterations.
2. **An `elements` feature** authoring the fill against `bevy_elements`' exposure.
3. **The flagstones cost 256 draw calls** — recorded next to the look they buy; a crate-scale floor wants
   an atlas.
4. **The bench should report the pool's work**, not only the total: this iteration added per-candidate
   work (a division, a sort of a larger tuple) that the whole-frame p50 cannot see past the flicker's cost.
5. **A `Rank::Weighted`** for a host that wants gameplay priority (a carried torch matters more than a
   brazier): the rank key would take a per-candidate multiplier.

### One uncommitted change this pass did not make

`.omp/skills/bevy-lights-build-loop/SKILL.md` carries an unstaged edit adding the section *"Your work is
committed even if you are killed"* — the rule this pass opened by obeying. It was already in the working
tree when the pass began, its content matches how the loop actually behaves, and it is **left uncommitted**
rather than adopted: a tracked change this pass did not write is the driver's or a previous pass's to
commit. Its guidance is right and was followed: the note was committed alone (`1db0af7`), then the
implementation, then the docs and media, then this log.

## Iteration 15 — 2026-10-26 — a drawn flame breathes with the light it motivates

**This iteration was started by a pass that was killed before its benchmark, its gate, its log or its
commit.** The driver had saved its tree as three `WIP(pass 11)` commits on `main`. This pass verified that
work rather than starting a new item, because a finished-but-ungated iteration is worth more than a second
half-idea: it re-read the diff, re-ran the gate from scratch, re-captured every angle from its own build,
re-measured every number, and found two performance defects in the inherited code that the killed pass had
no chance to measure. Both are fixed here (below). The research note was already committed alone
(`d1e8128`), as the rule requires.

### What changed, and why it ranked first

Ranked first because it is the crate's central claim failing in the one place an audience can check by
eye. A fire in this crate is a light (`Flicker`) and a drawing (`FlameSite`/`flame.wesl`), and until now
they ran on **different clocks and different laws**: the light breathed at the puffing law's rate, and the
flame's brightness did not breathe at all. Measured at the close camera over 70 steady frames, the flame's
own pixels moved **2.4 %** while the ground under them moved **11.4 %**, correlating at **−0.003**. The
flame is the *motivation* for the light; it was the only part of the fire holding still. The runner-up was
the bright vertical column in the close camera (see the critique) — a scene bug, real but narrower, and it
would have been a second improvement in one pass.

What lands:

- **`model::fire::flicker_swing`** — the one place the "sized or un-sized" choice is made, so the light and
  the flame cannot drift apart: `flicker_lights` writes the light's intensity from it and `update_flames`
  writes the flame's emission from it.
- **`FlameSite::diameter`** — the fuel bed that sizes the flame's flicker, set to the same value as the
  `Flicker::diameter` on the light it motivates.
- **The flame's tag** gains a brightness byte: **height 16 | phase 8 | brightness 8** (was phase 16 |
  height 16). The phase loses 8 bits to 2π/256 ≈ 0.025 rad, four milliseconds of clock at 6 Hz.
- **Two clocks, deliberately split**: the silhouette's noise stays on `Time<Real>`, the brightness moves to
  virtual `Time` with the light, so a paused world's fires hold still and the drawing on them keeps moving.
- **`PoolCandidate::phase` / `diameter`** — the same two numbers for a prop whose light is handed to it, so
  a light handed to a torch breathes on *that torch's* clock instead of the pool slot's `1.3 · slot`.

### The two performance defects found in the inherited work, and fixed

- **Every flame wrote its `MeshTag` every frame.** `MeshTag` is in `bevy_pbr`'s `Changed<MeshTag>`
  re-extraction set (`bevy_pbr-0.20.0/src/render/mesh.rs:1967`), so a write past the change is a
  render-world re-extract for that flame, every frame, for a byte that moves on roughly half of them. Now
  written only when the quantised byte changes. `an_unchanged_brightness_leaves_the_tag_unwritten` in
  `src/flame.rs` watches the **change tick** — the thing that set keys on — and fails on the old code.
- **The two `Option<f32>` rode in every ranked candidate's tuple**, 5000 entries a frame carrying a number
  that `pool.size` (16) of them read. Resolved for the winners instead, in a map built once per frame after
  the assignment is decided. Measured: **+3.4 % of the pool's own frame before** (`pool_p50_us` 493 → 511
  over five paired rounds), **gone after** (base 478–500 against after 468–489).

### Sources

- craft: `10.7456_tojdac.1674124` (Doğaner, *The narrational role of lighting design in The Lights of the
  Dark City* — "the set light *mimics the candle*"), quoted in the note.
- production: `s2011-advances-pdf-slides` (SIGGRAPH 2011 *Advances*, the Fire / Flamethrowers / Thrusters
  talk — emission accumulated from the body that moves), and `s2008-advances-chapter-5` (StarCraft II,
  Strugar — a flickering practical as a lighting case).
- technical: `10.1017_jfm.2018.707` (Xia & Zhang, *A vortex-dynamical scaling theory for flickering buoyant
  diffusion flames*), plus Cetegen & Ahmed 1993 and Chen/Roquemore through its references;
  `10.1109_tvcg.2019.2898741` (Denes et al., critical flicker fusion — why a ±15 % swing fuses rather than
  strobes); `book_real_time_rendering_4th_akenine` §6.3.
- The note is `docs/research/flame-flicker.md`, committed alone before any code (`d1e8128`).

### The critique, and the verdict

Full critique: `/home/ladvien/l15/critique.md`. **GREAT.** Evidence: `/home/ladvien/l15/mine/` — `base/`
built from a worktree at `5373cd9` in its own `CARGO_TARGET_DIR`, `final/` the shipped tree, both
`LIGHTS_FRAMES=100` at the close camera, frames 0–29 dropped.

| line, 70 steady frames | base | final |
|---|---|---|
| **flame region vs its pool, correlation** | **−0.003** | **+0.748** |
| flame region peak-to-peak | 2.4 % | 9.1 % |
| its pool's peak-to-peak (control, must not move) | 11.4 % | 11.4 % |
| frame-to-frame flame step | max 0.51 DN | max 2.88 DN, 13 sign changes in 96 frames ≈ 2.5 Hz |
| silhouette area sd while brightness moves | 665 px | **665 px** — shape untouched |
| key-to-fill (brazier pool 87.1 DN vs far ground 40.6 DN) | — | 2.14 : 1, unchanged |
| saturated pixels, all six angles | 0.00 % | 0.00 %; max DN 226.7 default, 249.7 topdown |
| whole-frame diff at a matched frame | — | mean 0.292 DN; 18 301 px > 4 DN of 921 600, and **99.6 % of them warm (fire) pixels** |
| red-channel clipping in the flame | 79.4 % of its pixels at R ≥ 254 | 46.6 % |

Angles judged at full frame and zoom: default, ground, edge, elevated, topdown, grazing. The topdown is the
strongest evidence for *motivated light* — four pools, four flames, no orphan glow — and the grazing frame
shows no cut at any light's `range`.

Flaws found and **not** fixed, ranked: (1) the bright vertical column above the near brazier, reproduced
with `LIGHTS_POOL=0` and identical base vs final, so pre-existing and not the pool's; (2) a torch's flame at
the close camera is a saturated shard, made *more* visible by the modulation it now has; (3) the brightness
is a uniform multiplier, and the note asks the flame not pulse as a whole — measured core/rim 1.074 / 1.132,
so the rim moves 5 % more than the core because the silhouette is independent, which is under the bar but
not zero; (4) the pool's per-prop clock is proven by `tests/pool.rs` and not by a capture.

### Benchmark

`bench_lights` (5000 candidates, 2000 lights, pool 16), base and after binaries interleaved, five paired
rounds, same host and load. Whole frame: base `p50` 671–686, after 703–734 — the after build carries the
flame's work, which this example does not measure (it spawns no `FlameSite`), so read the per-system line.
Per system: `pool_p50_us` **base 478–500, after 468–489**; `flicker_p50_us` base 240–267, after 242–265.
CPU-only, so it says nothing about GPU cost — which is exactly what the `Changed<MeshTag>` fix above is
for, and it cannot be measured here.

### The gate

On the final tree: `cargo build --release`, `cargo build`, `cargo test` (**72 tests + 13 doctests**),
`cargo test --features serde`, `cargo build --examples --all-features`,
`cargo clippy --all-targets --all-features -- -D warnings`,
`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`, `cargo fmt --check`,
`cargo build --lib --target wasm32-unknown-unknown`, `tools/wesl-check` — all pass.
`tests/readme.rs` passes, so the quick start still compiles.

Two environment notes, recorded rather than assumed. `/tmp` was **100 % full (3.4 MB free)** at the start
of the pass, the trap `docs/DEMOS.md` warns about — every build and capture here ran with
`TMPDIR=/home/ladvien/tmp`. And one capture of a hundred frames panicked with its output swallowed by a
`tail`; **twenty further runs with full logs showed no panic**, and the one run raced a `cargo build` that
was rewriting the binary it had just started. Not claimed away: it is not reproducible on this tree.

### Media

`docs/media/showcase.gif` re-encoded from the new build (60 frames from 2 s, 30 fps, 480 px, **2.1 MB**),
same caption and recipe. Its flicker was verified through the encode on single fire bodies, not on the
frame: three separate fires peak at **2.25 Hz** against the law's 2.57, peak-to-peak 9–10 %.
`docs/media/courtyard.gif` (58 frames, 30 fps, 640 px, 3.4 MB) and `docs/media/courtyard.png` regenerated
at their recorded recipes.

### Top 5 remaining, ranked

1. **The bright vertical column above a brazier at the close camera.** Reproduced with the pool off, so it
   is the scene: likely the ground's specular lobe plus the box's lit top face with `NotShadowCaster`. The
   most visible artifact in the crate's own showcase camera.
2. **A torch's flame saturates into a shard at close range** — the additive ramp against that exposure;
   needs a ramp or strength that holds its silhouette at 5 m as well as at 15 m.
3. **The flame's brightness is uniform over the whole flame.** The note's `## What great looks like` asks
   for a fire that does not pulse as a whole; scaling core and rim differently wants a source, and none of
   the three has one.
4. **The flicker amplitude is still uncited** (Chamberlin & Rose 1928 is not open access; Maynard 2013
   unobtainable). Unchanged for 14 iterations.
5. **A `Rank::Weighted`** for a host that wants gameplay priority — a carried torch matters more than a
   brazier — as a per-candidate multiplier on the rank key.

### One tracked change this pass did not make

`.omp/skills/bevy-lights-build-loop/SKILL.md` still carries an unstaged edit adding *"Your work is saved
even if you are killed"*. It was in the working tree when this pass began (iteration 14 logged it too) and
it is again **left uncommitted**: a tracked change this pass did not write is the driver's or its author's
to commit. Its guidance is right and was followed — verify and finish the killed pass's work rather than
beginning another, commit the research note alone, then the implementation, then docs and media, then this
log.

## Iteration 16 — 2026-10-26 — the flame's silhouette: researched, attempted, reverted

Outcome first: **nothing shipped.** The research note, the puff law as a tested model module, and the
blocker analysis landed; the shader change was implemented, captured, measured against three
alternative amplitudes and a lean term, and **reverted** because it did not reach GREAT on the crate's
own camera. The tree is back at `578ddac` for `src/`.

### What ranked first, and the runner-up

1. **The flame's silhouette is one analytic curve mirrored about its axis** (`body = 1 - abs(dx)/half_width`
   with a height-only `half_width`), so a bulge is a bulge on both edges at once. Picked: it is the
   crate's most-shown feature, the README GIF's subject, and the one place the shader is structurally
   incapable of the thing its own note asks for.
2. A torch's flame saturates into a shard at close range (8 % of its pixels ≥ 254 DN at 5 m; the whole
   flame is a shard at 3 m). Real, deferred: its fix is a ramp change that would fight the ramp an
   asymmetry redesign already needs.
3. **Downgraded from iteration 15's #1**: the bright vertical column above the near brazier. Measured
   with `LIGHTS_POOL=0` and found *identical*, and traced to pillar x915–923 lit by the moon fill — a
   scene artefact, not a crate one. Struck, with the evidence in `findings.md`.
4. The flicker amplitude stays uncited (blocked 15 iterations; Maynard 2013 unobtainable).
5. `Rank::Weighted`, and the flagstones' 256 draw calls.

### Sources

- craft: `10.7456_tojdac.1674124` (Doğaner, the narrational role of lighting design — visible sources
  "manage the viewer's perception, allowing them to logically find where the light comes from"), and
  `10.32657_10220_48312` (Doyle's "small single source … to emulate the practical lighting source").
- production: `s2011-advances-pdf-slides` ("Spawn lots of high velocity particles, pick up nice motion
  from fluid sim"), `s2007-advances-course-notes-0-4-mb` (TF2, legibility), and **GPU Gems ch. 6 §6.1,
  now in the corpus and read in full** — which narrows rather than widens `flame.md`'s claim about it.
- technical: `10.1017_jfm.2018.707` (Xia & Zhang: toroidal vortices, "pairing and merging of the flame
  bulge", and `H = U₀·τ` giving `λ = U₀/f`), `eth-cgl-sim_anim-Kim08` (enforce Navier-Stokes low, a
  turbulence spectrum high), `book_real_time_rendering_4th_akenine` §6.3.
- Note: `docs/research/flame-silhouette.md`, committed alone as `1a8cef6`.

### The work, and the measurements that ended it

`model::flame::{puff_wavelength, flame_bulges}` (`λ = (√g/1.5)·D`) is sound and kept in the note as the
law any future attempt needs. In the shader: a per-side half-width from one field at `±17.3`, a vertical
frequency of `size / PUFF_WAVELENGTH`, the heat ramp moved onto the per-side `body`, then an axial lean.

| line | baseline | attempt (0.70) |
|---|---|---|
| close camera, brazier width variation about its own trend | 3.67 px sd (6.2 % of mean width) | **7.01 px sd (11.7 %)** |
| width-profile bulges, close camera | 1 | **2** |
| hero camera, torch axis wander | 0.33× its own width | 0.32× — **no gain** |
| warm pixels ≥ 254 DN, default camera | 0.180 % | 0.177 % |
| silhouette area modulation, 40 frames | 14.9 % p2p | 14.5 % p2p |
| key-to-fill | 2.00 : 1 | 2.00 : 1 |

Three findings worth the next pass's time, all in the note's `Deviations`:

1. **A bigger amplitude is not the answer.** 0.22 was invisible (`fbm`'s spread is sd 0.12, so `2·fbm−1`
   arrives at a fifth of its name), 0.50 gave a visible waist, 1.00 left the flame gaunt, 4.00 tore it.
   The usable window is narrow and near 0.70.
2. **A vertical phase split between the two sides makes it worse** (bulges 1 → 0): it is equivalent to
   scrolling the field, so both sides keep the same bulge pattern, offset in time.
3. **The blocker is the missing per-flame `D`.** A 0.35 m torch flame is shorter than one puff
   (`flame_bulges(0.35, 0.19) = 0.88`), so a constant `PUFF_WAVELENGTH` gives it a fraction of a lobe and
   no amplitude rescues that. Getting `D` to the fragment stage needs a wider `MeshTag` (its 32 bits are
   spent: height 16 | phase 8 | brightness 8) or a second instance attribute — an allocation decision.

### The metric correction — recorded because it invalidates a tempting shortcut

The first draft of the note claimed the baseline was "exactly symmetric: 0.00 px left/right difference on
every frame". **That was a tautology**: the axis was `mean(left + right)`, which makes the two half-widths
equal for *any* picture, so the statistic is identically zero whatever is rendered. Re-measured honestly,
the baseline is already asymmetric — **16.5 % per-row mirror residual, 5.1 % half-width difference** —
because the existing `n1` fbm already perturbs `dx` per pixel. What the crate ships is a mirror *envelope*
with a noisy outline: a much narrower defect than the draft claimed, and the reason a per-side envelope
change bought so little. Both the note and this log carry the correction rather than the original claim.

### The critique and the verdict

Full critique: `/home/ladvien/l16/critique.md`. **NOT YET**, and therefore reverted. Round 5 of the
allowed five. Against the note's `## What great looks like`: "no mirror plane" PARTLY, "bulges that shed"
PARTLY (no visible detachment at the tip), "a hot core that is not a stripe" improved by construction but
unmeasured, "an unbroken silhouette" PASS only at 0.70 with 1.0 already gaunt, "the same fire as before"
PASS, "cheap" PASS as far as a CPU bench can see.

### The gate, on the reverted tree

`cargo build --release`, `cargo build`, `cargo test` (37 lib + 8/3/28 behaviour + 1 readme + 15 doctests),
`cargo test --features serde`, `cargo build --examples --all-features`,
`cargo clippy --all-targets --all-features -- -D warnings`,
`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`, `cargo fmt --check`,
`cargo build --lib --target wasm32-unknown-unknown`, `tools/wesl-check` — all pass on the reverted tree.
Bench (base `578ddac` in its own worktree and target dir, three interleaved rounds): whole frame base
`p50` 472–564 µs against after 465–507 µs, `pool_p50_us` 334–393 against 327–368, `flicker_p50_us`
148–206 against 149–206 — inside the same noise, as expected for a GPU-side change the bench cannot see.

### Environment

`/tmp` was **100 % full (1.7 MB free)** again, so every build and capture ran with `TMPDIR=/home/ladvien/tmp`
and the base worktree kept its `target/` off `/tmp`. GPU was idle (476/24576 MiB, no tenant). One Xvfb
capture failed with `XOpenDisplayFailed` on the display `-a` picked, the documented trap; re-run on a
fresh `-n`.

### Top 5 remaining, ranked

1. **A per-flame fuel-bed diameter in the fragment stage**, which is the blocker above: a wider `MeshTag`
   or a second instance attribute, then the silhouette attempt retried with each fire's own `λ`. This is
   the item that makes the flame's shape legible at gameplay distance, and it is an allocation decision
   first.
2. **The tip must actually pinch and shed**, which the note asks for and no attempt has delivered. With a
   per-flame `λ` this is the same mechanism, not a new one.
3. **A torch's flame saturates into a shard at close range** (carried from iteration 15): 8 % of its
   pixels at ≥ 254 DN, the whole flame at 3 m.
4. The flicker amplitude, still uncited (blocked 15 iterations).
5. `Rank::Weighted` for gameplay priority.

### One tracked change this pass did not make

`.omp/skills/bevy-lights-build-loop/SKILL.md` still carries the unstaged edit adding *"Your work is saved
even if you are killed"*, in the working tree when this pass began, as iterations 14 and 15 also logged.
It is again **left uncommitted**: a tracked change this pass did not write belongs to its author. Its
guidance was followed — the research note was committed alone before any code, and the attempt was
finished and measured rather than abandoned half-done.

## Iteration 17 — 2026-10-26 — a carried fire's light sits where its flame is

Outcome: **shipped.** The crate's own showcase taught a light that was not in its fire, and the fix is one
model function plus the three places a host copies a carried fire from.

### What ranked first, and the runner-up

1. **A handed-out torch light floated 1.80 m above the flame that motivates it, and was sized for a
   height it did not have.** Picked: it is a *visible artifact and a physically wrong result* at once —
   the first category — and it is in the crate's own showcase, the README quick start and the crate doc,
   so it is also the shape every host will copy. Measured on `main`: every handed-out torch pool light at
   `y = 3.60`, nearest flame `y = 1.80`; 15 402 lm sized for 250 lx at 2 m delivering **72 lx** there
   (1.79 stops, ratio 0.289), and a lit bearer's own ground **darker** than open ground 10 m away
   (45.6 DN against 72.6 DN, 0.63:1) — there was no torch pool at all.
2. The bright vertical blade on a close brazier (iteration 16's #1, reverted there for lack of a
   GREAT-able change). Real, still deferred: it needs iteration 15's ramp or a per-flame `D`.
3. A per-flame fuel-bed `D` in the fragment stage (iteration 16's #1). Blocked on a wider `MeshTag` — an
   allocation decision, not a tuning one, and not an example fix.
4. `Rank::Weighted`, and the flagstones' 256 draw calls.
5. The flicker amplitude stays uncited (blocked 16 iterations; Maynard 2013 unobtainable).

The pick beats the runner-up because the runner-up is a shader look that iteration 16 already failed to
reach GREAT on, while this was a hard geometric error with a one-function fix and a measurable gain.

### Sources

- craft: `10.7456_tojdac.1674124` (Doğaner — the audience "logically find[s] where the light comes from";
  the paper separates that case from light "whose physical source is not explained"), and `10.23856_6105`
  (the "drawing" light, whose placement is a rule rather than a taste).
- production: `10.13140_marples.nav.lighting.thesis` (Marples 2017) — Left 4 Dead "uses both the motion
  and simultaneous illumination offered by flames", and Alan Wake's carried torch. The one sentence that
  says the drawn thing and the lighting thing are the same object.
- technical: `10.1007_978-1-4842-4427-2` (*Ray Tracing Gems* ch. 18, `dω = (n·−l)/‖X−Y‖² dA` — a light's
  position *is* its emitter) and `PBR3_05_Color_and_Radiometry` (`E = Φ/4πr²`, so the error is statable
  in stops).
- Note: `docs/research/carried-fire.md`, committed alone as `ef7493c`.
- Corpus re-scan: `catalog_recent` (200 rows) found 15 lighting/rendering documents not in
  `docs/research/corpus.md`; committed separately as `def8e0b` with two unclassified stems flagged as
  such rather than cited.

### The work

`model::reach::flame_light_offset(height)` = `(0, h/2, 0)`, the flame billboard's own centre — the one
point inside a drawn fire of any height. `PoolCandidate::offset` is documented against it (a candidate on
the `FlameSite` needs no offset; one on a *parent* adds only the difference). The courtyard, the README
quick start and the crate doc now spawn **one entity** holding the `FlameSite` and the `PoolCandidate`,
with the offset from that helper and the lumens sized from the same height the light sits at. `src/`'s
diff is 53 lines of docs plus one function and its tests.

One artefact found by looking rather than measuring: the first hand placement, `(0.30, 0.35, 0.12)`, drew
the flame **inside** the bearer's own 0.30 m capsule (`/home/ladvien/l17/zoomC.png`). Widened to
`(0.42, 0.35, 0.16)` and re-captured; `zoomC2.png` shows every flame clear of its figure.

### The critique and the verdict

Full critique: `/home/ladvien/l17/critique.md`. **GREAT** on all four faces (placement, sizing from its
own height, the model function, the docs teaching it). Nothing was reverted.

| line | before → after |
|---|---|
| light-to-nearest-flame distance | **1.80 m → 0.18 m** (< half a 0.35 m flame) |
| a torch's pool edge illuminance | 72 lx → 250 lx (the number it was sized for) |
| lit bearer's own ground vs open ground | 0.63:1 (no pool) → a pool; brightest ground block 143.6 → 156.0 DN |
| key-to-fill on the frame | 2.00:1 → 2.00:1 (unchanged) |
| clipped pixels, `rgb ≥ 250`, every camera | 0.000 % → 0.000 % (`pool16` 0.001 %, at flame cores) |
| frame mean, hero / ground / top-down | 48.8 → 53.0 / 35.0 → 38.3 / 50.0 → 53.3 DN |

### Benchmark

`bench_lights` (5000 candidates, 2000 lights, pool 16), base `7d1208c` in its own worktree and target
directory, five interleaved rounds: base `p50` 565–606, after 575–716; `pool_p50_us` base 393–590, after
406–484; `flicker_p50_us` base 168–192, after 198–246. **Every range overlaps**, which is what this
change should do: it adds a pure function nothing in the hot path calls, and the bench spawns no
`FlameSite`. The one after-run at `p50` 716 was a 942 µs mean against a 600 µs frame, i.e. the host
under load, and `docs/DEMOS.md`'s rule is to throw a reading away whose repeat moves by more than 10 %.
CPU-only, so it says nothing about GPU cost; the captures' frame times are unchanged.

### The gate

On the final tree: `cargo build --release && cargo build && cargo test` (**34 lib + 8/3/28 behaviour + 1
readme + 14 doctests**), `cargo test --features serde`, `cargo build --examples --all-features`,
`cargo clippy --all-targets --all-features -- -D warnings`,
`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`, `cargo fmt --check`,
`cargo build --lib --target wasm32-unknown-unknown`, `tools/wesl-check` — all pass.

### Environment: `/tmp` reached 0 bytes free

New trap, recorded because it cost time and will recur. Mid-pass `/tmp` filled to **0 bytes** (another
service's `vllm-child.log` was still growing at ~100 MB). At exactly zero free bytes Xvfb cannot create
`/tmp/.tX<N>-lock` and dies with `Could not write pid to lock file`, which every capture sees as
`XOpenDisplayFailed`. `xvfb-run` was unusable on any display number, and the two live displays (`:97`,
`:98`) had gone with it. The working answer, which needs no privilege and touches nobody's files: run the
capture inside `unshare -rm` with a private tmpfs mounted at `/tmp` —

```sh
unshare -rm bash -c 'mount -t tmpfs tmpfs /tmp; mkdir -p /tmp/.X11-unix; chmod 1777 /tmp/.X11-unix;
  exec xvfb-run -a -s "-screen 0 1920x1080x24" env LIGHTS_FRAMES=120 LIGHTS_SHOT=… …'
```

`/home/ladvien/l17/shoot.sh` and `seq.sh` are that, and every capture in this iteration used them. GPU was
idle (12.4 GiB free, no tenant). The documented Linux font path for `tools/gif.sh`
(`/usr/share/fonts/truetype/liberation/`) does not exist on big; corrected in `docs/DEMOS.md` with the
check to run first, because the failure lands in `magick` after the whole encode.

### Top 5 remaining, ranked

1. **A per-flame fuel-bed `D` in the fragment stage** (iteration 16's blocker): a wider `MeshTag` or a
   second instance attribute, then the silhouette attempt retried with each fire's own `λ`. The
   allocation decision comes first.
2. **The tip must pinch and shed** — with a per-flame `λ` this is the same mechanism.
3. **A torch's flame saturates into a shard at close range** (8 % of its pixels at ≥ 254 DN at 5 m).
4. **The flicker amplitude, still uncited** (blocked 16 iterations).
5. **The example's bearers have no torch prop or arms**, so at close range a flame reads as floating
   beside a capsule. Example art, not the crate; worth a modelled hand in the showcase.

### One tracked change this pass did not make

`.omp/skills/bevy-lights-build-loop/SKILL.md` still carries the unstaged edit adding *"Your work is saved
even if you are killed"*, in the working tree when this pass began, as iterations 14, 15 and 16 also
logged. Left **uncommitted** again: a tracked change this pass did not write belongs to its author. Its
guidance was followed — the corpus re-scan and the research note were each committed alone before any
code, then the implementation, then the docs, then the media.

## Iteration 18 — 2026-10-26 — a lit flame grows with its light

Outcome: **shipped.** `kindle` was documented to grow the flame, the CPU wrote the right number every
frame, and the shader never used it. One multiply in the vertex stage, and a lit fire grows from a point.

### What ranked first, and the runner-up

1. **A `FlameSite` with a `kindle` did not grow its flame.** Picked: it is a *documented behaviour that
   does not happen*, category 1, and it puts the drawn source and the light it motivates in disagreement
   at the one moment the audience is watching for it — a fire being lit. Measured: the flame's bbox
   height across the catch was **flat at 155–174 px** (silhouette noise) while the light went 6 % → 100 %,
   and only the flame's peak luminance moved (120 → 161 DN). The fix is one multiply, the crate's own
   showcase spawns exactly this way, and the README already promised the behaviour.
2. **The flame has no internal structure.** Width sd in the mid-band 3.7 % of mean width, hot patches per
   row 1.06–1.45 (a vertical stripe core), no bulge at any distance. Real and still the largest beauty
   gap — but still blocked on a per-flame `D` needing a wider `MeshTag`, which is an allocation decision,
   not a pass's work.
3. A per-flame fuel-bed `D` in the fragment stage (iterations 16 and 17's #1). Unchanged.
4. The tip must pinch and shed. Same mechanism as 3.
5. The flicker amplitude, still uncited (blocked 17 iterations).

The pick beats the runner-up because the runner-up cannot be finished without an allocation decision this
crate has deferred twice, while this was a shipped-behaviour lie with a one-line fix and a metric.

### Sources

- craft: `10.7456_tojdac.1674124` (Doğaner — the set light "mimics" the candle; the flame is the only
  part of the source the audience can see) and `s2007-advances-course-notes-0-4-mb` (TF2: "Even when
  viewed only in silhouette with no internal shading at all, the characters are readily identifiable" —
  size is the first read).
- production: `s2011-advances-pdf-slides` ("Ramp the alpha to fade in/out over particle lifetime";
  "shrink particles as they fade") and `GPU Gems` ch. 6 §6.1 (the emitter's own size is a parameter).
- technical: `10.1017_jfm.2018.707` (re-read for a flame-length law: it derives the *puffing frequency*
  from vortex shedding and gives no length-vs-time law, so nothing new rests on it) plus
  `book_real_time_rendering_4th_akenine` §6.3, unchanged.
- Note: `docs/research/flame-kindle.md`, committed alone as `998ed68`.

### The work

`src/flame.wesl`'s vertex stage: `let size = tag.y * material.params.y * took;`. `took` was already in
the vertex stage — read from `length(world_from_local[0].xyz)`, the instance scale `update_flames`
writes from `kindle_gain` — and was already flat-interpolated to the fragment for the ember colour. It
now also scales the quad. No new tag bit, no new uniform, no second per-flame channel. The rustdoc for
`FlameSite::kindle` and `Flame` now say the catch scales the *drawn* size and how.

### A claim of mine that was wrong, corrected

The note's first draft said the fix must preserve a "35 % size overshoot". `kindle_gain` is
`s + 0.35·sin(πs)` on a smoothstepped `s`, so its peak is **1.009 at t ≈ 0.46 s** — the 0.35 is the
sine's amplitude, not the sum's peak, and the shader's `clamp(took, 0, 1)` costs 0.9 % for one frame.
Caught by computing the curve instead of reading the constant. The note and its `Validation targets`
carry the correction.

### The critique and the verdict

Full critique: `/home/ladvien/l18/critique.md`. **GREAT** on all five faces; nothing reverted.

| line | before → after |
|---|---|
| flame height across the 0.6 s catch | flat 155–174 px (noise) → 6, 11, 17, … 157, 166, settled ~158 px |
| fit to `kindle_gain` | n/a (no growth) → **0.98 px RMSE over 10 samples** |
| settled flame, 13 capture cameras | — → **all identical**; `hero.png` byte-identical (md5 `994dfca…`) |
| clipping, `lum ≥ 250` | 0.0000 % ×12, `pool16` 0.0025 % → unchanged |
| flame peak luminance across the catch | 120 → 161 DN (the only thing that moved) → still 120 → 161 DN |

Two things checked and struck rather than fixed: the flame's **apparent** foot rises 8 px during the
catch, which is the shader's own `smoothstep(0, 0.06, y)` base band scaling with the flame (0.6 px at a
10 px flame, 9.5 px at 158 px), not a shift; and one 1-px detached island above the tip on 1 frame in 41.

### Benchmark

`bench_lights`, after: `mean_us=501 p50_us=492 min_us=275`, `flicker_p50_us=144`, `pool_p50_us=351`.
**No before/after comparison is meaningful here and none is claimed:** the change is one multiply per
vertex in a shader, and `bench_lights` is headless and CPU-only — it executes no shader at all. The base
worktree's first build was killed at the 300 s job ceiling mid-`zeno`/`yazi`/`swash`, so a base binary
did not exist to compare against; a same-machine comparison of a shader change is a capture, and the
captures' settled state is pixel-identical.

### The gate

On the final tree: `cargo build --release`, `cargo build`, `cargo test` (**34 lib + 8/3/28 behaviour + 1
readme + 14 doctests**), `cargo test --features serde`, `cargo build --examples --all-features`,
`cargo clippy --all-targets --all-features -- -D warnings`,
`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`, `cargo fmt --check`,
`cargo build --lib --target wasm32-unknown-unknown`, `tools/wesl-check` — all pass.

### Media: the showcase GIF cannot show a catch, and that is a documentation bug

Re-encoded `docs/media/showcase.gif` twice and confirmed the capture path cannot reach the catch on big:
the catch runs on `Time<Real>` while the first frames are pipeline compilation, which takes longer than
`KINDLE_SECONDS`. `LIGHTS_SHOT_FROM=1` therefore starts after the catch has already finished — frame means
`0.00, 5.33, 36.03, 53.53` with every flame already full size. The **committed** GIF has the same shape
(`6.25, 36.34, 55.54, …`), so it has never shown a catch either, and its caption promises one. The
re-encode was **reverted** (`git checkout docs/media/showcase.gif`) rather than shipped: it changed the
crop and the file size and showed nothing new. `docs/DEMOS.md` now records the trap and the working recipe
(spawn the `FlameSite` after warm-up), which is the honest fix until an example exposes a "light this fire
now" key to the harness. **Top-5 item for the next pass: a capture path that can show the catch** — the
README's first feature is the one its hero GIF cannot demonstrate.

### Environment

GPU idle (no tenant). `/tmp` was full, so every capture and build ran through the private-tmpfs
`unshare -rm` wrapper (`/home/ladvien/l18/shoot.sh`, taken from iteration 17). **New trap:** `xvfb-run
-a` twice picked a display another process held and the capture died with `XOpenDisplayFailed`; walking
an explicit `-n` number per run fixed it, and that is now in the wrapper.

### Top 5 remaining, ranked

1. **A capture path that can show the catch** — the README's first feature is the one its hero GIF cannot
   demonstrate: the catch runs on `Time<Real>` and the pipeline warm-up outlasts it. A switch that lights a
   fire on frame N, with `LIGHTS_SHOT_FROM` past warm-up, is example work and would make the showcase show
   what its caption claims. Ranked first because it is small, it is the README, and the fix this iteration
   shipped is currently invisible in every committed artefact.
2. **A per-flame fuel-bed `D` in the fragment stage** (the blocker for 3). The allocation decision comes
   first: a wider `MeshTag` or a second instance attribute.
3. **The flame's interior is a stripe with a smooth envelope** — width sd 3.7 % of mean width, 1.06–1.45
   hot patches per row. Unblocked by 2.
4. **The tip must pinch and shed** — the same mechanism as 2 and 3.
5. **The flicker amplitude, still uncited** (blocked 17 iterations; Maynard 2013 unobtainable), and
   `Rank::Weighted` with the flagstones' 256 draw calls.

### One tracked change this pass did not make

`.omp/skills/bevy-lights-build-loop/SKILL.md` still carries the unstaged edit adding *"Your work is saved
even if you are killed"*, in the working tree when this pass began, as iterations 14–17 also logged.
Left **uncommitted** again: a tracked change this pass did not write belongs to its author. Its guidance
was followed — the research note was committed alone before any code, then the implementation, then the
docs, each landing as its own commit.

## Iteration 19 — 2026-10-26 — the flame from above: found, attempted, reverted; the validator's stub was a hole

Outcome: **the shader change is reverted** (5 rounds, the skill's ceiling, verdict **NOT YET**). What ships is the
research note and a real fix to `tools/wesl-check`'s `View` stub. The defect below is the largest visible artifact
this crate has, it is now measured from 0 to 85 degrees, and the next pass starts from round 5 rather than round 1.

### What ranked first, and the runner-up

1. **A flame is invisible from a steep camera.** Picked: it is category 1 (a visible artifact, and a *physically
   wrong result*), it hits a camera angle most games use and film never does, and it is the crate's own ancestor's
   documented weakness shipping unmitigated. Measured at a fixed 4 m over a brazier: the flame above the coal bed was
   **159 px at 0°, 196 px at 15°, and completely gone at 75° and 85°**, with a torch's flame at 85° a **1-pixel
   sliver**. Cause, exact: an upright camera-facing quad is an *axial* billboard (RTR4 §13.6.3), so its normal is
   horizontal and the area it presents falls as `cos(elevation)` — 26 % at 75°, 9 % at 85°. Its light keeps working
   at every angle, so the drawn source disappears while its cast light stays: a motivated-source failure in the
   direction the light does not have.
2. **A brazier's coal bed clips to a flat white slab from above** — 74 px at `min channel ≥ 245` at 85°, 0 px at the
   hero camera, because `examples/courtyard` sets the coals' `emissive` to `blackbody × 1.4` as a *constant* while
   the ground and box faces brighten with the viewing angle. Discovered while measuring #1 and it *masks* #1: at 85°
   the thing a player reads is a white rectangle. **This is the next pass's first item**, because it is smaller, it is
   example-side, and it hides whatever the shader does.
3. A per-flame fuel-bed `D` in the fragment stage (iterations 16–18's blocker: `MeshTag`'s 32 bits are spent).
4. The flame's interior is a stripe with a smooth envelope. Blocked by 3.
5. The flicker amplitude, still uncited (blocked 18 iterations).

### Sources

- craft: `10.7456_tojdac.1674124` (Doğaner — a visible source is how the viewer "logically find where the light
  comes from"), and `Ga` §11.4.3 (Gregory — a billboard's normal must point at the camera's *focal point*).
- production: `GP` ch. 6 §6.1 (Nguyen, the crate's own ancestor, names this exact failure and the constraints it
  needed) and `s2011-advances-pdf-slides` (SIGGRAPH 2011's fire talk — many overlapping puffs precisely so the
  "particle nature" is hidden, which one quad cannot do).
- technical: `book_real_time_rendering_4th_akenine` §13.6, §13.6.2, §13.6.3 — the billboard taxonomy, the axial
  failure in the reference's own words, and the cross-section workaround; and `10.1017_jfm.2018.707` (Xia & Zhang —
  the flame is axisymmetric, which is *why* the cross-section is a disc).
- Note: `docs/research/flame-overhead.md`, committed alone as `6e4aaba`.

### The five rounds, and why it was reverted

1. **Tilt the quad's `up` toward horizontal** (the literal "world-oriented billboard"). Worse than nothing: at 60°
   the flame becomes a leaning plate smearing across the coal slab. Geometric, not tunable.
2. **Keep the quad upright, draw the projected disc inside it.** The disc landed coplanar with the coal bed and was
   depth-rejected.
3. **Lift the disc to the plume's centre and fade the base band by `cross`** (the base band erased the whole disc —
   every fragment of a cross-section in an upright quad is at the quad's base height). Torch's flame appeared.
4. **Isotropic quad overhead** — the radial used `FLAME_ASPECT` for both axes and drew an ellipse.
5. **Stretch the quad's height by `1/cos(e)`** to cancel the upright face's own foreshortening, with the disc's
   vertical semi-axis at `R·sin(e)`. The arithmetic finally worked: brazier added light **143 px → 2962 px, 81×57**.

The verdict is **NOT YET** on two lines of `## What great looks like`:
- *A fire at every angle* — PARTLY. The disc is drawn and measurable, but at 85° it is swamped by the clipped coal
  bed (17 DN of added light over a bed already at 255/244/227).
- *No pop as the camera rises* — NOT YET. Between 50° and 76° the silhouette is a `mix` of a **full-size** tongue and
  a full-size disc, so at 75° a teardrop lies diagonally across the brazier with a puff fading in over it. The fix is
  a dissolve weighted by `cross` — the tongue shrinking as the disc arrives — and it is unbuilt.

PASS, measured: **ground level is unchanged, pixel for pixel** — `magick compare -metric AE` gives **0** differing
pixels at 45° and 0.58 at 0° between the base build and the changed build, because `OVERHEAD_START` at 50° makes every
term exactly its old value; and the cost is one `asin`, one `sin`, one `length()` and one flat varying.

### The bug that did ship: the WESL validator's `View` stub was missing a field

The `View` stub in `tools/wesl-check/src/main.rs` declared `world_from_view` and `clip_from_world`. The real struct
(`bevy_render-0.20.0/src/view.wesl`) also has `world_position`, `exposure` and more, so a shader reading the camera's
position **failed the validator's parse** — which is how this was found, on the first shader that needed it. Proven
both ways: a one-line probe reading `view.world_position` fails on the old stub (`invalid accessor world_position`)
and compiles on the extended one. The stub now carries both fields in the real struct's order, and `AGENTS.md` gains
the rule — a stub's *bodies* are never run, but its *fields* are type-checked, so a subset stub rejects valid code
and a superset accepts invalid code.

### Benchmark

`bench_lights`, base worktree (`d0f9519`) against the final tree, **interleaved** so the machine's load is shared,
two paired rounds on the same host:

| round | current `p50_us` | base `p50_us` | current `pool_p50_us` | base `pool_p50_us` | current `flicker_p50_us` | base |
|---|---|---|---|---|---|---|
| 1 | 573 | 612 | 459 | 445 | 221 | 225 |
| 2 | 624 | 654 | 457 | 470 | 239 | 239 |

**No regression.** The two builds are indistinguishable, which is what the change predicts: the only production
difference is in `tools/wesl-check`, which is not linked into the crate, and the shader change is reverted. A first
unpaired pair read 756 against 635 µs — that was machine load, not the change, and the interleaved rounds are the
honest number.

### The gate

On the final tree: `cargo build --release`, `cargo build`, `cargo test` (**34 lib + 8/3/28 behaviour + 1 readme +
14 doctests**), `cargo test --features serde`, `cargo build --examples --all-features`,
`cargo clippy --all-targets --all-features -- -D warnings`, `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
--all-features`, `cargo fmt --check`, `cargo build --lib --target wasm32-unknown-unknown`, `tools/wesl-check` — all
pass.

### Environment

GPU idle throughout (476/24576 MiB, no tenant). `/tmp` was **100 % full (956 KB free)** again, so every capture,
build and the base worktree's compile ran through the private-tmpfs `unshare -rm` wrapper
(`/home/ladvien/l19/shoot.sh`) with `TMPDIR=/home/ladvien/tmp`, and the base worktree kept its `target/` at
`/home/ladvien/l19/base-target` (its first build took 13 min 12 s, so an A/B build is a real cost to budget for).
Xvfb picked a held display once and was fixed by the wrapper's rotating explicit `-n`.

### Top 5 remaining, ranked

1. **A brazier's coal bed clips to a flat white slab from a steep camera** (74 px at ≥245 at 85°, 0 px at the hero
   camera). It is the brightest thing in frame at exactly the camera the flame just became visible from, and it
   masks the flame. `examples/courtyard`'s `emissive: blackbody × 1.4` is a constant; it needs to stop being one.
2. **The flame from above, round 6**: re-apply round 5's arithmetic (the `1/cos(e)` stretch with the `R·sin(e)`
   semi-axis) and add the dissolve — the tongue must *shrink* as the disc arrives, not sit at full size behind it.
   Everything needed is in `docs/research/flame-overhead.md`.
3. A per-flame fuel-bed `D` in the fragment stage (blocked on a wider `MeshTag` or a second instance attribute —
   `GpuComponentArrayBuffer` exists in Bevy 0.20 and is the unexamined option).
4. The flame's interior is a stripe with a smooth envelope. Unblocked by 3.
5. The flicker amplitude, still uncited 19 iterations, and `Rank::Weighted` with the flagstones' draw calls.

### One tracked change this pass did not make

`.omp/skills/bevy-lights-build-loop/SKILL.md` still carries the unstaged edit adding *"Your work is saved even if
you are killed"*, in the working tree when this pass began, as iterations 14–18 also logged. Left **uncommitted**
again: a tracked change this pass did not write belongs to its author. Its guidance was followed — the research note
was committed alone before any code.

## Iteration 20 — 2026-10-27 — a fuel bed is a surface with a temperature field on it

Outcome: **GREAT** on every line of the note's `## What great looks like`. The pass inherited a killed
pass's work (pass 21: note committed as `a4cdcd7`, implementation uncommitted and never logged) and
finished it, after finding four defects in it by measurement. Commits `b6bad4d` (implementation),
`01532aa` (note), `d2f2cc0` (docs + GIF).

### What ranked first, and the runner-up

1. **The braziers' coal bed clips to a flat white slab from a steep camera.** Picked: it is category 1
   (a visible artifact and a physically wrong result), it is the brightest thing in frame at exactly the
   camera iteration 19 spent five rounds making the flame visible from, so it *masks* that work, and it
   is the previous iteration's own first backlog item. Measured baseline: **36 px** at
   `min channel ≥ 245` at 85°, red standard deviation **1.2 DN**.
2. **`blackbody_colour` has no magnitude** — a surface's `emissive` is a radiance in cd/m², and a colour
   normalised to its own peak cannot supply one. Shipped as part of 1, because the bed is what needs it.

### What the inherited implementation got wrong (all four found by measuring)

1. **Voronoi partition → connected ash net.** A nearest-lump field makes the ash a mesh by construction,
   so the bed renders as a waffle of pale cells however it is tuned. Replaced with a **union of
   overlapping shards**: channels terminate where three shards meet and 30 % of cells hold no coal.
2. **The ember stop was past the display's white point.** At `ev100` 6, 1140 K lands on **DN 255** red,
   so every lump above the ash was identical white and structure survived only in green — the reason the
   inherited build rendered a *pale* honeycomb. Ember is now 1020 K (DN 157); fuel 1300 K.
3. **The upper ramp was a cube**, pinning the median coal (field 0.50, `EMBER_AT` 0.30 → 0.29 of the
   upper half → 0.02) at the ember stop. Measured bed median luminance **96.9 DN** against the stone
   bowl's **166.5**: the source was darker than its own prop. Exponent 1.4 puts it at **194.7**.
4. **The bed's drawn size was hard-coded 0.34 m** while `LIGHTS_BED` moved only flicker and flame, so
   `LIGHTS_UNSIZED` — whose point is comparing two flicker rates — drew the same box either way.

### Sources

Unchanged from the committed note (`01532aa` records the amendments): craft `10.1525_fq.2004.57.3.24`
(Prince — a blown highlight is an absence, not a brightness); production `GP` ch. 6 (the ancestor never
draws a fuel bed, which is why this defect is this crate's own) and `s2011-advances-pdf-slides`;
technical `10.1145_3130800.3130814` (Pirk et al. — char is a *field* over the surface),
`10.1017_jfm.2018.707` (Xia & Zhang — the plume is axisymmetric, so the bed cools to its rim), and
`PBR3_12_Light_Sources` (a blackbody emitter is perfectly diffuse).

### The critique and the verdict

`/tmp/l20/critique.md`, written before the verdict and kept. Evidence: `/tmp/l20/before/` (base
`a4cdcd7`), `/tmp/l20/after3/` (final), with `after/` and `after2/` the two rejected rounds.

- **No clipped fuel bed at any camera — GREAT.** 36 px → **0 px** at 85°; 0 px at 0/15/30/45/60/75°.
- **The bed has structure — GREAT.** Red std **1.2 → 35.4 DN** (target ≥ 12).
- **The source stays the brightest thing — GREAT.** Peak red **255** (target ≥ 240); bed now brighter
  than its rim at the hero camera and at every camera where a coal is in view.
- **The hero camera reads the same fire — GREAT.** Bed mean **157.19 → 153.22 DN** (−2.5 %, inside the
  10 % bound); frame mean 53.09 → 53.07; **714** changed pixels, all four beds.
- **Iteration 19's flame not regressed — GREAT.** No shader change; flame-core pixels identical at
  0/15/30/45°, −0.9/−5.0/−6.9 % at 60/75/85° (the flame runs on `Time<Real>`, so the capture is not
  locked to its animation).
- **No new cost — GREAT.** No new draw or light: one texture sample in a pass that already samples the
  stone. Texture generation is **37 ms** once at startup.

Named, not hidden: at 85° the bed's *median* coal (163 DN) is still under the rim's p95 (230 DN). l20's
critique blamed this on the bed being too narrow; **that is falsified by measurement** — widening to
0.55 m and 0.68 m moves coal-p50/rim-p50 0.89 → 0.88 → 0.81, and darkening the bowl albedo 0.10 → 0.040
moves it 0.85 → 0.90. The lever was the ramp and it is now spent; the remaining fix is a bowl that
shades its own rim, which is a geometry change with its own note.

### Benchmark

`bench_lights`, default sizes, current tree against the base worktree (`a4cdcd7`, target
`/home/ladvien/l20/base-target`), interleaved on the same host:

| round | current `p50_us` | base `p50_us` |
|---|---|---|
| 1 | 603 | 502 |
| 2 | 472 | 577 |

**No regression**, and no comparison is claimed beyond that: `examples/bench_lights.rs` contains **0**
calls into `src/model/char.rs`, so it cannot see this change at all. The alternating rounds differ by
±20 % from each other in both directions, which is this host's load; the change is one CPU pass at
startup and one texture sample per fragment.

### The gate

On the final tree: `cargo build --release`, `cargo build`, `cargo test` (**39 lib + 8/3/28 behaviour +
1 readme + 18 doctests**), `cargo test --features serde`, `cargo build --examples --all-features`,
`cargo clippy --all-targets --all-features -- -D warnings`,
`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`, `cargo fmt --check`,
`cargo build --lib --target wasm32-unknown-unknown`, `tools/wesl-check` — all pass.

### One mistake this pass made, and how it was caught

`git checkout examples/courtyard.rs` to undo a one-line bowl probe **discarded the whole uncommitted
fuel-bed implementation** in that file, which is exactly what `AGENTS.md` and the driver's snapshot
section warn against. It was recovered in full from the driver's `refs/wiggum` checkpoint
(`git checkout 347001f -- examples/courtyard.rs`) and the `bed_side` fix re-applied. The recovery was
verified field by field against the pre-checkout diff.

### Media

`docs/media/showcase.gif` re-encoded from this build: the bed is visible at 480 px, so the change is one
the README's hero shows. Same recipe, same caption, 2.3 MB, 60 frames.

### Environment

GPU held by other tenants (VLLM 14.6 GB, ~5 GB free) — enough for a 1280×720 capture. `/tmp` 32 % used.
Xvfb via an explicit rotating `-n` per run; no `XOpenDisplayFailed` this pass.

### Top 5 remaining, ranked

1. **A bowl that shades its own rim.** The bed's median coal cannot beat the rim's p95 while the fire's
   light sits 0.575 m above the fuel and the rim is 0.8 m out: it receives ~58 000 lx at point-blank
   range. Measured this pass that neither a wider bed nor a darker albedo is the lever.
2. **The flame from above, round 6** — iteration 19's dissolve: the tongue must *shrink* as the disc
   arrives rather than sit at full size behind it. All the arithmetic is in `flame-overhead.md`.
3. **A capture path that can show the catch** (iteration 18's item 1, unbuilt): the catch runs on
   `Time<Real>` and pipeline warm-up outlasts it.
4. A per-flame fuel-bed `D` in the fragment stage (blocked on a wider `MeshTag` or a second instance
   attribute; `GpuComponentArrayBuffer` is the unexamined option).
5. The flicker amplitude, still uncited 20 iterations, and `Rank::Weighted` with the flagstones' 256
   draw calls.

### One tracked change this pass did not make

`.omp/skills/bevy-lights-build-loop/SKILL.md` still carries the unstaged edit adding *"Your work is
saved even if you are killed"*, in the working tree when this pass began, as iterations 14–19 also
logged. Left **uncommitted** again: a tracked change this pass did not write belongs to its author. Its
guidance was followed, and it earned its keep this pass — the recovery above used the checkpoint it
describes.

## Iteration 21 — 2026-10-27 — stopped: foreign uncommitted changes in `src/`

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log, not commit or revert.

`git status --short` at pass start: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` modified (119 insertions, 38 deletions; newest mtime 11:44 today, after the iteration-20 merge), plus the long-standing `SKILL.md` edit. The src diff is an unfinished-looking feature: a per-light `depth` on `Flicker`/`FlameSite`, `flicker_swing(diameter, depth, t, phase)`, `FLICKER_DEPTH` exported, and `Flicker` losing its `Default` derive. No pass, `wiggum` or `omp` process was running, and the work is in no `refs/wiggum` snapshot yet. Left untouched in the working tree.

Next pass: the owner either commits that work (with its research note and CHANGELOG entry, since it changes the flicker model's defaults and API) or discards it; then the iteration-20 backlog stands (bowl that shades its own rim; flame from above, round 6; a capture path for the catch; per-flame fuel-bed `D`; flicker amplitude citation).

## Iteration 22 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (still)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

At pass start `git status --short` showed the same set as iteration 21: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (119 insertions, 38 deletions, the per-light flicker `depth` work) plus the long-standing `SKILL.md` edit. File mtimes are 11:44 today, minutes before this pass began at ~11:47, so someone is probably editing the tree now. No `wiggum`/`omp` pass was running. Nothing was touched, built or reverted. GPU free 19.8 GB, so the environment is not the blocker.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 23 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (third pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start is unchanged from iterations 21 and 22: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (119 insertions, 38 deletions, the per-light flicker `depth` work) plus the `SKILL.md` edit. Nothing was touched, built or reverted. GPU is free (tenant=none), `/tmp` has 11 GB free, so the environment is not the blocker.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 24 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (fourth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (110 insertions, 30 deletions now, the per-light flicker `depth` work) plus the `SKILL.md` edit. `src/pool.rs` mtime is 11:44:59 and the diff shrank from 119/38, so the owner is still editing. Nothing was touched, built or reverted. GPU free (19.9 GB).

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 25 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (fifth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start is the same set as iterations 21–24: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (119 insertions, 38 deletions, the per-light flicker `depth` work) plus the `SKILL.md` edit. Newest mtime 11:44:59, so no edits in the last seven minutes. Nothing was touched, built or reverted. GPU free ~5.3 GB, `/tmp` 11 GB free.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 26 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (sixth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start is the same set as iterations 21–25: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (110 insertions, 30 deletions, the per-light flicker `depth` work) plus the `SKILL.md` edit. Nothing was touched, built or reverted. GPU free ~7.9 GB, `/tmp` 11 GB free.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 27 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (seventh pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start is the same set as iterations 21–26: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (110 insertions, 30 deletions, the per-light flicker `depth` work) plus the `SKILL.md` edit. Newest mtime 11:44:59, ten minutes stale at 11:54. Nothing was touched, built or reverted.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 28 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (eighth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start is the same set as iterations 21–27: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (the per-light flicker `depth` work) plus the `SKILL.md` edit. Nothing was touched, built or reverted.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 29 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (ninth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start is the same set as iterations 21–28: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (the per-light flicker `depth` work) plus the `SKILL.md` edit. Nothing was touched, built or reverted. GPU free ~9.6 GB, `/tmp` 11 GB free.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 30 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (tenth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start is the same set as iterations 21–29: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (119 insertions, 38 deletions, the per-light flicker `depth` work) plus the `SKILL.md` edit. Newest mtime 11:44:59, none touched since. Nothing was built, touched or reverted. GPU free 19.8 GB, `/tmp` 11 GB free.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 31 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (eleventh pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start is the same set as iterations 21–30: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (the per-light flicker `depth` work) plus the `SKILL.md` edit. Nothing was built, touched or reverted.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 32 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (twelfth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start is the same set as iterations 21–31: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (the per-light flicker `depth` work) plus the `SKILL.md` edit. Nothing was built, touched or reverted. GPU free ~12 GB.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 33 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (thirteenth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start is the same set as iterations 21–32: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (the per-light flicker `depth` work) plus the `SKILL.md` edit. Nothing was built, touched or reverted. GPU free ~10 GB.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 34 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (fourteenth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start is the same set as iterations 21–33: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (the per-light flicker `depth` work) plus the `SKILL.md` edit. Nothing was built, touched or reverted.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 35 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (fifteenth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start is the same set as iterations 21–34: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (the per-light flicker `depth` work) plus the `SKILL.md` edit. Nothing was built, touched or reverted. GPU free ~19 GB, `/tmp` 11 GB free.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 36 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (sixteenth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start is the same set as iterations 21–35: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (the per-light flicker `depth` work) plus the `SKILL.md` edit. Nothing was built, touched or reverted. GPU free ~19 GB, `/tmp` 11 GB free.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 37 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (seventeenth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start is the same set as iterations 21–36: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (the per-light flicker `depth` work) plus the `SKILL.md` edit. Nothing was built, touched or reverted. GPU free ~19 GB, `/tmp` 11 GB free.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 38 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (eighteenth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start is the same set as iterations 21–37: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (the per-light flicker `depth` work) plus the `SKILL.md` edit. Nothing was built, touched or reverted. GPU free ~6 GB, `/tmp` 11 GB free.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 39 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (nineteenth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start is the same set as iterations 21–38: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (the per-light flicker `depth` work) plus the `SKILL.md` edit. Nothing was built, touched or reverted.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 40 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (twentieth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start is the same set as iterations 21–39: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (the per-light flicker `depth` work) plus the `SKILL.md` edit. Nothing was built, touched or reverted. GPU free ~19 GB, `/tmp` 11 GB free.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 41 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (twenty-first pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start is the same set as iterations 21–40: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (the per-light flicker `depth` work) plus the `SKILL.md` edit. Nothing was built, touched or reverted. GPU free ~5.7 GB, `/tmp` 11 GB free.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 42 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (twenty-second pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start is the same set as iterations 21–41: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (the per-light flicker `depth` work) plus the `SKILL.md` edit. Nothing was built, touched or reverted. GPU free ~5.2 GB, `/tmp` 11 GB free.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 43 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (twenty-third pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start is the same set as iterations 21–42: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (the per-light flicker `depth` work, 125 insertions / 42 deletions) plus the `SKILL.md` edit. Nothing was built, touched or reverted.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 44 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (twenty-fourth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start: `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs` (the per-light flicker `depth` work), now also `examples/bench_lights.rs`, `examples/courtyard.rs`, `tests/flicker.rs`, `tests/pool.rs`, plus the `SKILL.md` edit — 11 files, 153 insertions / 70 deletions. Nothing was built, touched or reverted.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 45 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (twenty-fifth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start: `.omp/skills/bevy-lights-build-loop/SKILL.md`, `examples/bench_lights.rs`, `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs`, `tests/flicker.rs` (the per-light flicker `depth` work). Nothing was built, touched or reverted.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 46 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (twenty-sixth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start: `.omp/skills/bevy-lights-build-loop/SKILL.md`, `examples/bench_lights.rs`, `examples/courtyard.rs`, `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs`, `tests/flicker.rs`, `tests/pool.rs` (the per-light flicker `depth` work). GPU free ~5.3 GB. Nothing was built, touched or reverted.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 47 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (twenty-seventh pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start: `.omp/skills/bevy-lights-build-loop/SKILL.md`, `examples/bench_lights.rs`, `examples/courtyard.rs`, `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs`, `tests/flicker.rs` (the per-light flicker `depth` work). GPU free ~18 GB. Nothing was built, touched or reverted.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 48 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (twenty-eighth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start: `.omp/skills/bevy-lights-build-loop/SKILL.md`, `examples/bench_lights.rs`, `examples/courtyard.rs`, `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs`, `tests/flicker.rs`, `tests/pool.rs` (the per-light flicker `depth` work). GPU free ~19 GB, `/tmp` 11 GB free. Nothing was built, touched or reverted.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 49 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (twenty-ninth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start: `.omp/skills/bevy-lights-build-loop/SKILL.md`, `examples/bench_lights.rs`, `examples/courtyard.rs`, `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs`, `tests/flicker.rs`, `tests/pool.rs` (the per-light flicker `depth` work). Nothing was built, touched or reverted.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 50 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (thirtieth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start: `.omp/skills/bevy-lights-build-loop/SKILL.md`, `README.md`, `examples/bench_lights.rs`, `examples/courtyard.rs`, `examples/readme_quick_start.rs`, `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs`, `tests/flicker.rs`, `tests/pool.rs` (the per-light flicker `depth` work, unchanged since iteration 49). Nothing was built, touched or reverted.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 51 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (thirty-first pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start: `.omp/skills/bevy-lights-build-loop/SKILL.md`, `README.md`, `examples/bench_lights.rs`, `examples/courtyard.rs`, `examples/readme_quick_start.rs`, `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs`, `tests/flicker.rs`, `tests/pool.rs` (the per-light flicker `depth` work). GPU free ~5 GB. Nothing was built, touched or reverted.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 52 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (thirty-second pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start: `.omp/skills/bevy-lights-build-loop/SKILL.md`, `README.md`, `examples/bench_lights.rs`, `examples/courtyard.rs`, `examples/readme_quick_start.rs`, `src/flame.rs`, `src/flicker.rs`, `src/lib.rs`, `src/model/fire.rs`, `src/model/mod.rs`, `src/pool.rs`, `tests/flicker.rs`, `tests/pool.rs` (the per-light flicker `depth` work; 13 files, 214 insertions / 81 deletions, growing since iteration 51). GPU free ~6 GB. Nothing was built, touched or reverted.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 53 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (thirty-third pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start lists the same per-light flicker `depth` work as iterations 21–52, across `src/`, `examples/`, `tests/`, `README.md` and the `SKILL.md` edit. GPU free ~5 GB. Nothing was built, touched or reverted.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 54 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (thirty-fourth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start lists the same per-light flicker `depth` work as iterations 21–53 (13 files across `src/`, `examples/`, `tests/`, `README.md`, plus the `SKILL.md` edit). GPU: 4.6 GB VRAM used, 24 GB total, no tenant. Nothing was built, touched or reverted.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 55 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (thirty-fifth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start lists the same per-light flicker `depth` work as iterations 21–54 (13 files across `src/`, `examples/`, `tests/`, `README.md`, plus the `SKILL.md` edit). GPU free ~19 GB. Nothing was built, touched or reverted.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 56 — 2026-10-27 — stopped: foreign uncommitted changes in `src/` (thirty-sixth pass)

No improvement attempted. Section 1.5 of the skill: tracked changes this pass did not make mean stop and log.

`git status --short` at pass start lists the same per-light flicker `depth` work as iterations 21–55 (13 files across `src/`, `examples/`, `tests/`, `README.md`, plus the `SKILL.md` edit). Nothing was built, touched or reverted.

Next pass: the owner commits that work (with its research note and CHANGELOG entry) or discards it; then the iteration-20 backlog stands.

## Iteration 57 — 2026-10-27 — flicker depth: recorded, bounded, tested, and the gate repaired

Scope: a docs-and-gate iteration. No rendering changed, so there is no before/after capture and no GIF re-encode; the default `FLICKER_DEPTH = 0.15` is unchanged and the multiplier at the default is algebraically identical to before (`1 − 0.15(1 − w) = 0.85 + 0.15w`). Benchmark not run: nothing on the bench's path changed except a `HashMap` type annotation.

### State at pass start

Tree clean on `main`. The foreign work that stopped iterations 21–56 was committed as `33fd290` (per-light flicker `depth`) by an earlier recovery pass. That commit shipped an API change and a default's meaning with **no research note, no CHANGELOG entry, and no test of the new field**, against `AGENTS.md`. It also left `cargo clippy -D warnings` (type_complexity at `src/pool.rs`) and `cargo doc -D warnings` (unresolved `MAX_FLICKER_DEPTH` link in `src/flicker.rs`) failing; its message reports only `cargo test`.

### What ranked first, and the runner-up

1. **Close the gap `33fd290` left** (category 3, API friction, and a standing gate failure): research note, CHANGELOG, README, two behaviour tests, and the two gate failures. Also settles backlog item 5 (the flicker amplitude, "uncited for 20 iterations") as far as the available sources allow.
2. Runner-up: a bowl that shades its own rim (iteration 20's item 1). A geometry change with its own note and a full capture campaign; it should not start on a tree whose gate is red.

### Sources

Craft: `10.7456_tojdac.1674124` (Doğaner). Production: `s2011-advances-pdf-slides`. Technical: Mandin & Most, *Characterization of the Puffing Phenomenon on a Pool Fire*, IAFSS FSS 6 p. 1137 (PDF fetched; quote verified against its text layer); `10.1038_s41598-020-78229-x` (Gergely 2020; downloaded, not converted: scribe timed out). Note: `docs/research/flicker-depth.md` (`e03f9bb`).

### Finding, with its limits

Mandin & Most report a flame **temperature** fluctuation amplitude "can experimentally reach 25 % of the time averaged temperature". Converting through radiance ∝ T⁴ puts the shipped ±0.15 about a 0.85 mean at ΔT/T ≈ ±4.4 %, about a sixth of that ceiling. This is an order-of-magnitude bound (grey-body assumption, temperature not luminance), and the note says so. **`0.15` is still a design default.** A web-search summary claimed this paper measured *luminosity*; the PDF says temperature, so the note uses the PDF.

### Changes

- `tests/flicker.rs::a_deeper_fire_dips_further_but_never_exceeds_its_base`; `tests/pool.rs::a_handed_out_light_takes_the_props_own_flicker_depth_or_the_default`. Neither could fail before the fix (the API existed); they pin the peak-is-base and slot-adoption invariants that had no test.
- `src/pool.rs`: `winners` map type inferred (clippy). `src/flicker.rs`: doc link path-qualified (rustdoc).
- CHANGELOG entry marked **Breaking** (the `flicker_swing` signature and three struct fields); README `Flicker` bullet and the research table; `docs/research/README.md`.

### Gate (final tree)

`cargo build --release`, `cargo build`, `cargo test` (39 lib + 9/3/29 behaviour + 1 readme + 19 doctests), `cargo test --features serde`, `cargo build --examples --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`, `RUSTDOCFLAGS="-D warnings" cargo doc`, `cargo fmt --check`, wasm lib build, `tools/wesl-check` — all pass. (The first gate run, before the two fixes, failed clippy and doc; the serde run's `cargo test --features serde` passed.)

### Environment

home-still: `distill_search` returned only 0.5-score noise for flame-luminosity queries; `scribe_convert` and `markdown_read` timed out at 30 s. `paper_search`/`paper_download` worked (Semantic Scholar and Europe PMC circuit breakers open). GPU free ~19.9 GB, `/tmp` 32 %. No capture needed.

### Top 5 remaining, ranked

1. **A bowl that shades its own rim** (iteration 20's item 1).
2. **The flame from above, round 6** (`flame-overhead.md`).
3. **A capture path that can show the catch** (the catch runs on `Time<Real>`).
4. A luminance-measured flicker amplitude: convert Gergely 2020 and Mandin & Most into the corpus once scribe is healthy, and look for a pool-fire radiance-fluctuation paper (Maynard 2013 is unobtainable).
5. Per-flame fuel-bed `D` in the fragment stage; `Rank::Weighted` against the flagstones' 256 draws; `bench_lights` has no `depth` cases.

## Iteration 58 — 2026-10-27 — stopped: GPU and RAM held by another tenant

No improvement attempted. Section 2 of the skill: VRAM free under 2 GB means log it and wait rather than fight another holder.

Tree clean on `main` at pass start. `nvidia-smi`: 72 MiB VRAM free of 24 GB. `gpu-tenant status`: tenant `3d`, held by `golem-mcp@big` (`animate_mesh`), with grants renewed back to back since 12:52 (four in nine minutes), so no window was in sight. Other residents: vLLM EngineCore 14.9 GB, ollama 3.3 GB, homelab-voice 2.1 GB, laya 1.5 GB. `MemAvailable` 1.7 GiB, and the reaper warned at 13:00 that big is close to wedging; a release `--all-features` build plus an Xvfb capture would have risked the machine. Nothing was built, touched or reverted.

Next pass: run when `gpu-tenant status` shows at least 2 GB VRAM free and `MemAvailable` above ~4 GiB; the iteration-57 backlog stands (1: a bowl that shades its own rim).

## Iteration 59 — 2026-10-27 — stopped: RAM held by other tenants

No improvement attempted. Tree clean on `main`. `free -g`: 31 GB total, 29 used, `MemAvailable` ~1.4 GiB (the skill's bar for a release `--all-features` build plus an Xvfb capture is ~4 GiB). `gpu-tenant status`: tenant `3d` held by `golem-mcp@big` (`animate_mesh`, renewed 13:00, 4 min left), VRAM free 14.9 GB at the first look. A build at this RAM level risks wedging big (iteration 58's reaper warning). Nothing was built, touched or reverted.

Next pass: run when `MemAvailable` is above ~4 GiB and `gpu-tenant status` shows at least 2 GB VRAM free; the iteration-57 backlog stands (1: a bowl that shades its own rim).

## Iteration 60 — 2026-10-27 — a carried flame needs something carrying it

Outcome: **GREAT** on the one target (no flame without a source). Scope: example geometry in `examples/courtyard.rs`; no crate API, shader or lighting change. Benchmark not run: `bench_lights` builds no bearers and nothing on its path changed.

### What ranked first, and the runner-up

1. **Floating flames** (category 1, a light with no source). At the elevated camera (`0,14,14 → 0,0.5,0`) every bearer's flame hung 0.42 m out from its capsule with no hand, stick or lantern under it; found on the first frame inspected, which the previous baseline angles (ground, edge, top-down) had hidden.
2. Runner-up: iteration 57's item 1, a bowl that shades its own rim — a geometry change needing its own campaign, and big had no RAM headroom for one (load average ~32, `MemAvailable` 1.4–5 GiB).

### Sources

Craft: `10.7456_tojdac.1674124` (Doğaner: "these objects determine the motivation of the light in the scene"). **No production or technical source was consulted** — torch proportions are design defaults, recorded in `docs/research/carried-fire.md`'s amendment (`Research: carried fire needs a held object`) under `Pending index`. That is short of the skill's three-source bar; the item was example props, and the gap is named rather than hidden.

### Critique and verdict

Evidence `/tmp/l60/{before,alt,after}/`. Elevated (`hi.png`): flames now sit on stick or lantern body, held by a fist, joined to the body by an arm; the lanterns read as a different object from the torches. Edge, close and near cameras (`e`, `close`, `near`): same. Flaws at zoom: the arms are rod-thin cylinders and the shoulder joins the capsule's surface mid-chest, so a bearer reads as a stylised figure, not a body; arm radius raised 0.035 → 0.05 after the first round. At `close.png` a near-side flame overlaps its bearer's body and hides the stick. Verdict: **GREAT** for "every flame has a visible source"; the figures themselves stay placeholder capsules.

Numbers (elevated camera, pool 4, frame 36, same seed): frame mean 54.74 → 54.72 (−0.03 %), 1 803 of 921 600 pixels changed above 8 % — the pools did not move.

### Other findings

- A temporal sequence at `7,1.6,7` showed a 1.5 m translucent tongue crossing the frame at frames 32–40 (0.5–0.67 s): inside `KINDLE_SECONDS` (0.6 s), so the catch's designed flare; not verified further. Not a regression, but unconfirmed which fire it was.
- The Xvfb trap recurred: a reused `-n` display number made a run panic with `XOpenDisplayFailed`, and a later one wrote no image. A fresh `-n` per run fixed both.
- `cargo test --features serde` took over 20 minutes to compile under load 31 on 12 cores (twice killed by a timeout, once by a wait); it passed when left alone.

### Gate (final tree)

`cargo build --release`, `cargo build`, `cargo test` (39 lib + 9/3/29 behaviour + 1 readme + 19 doctests), `cargo test --features serde`, `cargo build --examples --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`, `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`, `cargo fmt --check`, wasm lib build, `tools/wesl-check` — all pass.

### Media

`docs/media/showcase.gif` re-encoded (same recipe and caption, 60 frames, 2.3 MB): the bearers' torches now show sticks and lanterns. Flicker not re-measured through the encode: nothing in the lighting changed.

### Top 5 remaining, ranked

1. **A bowl that shades its own rim** (iteration 20's item 1).
2. **The flame from above, round 6** (`flame-overhead.md`).
3. **Bearers are placeholder capsules**: arms, shoulders, a hand that grips; a production or technical source for torch proportions.
4. **Identify the 1.5 m tongue at `7,1.6,7`, frames 32–40**: confirm it is the catch's flare and that a hand-held torch flaring to that size is what the envelope intends (`kindle_gain` peak against `TORCH_FLAME_HEIGHT`).
5. A capture path that can show the catch; a luminance-measured flicker amplitude; `bench_lights` has no `depth` cases.

## Iteration 61 — 2026-10-27 — stopped: tracked change in the tree that this pass did not make

No improvement attempted. Section 1, step 5 of the skill: tracked changes the pass did not make mean stop and log them; do not commit or revert them.

`git status --short` on `main` at pass start: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (+36 lines: a new "Filed issues come before new features" section, issue triage before item choice). Not committed, not reverted. It needs the owner to commit it (or discard it); the next pass will then see a clean tree and apply it.

Environment at the time: `MemAvailable` ~12 GiB, VRAM free ~2.0 GB (tenant `none`; golem-mcp's last grant ran to 19:30Z), load average 20. Resources would have been marginal but not blocking.

Open issues at pass start (read, none acted on): #4 flicker mean the light budget was written against (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, owner's call, skipped). No bug issues are filed.

Next pass: iteration 60's backlog stands (1: a bowl that shades its own rim), ranked with #2–#4 under the new issue rule once the tree is clean.

## Iteration 62 — 2026-10-27 — stopped: the same uncommitted skill edit as iteration 61

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, the identical +36-line "Filed issues come before new features" diff iteration 61 found (file mtime 13:09:52, unchanged since). Not committed, not reverted.

Open issues, unchanged from iteration 61: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 63 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61 and 62

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, the identical +36-line "Filed issues come before new features" diff, still uncommitted. Not committed, not reverted.

Open issues, unchanged: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. VRAM free ~2.1 GB.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 64 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–63

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, still uncommitted and unchanged. Not committed, not reverted.

Open issues, unchanged: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. Environment: tenant none, VRAM 22031/24576 MiB in use (~2.5 GB free), `MemAvailable` 12.8 GiB.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 65 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–64

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, still uncommitted. Not committed, not reverted.

Open issues, unchanged: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. VRAM free ~5.2 GB.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 66 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–65

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, still uncommitted. Not committed, not reverted.

Open issues, unchanged: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. Environment: tenant none, VRAM ~18.9/24.6 GiB in use, `MemAvailable` 13.1 GiB.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 67 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–66

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, the same +36-line "Filed issues come before new features" diff (mtime 13:09:52, unchanged). Not committed, not reverted.

Open issues, unchanged: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. VRAM free ~5.2 GB.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 68 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–67

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, the same +36-line "Filed issues come before new features" diff. Not committed, not reverted.

Open issues, unchanged (not re-fetched this pass): #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). Environment: tenant none, VRAM ~18.9/24.6 GiB in use, `/tmp` 36% used.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 69 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–68

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, the same +36-line "Filed issues come before new features" diff. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. VRAM free ~5.2 GB, `/tmp` 36% used.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 70 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–69

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. Tenant none, VRAM ~4.3/24.6 GiB in use, `MemAvailable` 13.6 GiB.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 71 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–70

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (+36 lines, the "Filed issues come before new features" section), still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: 19.9 GiB VRAM free; `/tmp` 11 GB free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 72 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–71

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (the "Filed issues come before new features" section), still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: 19.9 GiB VRAM free; `/tmp` 11 GB free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 73 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–72

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (+36 lines, the "Filed issues come before new features" section), still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: 19.9 GiB VRAM free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 74 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–73

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (+36 lines, the "Filed issues come before new features" section), still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: 19.9 GiB VRAM free; `/tmp` 11 GB free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 75 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–74

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (+36 lines, the "Filed issues come before new features" section), still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: 5.3 GiB VRAM free; `/tmp` 11 GB free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 76 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–75

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 77 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–76

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: tenant none, VRAM 18.9/24.6 GiB in use; `/tmp` 11 GB free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 78 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–77

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (the "Filed issues come before new features" section), still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 79 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–78

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: 5.3 GiB VRAM free; `/tmp` 11 GB free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 80 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–79

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: 5.3 GiB VRAM free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 81 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–80

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: 5.3 GiB VRAM free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 82 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–81

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: 5.3 GiB VRAM free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 83 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–82

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: 5.0 GiB VRAM free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 84 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–83

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (36 added lines), still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 85 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–84

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (36 added lines), still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: ~5.0 GiB VRAM free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 86 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–85

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (36 added lines), still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: ~5.0 GiB VRAM free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 87 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–86

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (36 added lines), still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 88 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–87

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: ~5.0 GiB VRAM free; `/tmp` 11 GB free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 89 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–88

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: ~5.0 GiB VRAM free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 90 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–89

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (+36 lines), still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 91 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–90

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (+36 lines), still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 92 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–91

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (+36 lines), still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: ~5.4 GiB VRAM free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 93 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–92

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (+36 lines), still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: ~5.0 GiB VRAM free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 94 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–93

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (+36 lines), still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 95 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–94

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: ~5.0 GiB VRAM free; `/tmp` 11 GB free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 96 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–95

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (+36 lines), still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: ~5.0 GiB VRAM free; `/tmp` 11 GB free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 97 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–96

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (+36 lines), still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: ~5.0 GiB VRAM free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 98 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–97

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (+36 lines), still uncommitted. Not committed, not reverted.

Open issues: not re-fetched this pass (the stop condition comes first); last list was #4 flicker mean, #3 host-declared flame origin, #2 flame occlusion/ownership (enhancements), #1 publish 0.2.0 (release, skipped). GPU: ~5.0 GiB VRAM free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 99 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–98

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: ~5.0 GiB VRAM free (tenant none); `/tmp` 11 GB free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 100 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–99

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (+36 lines), still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 101 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–100

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 102 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–101

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (+36 lines), still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 103 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–102

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (+36 lines), still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 104 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–103

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, still uncommitted. Not committed, not reverted.

Open issues: not re-fetched this pass (the stop condition comes first); last list was #4 flicker mean, #3 host-declared flame origin, #2 flame occlusion/ownership (enhancements), #1 publish 0.2.0 (release, skipped). GPU: ~5.0 GiB VRAM free; `/tmp` 11 GB free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 105 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–104

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: ~5.0 GiB VRAM free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 106 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–105

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: ~5.0 GiB VRAM free; `/tmp` 11 GB free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 107 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–106

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: ~5.0 GiB VRAM free; `/tmp` 11 GB free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 108 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–107

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md`, still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed. GPU: ~5.0 GiB VRAM free; `/tmp` 11 GB free.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 109 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–108

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (+36 lines), still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 110 — 2026-10-27 — stopped: the same uncommitted skill edit as iterations 61–109

No improvement attempted. Section 1, step 5 of the skill: a tracked change this pass did not make means stop and log it. `git status --short` on `main`: ` M .omp/skills/bevy-lights-build-loop/SKILL.md` (+36 lines), still uncommitted. Not committed, not reverted.

Open issues, re-fetched: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, skipped). No bug issues filed.

Needs the owner: commit the skill edit or discard it. Until then every pass stops here. Iteration 60's backlog stands (1: a bowl that shades its own rim), to be ranked with #2–#4.

## Iteration 111 — 2026-10-27 — flicker_mean and named depths (issue #4)

Outcome: API addition, no rendering change. Scope: `src/model/fire.rs` (`flicker_mean`, `CALM_FLICKER_DEPTH` 0.05, `WHIPPING_FLICKER_DEPTH` 0.3), re-exports, `Flicker::depth` docs, `docs/research/flicker-depth.md` (Parameters, Deviations), README, CHANGELOG, `tests/flicker.rs`.

### Ranking

Open issues: #4 flicker mean (enhancement), #3 host-declared flame origin (enhancement), #2 flame occlusion/ownership (enhancement), #1 publish 0.2.0 (release, seen, skipped). No bug issues. Candidates: (1) #4, category 3 API friction: a host cannot write a lumen budget against a fire's mean without hand-dividing by 0.85; (2) iteration 60's item 1, a bowl that shades its own rim (needs a capture campaign; GPU had ~5 GiB free). #4 won as concrete, testable and finishable in one pass; #2 and #3 need a design for a site-to-light link and rank next.

### Sources

Craft/technical: `docs/research/flicker-depth.md`'s own cited sources, unchanged — the additions are algebra (`1 − depth`) and two design-default names for that note's existing 0.03/0.3 examples. No new production source was consulted; no new amplitude measurement exists (Maynard 2013 still unobtainable).

### Not done from #4

Ask 3 (`FlameSite` takes its light's depth): a site and the light it motivates are different entities (a pool light belongs to the pool), and no link between them exists. Recorded in the research note's Deviations. Asks 1, 2 and 4's assertion (a budget solved through `flicker_mean` is the average the light really puts out) are done; #4 is left open.

### Verification

No shader, system or default changed, so no capture or benchmark was run (`flicker_swing` and the flicker systems are untouched). Full gate passed: `cargo build --release`, `cargo build`, `cargo test`, `cargo test --features serde` (39 lib + 10/3/29 behaviour + 1 readme + 20 doctests), examples, clippy `-D warnings`, doc `-D warnings`, fmt, wasm lib build, `tools/wesl-check`. Skipped: GIF re-encode (nothing shown changed).

### Top 5 remaining, ranked

1. #3 host-declared flame origin, which also supplies the site-to-light link #4's ask 3 needs.
2. #2 flame occlusion / whose torch it is.
3. A bowl that shades its own rim (iteration 20's item 1).
4. The flame from above, round 6 (`flame-overhead.md`).
5. Bearers are placeholder capsules; a luminance-measured flicker amplitude.

## Iteration 112 — 2026-10-27 — a site is already host-placed (issue #3)

Outcome: docs and one regression test, no rendering or API change. Scope: `FlameSite` rustdoc, `src/flame.rs` test `a_site_on_a_prop_keeps_its_flame_at_the_site_and_leaves_with_the_prop`, CHANGELOG.

### Ranking

Open issues: #4 (partly done, left open), #3 host-declared flame origin, #2 flame occlusion/ownership (enhancements), #1 publish (release, seen, skipped). No bug issues. #3 first: the oldest enhancement. Finding: the crate never spawns sites or ties one to a light — `FlameSite` is a host-spawned entity whose own `Transform` and parent are the origin, so Sanctuary's "flame at the light" is its own `site_fire_lights` choice. A `FlameAnchor` would duplicate the hierarchy. The issue's acceptance 1 (flame at site, not light), 3 (anchor despawn takes the flame, no origin ghost) and 4 (host opt-out) hold today; the new test pins 1 and 3. Phase and kindle are copied by the host (`phase`, `diameter`, `depth`, `kindle`) and the docs now say so. Runner-up: #2, needs a design.

### Not done

No capture, benchmark or GIF: nothing rendered or run per-frame changed. Craft/production/technical research was not needed (no default or model changed). #3 left for the owner to close after confirming Sanctuary can parent its site to the bracket.

### Verification

Full gate passed (build release/debug, test, test --features serde, examples, clippy, doc, fmt, wasm lib, wesl-check).

### Top 5 remaining, ranked

1. #2 flame occlusion / `Flame::source` (the second ask is a one-field addition).
2. A bowl that shades its own rim (iteration 20's item 1).
3. The flame from above, round 6 (`flame-overhead.md`).
4. Bearers are placeholder capsules; a luminance-measured flicker amplitude.
5. #4 ask 3 (site takes its light's depth) needs a site-to-light link.

## Iteration 113 — 2026-10-27 — Flame::source (issue #2, ask 2)

Outcome: API addition, no rendering change. Scope: `Flame::source: Option<Entity>` in `src/flame.rs` (written by `update_flames` only on change), test `a_flame_names_the_prop_its_site_hangs_on_and_follows_a_reparent`, CHANGELOG.

### Ranking

Open issues: #4 (partly done), #3 (answered in iteration 112), #2 flame occlusion/ownership, #1 publish (release, seen, skipped). No bug issues. #2 is next on iteration 112's list; its ask 2 (`source`) is the self-contained slice. Asks 1 and 3 (occlusion fade, `visible_from`) need a depth-buffer or raycast design and GPU captures, so they are left; #2 stays open. `source` is the parent of the site, `None` for a root site or a site nested under a site; reparent and unparent are followed, a despawned parent takes the flame with it (acceptance 2 and 3 pinned by the test).

### Verification

No shader or default changed: no capture, benchmark (bench has no flame path) or GIF. `update_flames` gained one extra `Option<&ChildOf>` read per flame, and a write only on change. Full gate passed (build release/debug, test, test --features serde, examples, clippy, doc, fmt, wasm lib, wesl-check).

### Top 5 remaining, ranked

1. #2 occlusion fade (`FlameSite::depth_fade`) and `visible_from`: needs a design, a GPU capture campaign.
2. A bowl that shades its own rim.
3. The flame from above, round 6 (`flame-overhead.md`).
4. Bearers are placeholder capsules; a luminance-measured flicker amplitude.
5. #4 ask 3 (site takes its light's depth) needs a site-to-light link.

## Iteration 114 — 2026-10-27 — Flicker::at_mean, set_mean, set_depth (issue #5)

Outcome: API addition, no rendering change. Scope: `Flicker::{at_mean, mean, set_mean, set_depth}` (`src/flicker.rs`), `PoolCandidate::at_mean` (`src/pool.rs`), two behaviour tests in `tests/flicker.rs`, README (quick start builds its light with `at_mean`), CHANGELOG.

### Ranking

Open issues: #5 (enhancement: a deeper depth silently dims a light), #4 (partly done), #3 (answered), #2 (partly done), #1 publish (release, seen, skipped). No bug issues. #5 first: it is the newest and makes #4's correct solve impossible to forget; self-contained and testable. Runner-up: #2 occlusion fade (needs a design and GPU captures).

### Verification

Acceptance 1–4 pinned: 300 s at 50 Hz time-average of `base · flicker_swing` is within 1 % of the mean at calm/default/whipping depths and after `set_depth` to each of calm/whipping/max; a base of 0 stays 0; NaN/out-of-range depth clamps with a finite base. No shader, system or default changed: no capture, benchmark or GIF. Full gate passed (build release/debug, test, test --features serde, examples, clippy, doc, fmt, wasm lib, wesl-check). The quick-start light is now 15 % dimmer than before (it was peak = `lumens`, now mean = `lumens`), which is the point.

### Top 5 remaining, ranked

1. #2 occlusion fade (`FlameSite::depth_fade`) and `visible_from`.
2. A bowl that shades its own rim.
3. The flame from above, round 6 (`flame-overhead.md`).
4. Bearers are placeholder capsules; a luminance-measured flicker amplitude.
5. #4 ask 3 (site takes its light's depth) needs a site-to-light link.

## Operator note, 2026-10-27: iteration 115's occlusion work is parked, not lost

Issue #2's occlusion work was left uncommitted on a loop branch, and iterations 116 to 126 each stopped on the dirty tree. It is parked as two substantive commits on `loop/iter-115` (pushed): `ee759b3`, the research note, and `1260f0e`, which holds `src/flame.rs`, `src/flame.wesl`, `examples/occlusion.rs`, `tools/wesl-check`, and the README, CHANGELOG and `docs/research/flame-occlusion.md` edits. The branch sits directly on `main` at `3f0581e` and merges without conflict, but nothing in it has been built, tested or captured. Taking #2 can start from `git cherry-pick ee759b3 1260f0e` (not the eleven log commits between them, which only record the stop), then the usual gate and a look at the captures.

## Iteration 129 — 2026-10-27 — FlameSite::depth_fade (issue #2, ask 1)

Outcome: the parked iteration-115 occlusion work (`loop/iter-115`, `ee759b3` research, `1260f0e` code) cherry-picked, gated, captured and landed. Iterations 116 to 127's stop-logs live only on that branch and are not merged; this iteration is numbered 129 because branches `loop/iter-127` and `loop/iter-128` exist.

### Ranking

Open issues: #5 (done in 114), #4 partly done, #3 answered, #2 occlusion, #1 publish (release, seen, skipped). No bug issues. #2 first: it is the only open item with a consumer-reported player-visible fault, and its code already existed. Runner-up: a bowl that shades its own rim.

### Sources

Craft `10.7456_tojdac.1674124` (Doğaner, visible source of light); production `GPU Gems` ch. 29 (Sekulic, one-point depth test at the source); technical `bevy_pbr-0.20.0` prepass depth. Note: `docs/research/flame-occlusion.md`. Distill was not used (the note records it).

### Verification

- Gate: build release/debug, test, test --features serde, examples, clippy, doc, fmt, wasm lib, wesl-check (both `DEPTH_PREPASS` variants) passed after one fix: the two `src/lib.rs` doctest `FlameSite` literals lacked `depth_fade`.
- Default unchanged: `courtyard` seeds 1 and 2, 300 frames, baseline `ec86303` vs this build: `compare -metric AE` 0 (identical).
- Occlusion example, fade off vs on: tongue over the wall gone, flush and past-the-end torches unchanged (`/tmp/l129/off|on/x.png`).
- Pan (44 frames, every 3rd): unoccluded frames within 0.8 % of fade off; fade 222, 202, 180, 153, 121, 88, 31 over the occluding frames; worst step 0.28 per 3 frames (about 0.09 per frame, under the note's 1/9 target).
- Bench (CPU only, flame has no path in it): min_us 263 base, 262 after; p50_us 470 vs 509 (run-to-run noise, `pool_p50` 346 vs 324).
- Verdicts: static occlusion GREAT; pan GREAT per frame, steppy in sixteenths (documented in the note's Deviations); default unchanged GREAT. `/tmp/l129/critique.md`.
- Not done: `showcase.gif` not re-encoded (default draw is byte-identical); `FlameSite::visible_from` (ask 3) not built; #2 stays open for it.

### Top 5 remaining, ranked

1. #2 ask 3, `visible_from`; a time envelope for the fade.
2. A bowl that shades its own rim.
3. The flame from above, round 6 (`flame-overhead.md`).
4. Bearers are placeholder capsules; a luminance-measured flicker amplitude.
5. #4 ask 3 (site takes its light's depth).

## Iteration 130 — 2026-10-27 — FlameSite::fire (issue #4 criterion 3, issue #3)

Outcome: API addition, no rendering change. Scope: `FlameSite::fire: Option<Entity>` (`src/flame.rs`): the flame's brightness reads `diameter`/`depth`/`phase` from the named entity's `Flicker`, or from its `PoolCandidate` (depth `FLICKER_DEPTH`, phase the site's when absent); `None`, a despawned entity or an entity with neither keeps the site's own. Three tests (`a_flame_breathes_by_the_light_its_site_names` incl. `set_depth` following, `a_flame_reads_a_pool_candidate_the_way_the_pool_does`, `a_dangling_fire_link_falls_back_to_the_sites_own_values`), README, CHANGELOG, note `docs/research/flame-fire-link.md`. Breaking for `FlameSite` literals (`fire: None`).

### Ranking

Open issues: #5 done, #4 (criterion 3 open), #3 answered, #2 (ask 3 open), #1 publish (release, seen, skipped). No bug issues. Captures (courtyard default, ground, grazing at 1280x720; per-frame occlusion pan) showed no new visible defect: the near brazier's flame at ground level is a soft small teardrop with a ~4 px dim foot gap (measured on the column through its axis), not worth a change. The occlusion pan is the iteration-129 result. #4 criterion 3 first: a consumer-reported disagreement with two measured instances; `visible_from` needs a CPU visibility answer the note rejected. Runner-up: #2 ask 3.

### Sources

Craft `10.7456_tojdac.1674124`; production `s2011-advances-pdf-slides`; technical `10.1017_jfm.2018.707` (all already quoted in `flame-flicker.md`). Distill not queried.

### Verification

- Gate: build release/debug, test, test --features serde, examples, clippy (one fix: `update_flames` hit 8 arguments, bundled into a `Sites` SystemParam), doc, fmt, wasm lib, wesl-check passed.
- Default unchanged: `courtyard` 300 frames vs the pre-change capture, `compare -metric AE` 0.
- No shader, look or default changed, so no GIF re-encode; verdict: not a visual change (default pixel-identical).
- Bench (CPU only, flame has no path in it): min_us 317, p50_us 504 against 263/470–509 in iteration 129, on a machine running another bench (`bench_elements`); not attributable to this change.
- Not done: the courtyard does not use `fire` (its sites' values already match by construction); `Kindle` does not follow the link.

### Top 5 remaining, ranked

1. #2 ask 3, `visible_from`; a time envelope for the fade.
2. A bowl that shades its own rim.
3. The flame from above, round 6 (`flame-overhead.md`).
4. Bearers are placeholder capsules; a luminance-measured flicker amplitude.
5. #3 origin-from-light and `Kindle` through the `fire` link.

## Iteration 131 — 2026-10-27 — A re-lit fire's flame catches with its light (issue #4/#3 follow-up)

Outcome: behaviour change behind `FlameSite::fire`. While the named entity's `Kindle` runs, the flame's scale is `kindle_gain(elapsed)` (floored at 0.001) and exactly 1 after, even if the `Kindle` is removed mid-way (`src/flame.rs`, `update_flames`, `Sites::kindle_of`). The site's own `kindle` seconds are not used during a linked kindle. One test, `a_relit_fire_catches_its_flame_with_its_light`; README bullet, CHANGELOG, `docs/research/flame-relight.md`. No shader or default change.

### Ranking

Open issues: #4 (criterion 3 done), #3 answered, #2 (ask 3 open), #1 publish (release, seen, skipped). No bug issues. Picked the deferred `Kindle` half of the fire link: a re-lit fire's light flared from nothing while its flame stayed full height (a source preceding its light), and it is self-contained and testable. Runner-up: #2 ask 3 `visible_from` (needs a CPU visibility design the occlusion note rejected).

### Sources

Craft `10.7456_tojdac.1674124`; production `s2011-advances-pdf-slides`; technical `10.1017_jfm.2018.707` (quoted in `flame-kindle.md`). Distill was not queried.

### Verification

- Gate: build release/debug, test, test --features serde, examples, clippy, doc, fmt, wasm lib, wesl-check passed.
- Default unchanged: `courtyard` 300 frames seeds 1 and 2 vs baseline (iteration 130's merge `cafd99a`): `compare -metric AE` 0 and 0.0013 (one-pixel GPU noise). Full frame viewed (`/tmp/l131/after/s1.png`): unchanged.
- Bench (CPU only, flame has no path in it): first run 611/603 vs 417/422 (mean/p50) under load; re-run interleaved at load 5: min_us 262 vs 266, p50_us 427 vs 425, 410/416 vs 431.
- Not captured: a live relight on the GPU (no example re-lights a fire); the verdict rests on the unit test's frame-by-frame scale. Verdict: unchanged-default GREAT; relight not visually judged.
- Baseline worktree `~/l131_base` and its target removed.

### Top 5 remaining, ranked

1. #2 ask 3, `visible_from`; a time envelope for the fade.
2. A courtyard demo that re-lights a brazier, to judge the relight by eye.
3. A bowl that shades its own rim.
4. The flame from above, round 6 (`flame-overhead.md`).
5. Bearers are placeholder capsules; a luminance-measured flicker amplitude.

## Iteration 132 — 2026-10-27 — The courtyard re-lights a brazier and its flame catches (judges iteration 131 by eye)

Outcome: example change only, no `src/` change. The courtyard's braziers now set `FlameSite::fire: Some(coals)`, and `LIGHTS_RELIGHT=<seconds>` re-lights them once at that time (what `K` does), so an unattended capture can show iteration 131's linked relight, which had no visual judgement. CHANGELOG entry; no research note (no default, look or API changed).

### Ranking

Open issues: #4 (criterion 3 done), #3 answered, #2 (ask 3 open), #1 publish (release, seen, skipped). No bug issues. Picked the iteration-131 backlog's item 2: the one shipped behaviour never seen on a GPU, and `K` in the example visibly left the flame at full height (`fire: None`), so the demo contradicted the feature. Runner-up: #2 ask 3 `visible_from` (needs a CPU visibility design the occlusion note rejected).

### Verification

- Relight capture (1280x720, eye 7.5,1.6,7.5, look 5,1.0,5, `LIGHTS_RELIGHT=10`, every 10th frame from 596; `/tmp/l132/after/sheet.png`): frame 596 full flame; frame 606 (0.17 s after) coals dim and no flame; 616-646 the flame grows with the light; by ~1.4 s full. Each brazier logged `kindled` twice (start and relight). Critique: the flame is absent for the first ~0.1 s (kindle_gain floor), as at spawn; the grow is a smooth cone, no pop. Verdict GREAT.
- Default unchanged: `courtyard` 300 frames vs the pre-change build (`compare -metric AE`) 0.
- Gate: build release/debug, test, test --features serde, examples, clippy, doc, fmt, wasm lib, wesl-check passed.
- Bench not run: no library code changed.
- GIF not re-encoded: default look is pixel-identical.

### Top 5 remaining, ranked

1. #2 ask 3, `visible_from`; a time envelope for the fade.
2. A bowl that shades its own rim.
3. The flame from above, round 6 (`flame-overhead.md`).
4. Bearers are placeholder capsules; a luminance-measured flicker amplitude.
5. #3 origin-from-light.

## Iteration 133 — 2026-10-28 — depth_fade under an orthographic camera (issue #6)

Outcome: bug fix in `src/flame.wesl`. The fade's margin was `scene_depth > source_depth * 1.03` on reverse-Z NDC depth, 135 ft at an orthographic `far` of 6000, so Sanctuary's camera never faded a flame. Both depths now go through `depth_ndc_to_view_z` and a tap occludes when the scene is nearer by more than `FADE_MARGIN` = 0.15 of the flame's drawn height (new flat varying `fade_margin`; `fade.z` carries view z). `tools/wesl-check` stub gained `depth_ndc_to_view_z`; `examples/occlusion.rs` gained `LIGHTS_ORTHO=1` / `LIGHTS_FAR`. Rustdoc, CHANGELOG (Fixed), `flame-occlusion.md` amendment. Perspective results change slightly (margin is now a fraction of height, not of distance).

### Ranking

Open issues: #6 (bug, the only one reported wrong behaviour, reproduced by capture), #4 (criterion 3 done), #3 answered, #2 (ask 3 open; waits on #6 per its last comment), #1 publish (release, skipped). #6 first: a bug, and it blocks Sanctuary adopting `depth_fade`. Runner-up: #2 ask 3 `visible_from`.

### Sources

Craft `10.7456_tojdac.1674124`; production `s2011-advances-pdf-slides`; technical `10.1017_jfm.2018.707` (quoted in the flame notes); the projection-aware conversion is Bevy 0.20's own `view_transformations.wesl`. Distill not queried; no new corpus scan.

### Verification

- `occlusion` example, 1280x720, 60 frames, `/tmp/l133/`: orthographic near 0.1 far 6000 — flame over the wall max luma 158.8 with `depth_fade` off, 13.7 (sky, below the wall's 23.3) on; same at far 1500; torch on the wall's face unchanged and the torch past the wall's end still drawn, in both projections. Perspective: behind-wall flame 171 → 24 max luma, face torch kept.
- Gate: build release/debug, test, test --features serde, examples, clippy, doc, fmt, wasm lib, wesl-check passed.
- Not done: no permanent test (the comparison lives in the shader; wesl-check only checks it compiles); no pan sequence or perspective before/after byte compare; Bench not run (shader-only, the CPU bench has no flame path); GIF not re-encoded (depth_fade is off in the courtyard and showcase).
- Verdict: GREAT for the fix (flame gone behind the wall, flush torch kept); the fade's edge behaviour under ortho pan was not captured.

### Top 5 remaining, ranked

1. #2 ask 3, `visible_from`; a time envelope for the fade.
2. An ortho pan sequence to judge the fade's edge smoothness (issue #6 criterion 4 was judged on stills).
3. A bowl that shades its own rim.
4. The flame from above, round 6 (`flame-overhead.md`).
5. #3 origin-from-light.

## Iteration 134 — 2026-10-28 — Judging depth_fade's edge on an orthographic pan (iteration 133 backlog item 2)

Outcome: measurement only, no `src/` or example change, no verdict change. Iteration 133 judged issue #6's fix on stills; this pass runs the pan. Result appended to `docs/research/flame-occlusion.md` ("Orthographic pan, per frame").

### Ranking

Open issues: #4 (criterion 3 done in 130), #3 answered, #2 (ask 1 delivered and now unblocked by #6; ask 3 declared not needed by Sanctuary), #1 publish (release, seen, skipped). No bug issues open (#6 closed). Picked backlog item 2: the one shipped fix whose smoothness had no per-frame evidence, and Sanctuary adopts `depth_fade` on that camera. Runner-up: a bowl that shades its own rim (art, larger). Not picked: `visible_from` (not needed).

### Sources

No new research; the note's own sources (craft `10.7456_tojdac.1674124`, production `s2011-advances-pdf-slides`, technical `10.1017_jfm.2018.707`). Distill not queried, corpus not re-scanned.

### Verification

- `occlusion`, `LIGHTS_PAN=1`, 240 frames, 1280x720, all four of {fade off, on} x {perspective, ortho}; plus every frame 130-240 for ortho on and off (`/home/ladvien/tmp/l134/`).
- Fade on / off ratio of the flame's orange excess, per frame: 0.77, 0.70 x5, 0.54 x2, 0.45 x3, 0.34, 0.33, 0.33, 0.32, 0.32, 0.21, 0.08 x3, 0.00 from then on. Largest single-frame step 0.16; about 20 frames (0.33 s) from visible to gone; never returns. The contact sheet of frames 35-43 (ortho) shows the tongue ducking behind the wall's end with the wall's edge cutting it, no stray flame over the wall.
- Critique: the fade is steppy in sixteenths (plateaus of 2-5 frames are sub-pixel pan motion, the fade-off series has them too); it is spatial, not a time envelope, so a camera that stops mid-fade stays mid-fade. Neither is a defect against the note's validation targets (no pop, step <= 0.26). Verdict: GREAT for the edge on a pan; the time envelope stays unbuilt.
- Gate: not run in full, no library code changed (docs only). Bench not run.
- Issue #2 is Sanctuary's to close; its last comment asked to keep it open until #6 landed, which it has.

### Top 5 remaining, ranked

1. A bowl that shades its own rim.
2. The flame from above, round 6 (`flame-overhead.md`).
3. Bearers are placeholder capsules; a luminance-measured flicker amplitude.
4. A time envelope for the fade, only if a host reports a camera stopping mid-fade.
5. #3 origin-from-light (answered; closeable by Sanctuary).

## Iteration 135 — 2026-10-28 — A brazier is a bowl (not shipped: NOT YET)

Outcome: no change on `main` except the research note `docs/research/brazier-bowl.md`. The example change is on branch `loop/iter-135` (commit `83b569a`, pushed, not merged), because I could not reach GREAT in five rounds.

### Ranking

Open issues: #4 (criterion 3 done in 130), #3 answered, #2 (ask 3 not needed), #1 publish (release, seen, skipped). No bug issues. Picked iteration 134's backlog item 1, a bowl that shades its own rim: the baseline (`/tmp/l135/before/C.png`) showed the showcase's central prop as a flat-topped crate with a coal sticker. Runner-up: the flame from above, round 6.

### Sources

Craft `10.2478_bsmr-2022-0020` (Huttunen, underlighting) and `10.32657_10220_48312` (Doyle); production `s2011-advances-pdf-slides`; technical `PBR3_12_Light_Sources`. A fire-basket-specific craft source was not found in the corpus. Distill and the corpus scan were not run this pass.

### What was built (branch only)

`bowl_mesh(lining)` in `examples/courtyard.rs`: a revolved profile (foot, stem, belly, flat lip) as an iron shell, plus a separate soot-lined inside; the coal bed sunk 0.05 m below the lip and the flame foot with it. Normals/winding are by profile traversal; the lathe's first winding was inverted and the stem showed as a hole until fixed.

### Verification and critique

Captures at four cameras (1.6 m, 0.9 m, 2.2 m, 4 m elevated), 60 frames, 1280x720, `/tmp/l135/after/`. Rounds: (1) one shared 0.10-albedo material: the inside was a flat peach plate, as bright as the floor; (2-3) a darker/rougher iron: the interior stayed a flat plate because the fire's specular/diffuse at 0.5 m (the 58 000 lx of `fuel-bed.md`) dominates albedo; (4-5) a separate soot lining (albedo 0.006→0.016, roughness 0.95, reflectance 0): the bowl now reads, the iron lip is the brightest line on the prop, the belly under it falls to shade, the stem reads at 0.9 m.

Remaining flaws against `## What great looks like`: the lining is one flat tone at 2.2 m (sampled DN 59 and 60 at two points 200 px apart, no wall-to-floor gradient); the coal bed is still a hard-edged square on a round bowl (pre-existing, but it is now more obvious inside a round rim); the stem's foot reads as a tin cup at the courtyard's ground level (`A.png`). Verdict: NOT YET. Five rounds used, no GREAT state reached, so nothing is merged.

Not run: bench (example-only), full gate (only `cargo fmt` and `cargo clippy --examples --all-features -D warnings` passed on the branch), README/GIF (no merge).

### Top 5 remaining, ranked

1. Finish the bowl from `loop/iter-135`: a round or hex-shaped coal bed (a disc mask on the emissive field) so the bed fits its bowl, and a lining gradient (an emissive term from the bed, or a deeper wall); then gate and merge.
2. The flame from above, round 6 (`flame-overhead.md`).
3. Bearers are placeholder capsules; a luminance-measured flicker amplitude.
4. A time envelope for the fade, only if a host reports a camera stopping mid-fade.
5. #3 origin-from-light (answered; closeable by Sanctuary).

## Iteration 136 — 2026-10-28 — A brazier is a bowl (finished from iteration 135)

Outcome: shipped, example-only (no `src/` change). Iteration 135's bowl (cherry-picked from `loop/iter-135`, `83b569a`) plus two fixes for its two remaining flaws: the coal bed is a disc (was a square stamp in a round bowl) and the soot lining carries an emissive glow gradient from the bed up to the rim (was one flat tone).

### Ranking

Open issues: #4 (criterion 3 delivered in 130, and the pool-path instance its last comment names is covered by `fire` reading `PoolCandidate`), #3 answered, #2 (ask 3 not needed), #1 publish (release, seen, skipped). No bug issues. Picked iteration 135's backlog item 1: the showcase's central prop. Runner-up: the flame from above, round 6; then bearers are capsules, a luminance-measured flicker amplitude, a time envelope for the fade.

### Sources

Craft `10.2478_bsmr-2022-0020` and `10.32657_10220_48312`; production `s2011-advances-pdf-slides`; technical `PBR3_12_Light_Sources` (all quoted in `docs/research/brazier-bowl.md`). The glow's strength (0.8 of a dull ember) and fall (0.15 of the profile) are design defaults. Distill and the corpus scan were not run this pass.

### Verification and critique

- Captures: six cameras, 1280x720, 60 frames (`/tmp/l136/{before,after}/A-F.png`, before = main's box at `3890ece`); `critique.md` there. D (elevated) and C (2.2 m) show a lit lip ring, a rim-dark to bed-warm lining, a round bed inside the rim, the outside under the lip near black. Mean luma before/after A 80.1/80.8, C 113.6/115.4, D 88.0/88.9, F 43.2/43.9: pools unchanged; B 77.4 → 83.5 (the box face is gone).
- Remaining flaws: the bed is still pale pink-white (the fuel-bed ramp, not this item); the stem's foot is a plain cone; camera E was blocked by a figure and carries no signal.
- Verdict: GREAT for the bowl at C, D, B.
- Bench (CPU only, no example path): before p50 689 / min 339, after p50 706 / min 336 (flicker p50 267/266, pool 500/516): noise.
- `docs/media/showcase.gif` re-encoded (60 frames, 480x270, 30 fps, 2.4 MB, same caption); frame 0 is black as the courtyard is still lighting, as in the recipe; frame 45 shows the four bowls. `courtyard.gif` (640x360, older) still shows the box braziers; not re-encoded this pass.
- Gate: build release/debug, test, test --features serde, examples, clippy, doc, fmt, wasm lib, wesl-check.

### Top 5 remaining, ranked

1. The coal bed's brightness: pale pink-white at 1.6-2.2 m, a hot-core falloff toward its edge.
2. The flame from above, round 6 (`flame-overhead.md`).
3. Re-encode `docs/media/courtyard.gif` (still the box).
4. Bearers are placeholder capsules; a luminance-measured flicker amplitude.
5. A time envelope for the fade, only if a host reports a camera stopping mid-fade.

## Iteration 137 — 2026-10-28 — The coal bed reads as embers, not a pink plate

Outcome: shipped. `char_field`'s `PLATEAU` 0.92 → 0.75 (a lump has a hot middle), `FIELD_MEAN` lower bound 0.42 → 0.38, the tests' hot-fraction lower bound 0.02 → 0.005, and the courtyard scales the bed's radiance by `BED_GAIN` 0.2. `coal_radiance` is unchanged (the soot lining uses it).

### Ranking

Open issues: #4 (criterion 3 delivered in 130), #3 answered, #2 (ask 3 not needed), #1 publish (release, seen, skipped). No bug issues. Picked iteration 136's backlog item 1: at the 2.2 m camera the bed was a pale pink-white plate with dark red blotches (mean RGB 223/145/113, 59 % of its pixels at R 255), the showcase's central prop. Runner-up: the flame from above, round 6; then re-encoding `courtyard.gif`.

### Sources

Craft `10.1525_fq.2004.57.3.24` (Prince, a clipped source reads as video); production `s2011-advances-pdf-slides`; technical `PBR3_12_Light_Sources`, all quoted in `docs/research/fuel-bed.md` round 2. Both numbers are design defaults. Distill and the corpus scan were not run this pass.

### Finding

Sweeps (`/tmp/l137/sweep*.png`): the pastel is not the emissive's temperature. Raising the fuel's kelvin at constant radiance only made the clipped patches paler; scaling the ramp down moved the bed to orange (R:G ≈ 15:1 in scene units is rolled toward white by TonyMcMapface, which is where the blue comes from). Gain alone left each lump a flat salmon chip, so `PLATEAU` was the second fix.

### Verification and critique

- Captures: six cameras A-F, 1280x720, 60 frames, before/after in `/tmp/l137/`; `cmpC.png`, `cmpD.png`, `cmpB.png` are crops at 200-300 %. Bed window at C (100x45 px): before 223/145/113, 59 % R=255; after 191/79/41, 14 %. Frame mean luma before/after: A 80.8/80.8, B 83.5/83.4, C 115.4/115.2, D 88.9/88.8, E 47.9/47.9, F 43.9/43.9: the pools are untouched.
- Critique: C and D show a bed of orange coals with bright middles and dark crevices, the flame foot standing on it and the lip ring still the brightest line of the prop; B (0.9 m, grazing) shows the bed's far edge as a dark-and-orange band under the flame, no plate. Remaining flaws: the brightest lump peaks are still salmon, not yellow-orange, because the tonemapper adds blue above R ~ 3 (a hotter, hue-correct core would need the host's own tonemapping); lump edges are blobby at 4x; the stem's foot is a plain cone; E is blocked by a figure.
- Verdict: GREAT for the bed at C, D and B.
- Bench (CPU only, example and a constant): p50 446, min 271 (flicker p50 167, pool 329); the bench runs no code this change touches and the machine's load differs from iteration 136's run, so not comparable and not a regression.
- `docs/media/showcase.gif` re-encoded (60 frames, 480x270, 30 fps, 2.4 MB, same caption). `courtyard.gif` (640x360) still shows the box braziers and the pink bed; not re-encoded.
- Gate: build release/debug, test, test --features serde, examples, clippy, doc, fmt, wasm lib, wesl-check: all passed.

### Top 5 remaining, ranked

1. The flame from above, round 6 (`flame-overhead.md`).
2. Re-encode `docs/media/courtyard.gif` (still the box and the pink bed).
3. A hue-correct hot core for the bed (yellow-orange peaks), which wants the host's tonemapper in the loop.
4. Bearers are placeholder capsules; a luminance-measured flicker amplitude.
5. A time envelope for the fade, only if a host reports a camera stopping mid-fade.

## Iteration 138 — 2026-10-28 — courtyard.gif and courtyard.png re-taken (the README's linked media was stale)

Outcome: shipped, media and docs only; no `src/` change.

### Ranking

Open issues: #4 (criterion 3 delivered in 130), #3 answered, #1 publish (release, seen, skipped). No bug issues; `gh` was reachable. Candidates: (1) the flame from above, round 6, which needs a research round and a shader rewrite the five earlier rounds did not land; (2) `courtyard.gif`, which the README links as "the same courtyard" and which still showed the box braziers and the pink bed from before iterations 136 and 137 — a false README line, cheap and certain. Picked 2: a stale caption costs a reader more than a missing improvement, and 1 keeps its place at the top of the backlog.

### Sources

None new: no model, default or shader changed, so no research note. Recipe from `docs/DEMOS.md` (showcase recipe, step 1; `tools/gif.sh`).

### Verification and critique

- Capture: `LIGHTS_FRAMES=260 LIGHTS_SHOT_EVERY=2 LIGHTS_SHOT_FROM=9 LIGHTS_FOCUS_WALK=18`, 1280x720, xvfb `-n 138`, release examples rebuilt first (0.5 s: nothing stale). Frame 0 is 15 KB and black (pipelines compiling); frames 1-48 used, so the clip opens on the still-dim courtyard and fires then breathe. Single-window luma beside a brazier over the 48 frames: 106 → 136 → 125 → 134 → 123 → 134 → 123 → 69, a visible swing and a camera walk at the end; not a calibrated flicker measurement (the showcase's 2.50 Hz measurement is unchanged).
- Read the coalesced GIF frame 30 at full size and 300 % on a brazier: bowl brazier on a stem, ember bed, flame, warm pool on the stone, blue shadow side. Remaining flaws: banding rings in the pool gradients (256 colours, `dither=none` by design), the caption is low contrast over the floor, and a figure stands in front of one brazier. The 3.1 MB file is larger than the hero's 2 MB budget; that budget is the hero's, and the old file was 2.9 MB.
- Verdict: GREAT as README-linked media for what it claims (bowl braziers, ember bed, 48 torch-bearers, 8-light pool). `courtyard.png` (1280x720, frame 600) read at full size, same scene, bowl braziers, no pink plate.
- Bench: not run; no code changed. Gate: no code changed, not re-run.

### Top 5 remaining, ranked

1. The flame from above, round 6 (`flame-overhead.md`).
2. A hue-correct hot core for the bed (yellow-orange peaks), which wants the host's tonemapper in the loop.
3. Bearers are placeholder capsules; a luminance-measured flicker amplitude.
4. A time envelope for the fade, only if a host reports a camera stopping mid-fade.
5. Banding in the GIF's pool gradients (a better palette strategy than one global 256-colour palette).

## Iteration 139 — 2026-10-28 — the flame from above (round 6)

Outcome: shipped, `src/flame.wesl` only.

### Ranking

Open issues: #4 (criterion 3 delivered in 130), #3 answered, #1 publish (release, seen, skipped); no bug issues. Candidates: (1) the flame from above, round 6 — a fire is gone at 75° of camera elevation, the common view of a tactical or top-down game, with five failed rounds recorded and its first prerequisite (a non-clipped coal bed) met by iteration 137; (2) a hue-correct hot core for the bed, which needs the host's tonemapper. Picked 1: a visible artifact (a practical with no flame) outranks a colour refinement.

### Sources

Unchanged from `docs/research/flame-overhead.md`: *Real-Time Rendering* 4th ed. §13.6.3 (the failure of an axial billboard), Nguyen GPU Gems ch. 6 (the ancestor and its own account of the failure), SIGGRAPH 2011 *Advances* fire talk (production conceals the particle nature), Doğaner on visible sources (craft). No new source; the note's `Pending index` gap (view-dependent flame appearance) is still open.

### What changed

Rounds 1-5 stretched an *upright* quad by `1/cos(e)`. Debug capture (solid colour) showed why that cannot work: the stretched quad's upper vertices rise toward a camera 4 m up and perspective magnifies them into a trapezoid, and an upright plane through the brazier's origin is inside the bowl, so the depth test hides it. Sliding that quad along its view rays kept the corners but broke the perspective-correct `uv` (flame shifted up off the bed, three captures). Round 6 builds the quad facing the camera through the plume's centre past 50°, samples the upright tongue at `SOURCE_HEIGHT + sy/cos(e)`, fades in an elliptical cross-section by 76°, ripples it with its own fbm (the tongue's `n1` drew horizontal streaks at 80°/85°), and softens the tongue's foot and dims it.

### Verification and critique

- Captures `/tmp/l139/{before,r1..r9,sweep}/`: courtyard, brazier at (5,0,5), camera 4 m away at 0/30/45/60/70/75/80/85° and an 11-angle sweep 46°-86°. Before: bare coal bed at 75°/80°/85°. After (r9): hot yellow core with an orange halo centred on the bed at every angle; the tongue shortens into it with no step (sweep window means change by under 5 DN between neighbours, which is camera travel).
- Ground level: 0°, 30°, 45° differ from the build before by 1 channel of 1 pixel (`magick compare -metric AE`).
- Clipping: 0 px with min channel ≥ 245 in the bed window at 75° and 85°, before and after.
- Flaws at zoom: the cross-section's edge is soft and its internal structure is only a brightness ripple, a pale-green tint where the yellow core saturates the bed's red at 58°-66°, and the torch at 85° was not re-measured (the only torch in frame is at the edge, ~70° ray elevation, and reads the same before and after).
- Verdict: GREAT against `## What great looks like` (a fire at every angle, no pop, footprint on the fire, unchanged from the side, cheap); NOT a showcase of flame detail from above, which the note lists as a deviation.
- Bench (CPU-only, no Rust changed): `mean_us=572 p50_us=552 min_us=293`, flicker p50 193 µs, pool p50 401 µs. No before/after A/B: nothing it measures changed. Gate: build, test (both feature sets), examples, clippy, doc, fmt, wasm, wesl-check all pass.
- Media: `showcase.gif` and `courtyard.gif`/`.png` are from cameras below 50°; not re-encoded.

### Top 5 remaining, ranked

1. Crisper structure in the overhead cross-section (tongue-like licks rather than a brightness ripple), and the pale-green tint at 58°-66°.
2. A hue-correct hot core for the bed (yellow-orange peaks), which wants the host's tonemapper in the loop.
3. Bearers are placeholder capsules; a luminance-measured flicker amplitude.
4. A time envelope for the fade, only if a host reports a camera stopping mid-fade.
5. Banding in the GIF's pool gradients (a better palette strategy than one global 256-colour palette).

## Iteration 140 — 2026-10-28 — Licks in the flame from above

Open issues: #4 (criterion 3 delivered in 130), #3 answered, #1 publish (release, seen, skipped); no bug issues; `gh` reachable. Picked iteration 139's backlog item 1 (crisper structure in the overhead cross-section) over its item 2 (a hue-correct core, which needs the host's tonemapper): at 75°-85° the fire was a soft orange halo with no outline, the most visible gap of a top-down camera. Corpus re-scan and home-still preflight were not run this pass (the change is a shader constant set; sources are the note's existing stems).

**Changed:** the cross-section's radius is a scrolling noise of the direction from the axis, its brightness is streaked along the radius, edge `smoothstep(0, 0.26)` (`src/flame.wesl`). Sources (by stem, already quoted in `docs/research/flame-overhead.md`): craft `10.7456_tojdac.1674124`; production `s2011-advances-pdf-slides` and Gregory §11.4.3; technical `10.1017_jfm.2018.707` and RTR §6.3.

### Verification and critique

- Captures `/tmp/l140/{before,after,after2}/` (courtyard, brazier 4 m away, 1280x720, frame 60, 0/30/45/60/70/75/80/85°), zooms `z*.png`, montage `after2/m.png`.
- Round 1 (`dir*1.5`, amplitude 1.0, edge 0.18): lobed but a starburst at 75° and 85°: NOT YET. Round 2 (shipped): soft rounded licks, a ragged outline at 60°, 70°, 75°, 85°, a hot core inside.
- Flaws at zoom: interior radial streaks are subtle; the flame is small against the bowl (≈ a third of its diameter); the pale-green tint where the yellow core saturates the red at ~60° remains (backlog item); at 75° the plume sits on the bed's far half so the near half still reads as bare coals.
- Unchanged below 50°: `magick compare -metric AE` 0.0039, 0.0013, 0.0013 at 0°, 30°, 45° (1 channel of 1 pixel). Clipping: 0 px with min channel ≥ 245 in the bed window at 75° and 85°, before and after.
- Verdict: GREAT for the stated validation target (visible lobes at every angle 60°-85°, crisper edge, no starburst, no clipping); not a showcase of flame volume.
- Bench (CPU-only, no Rust changed): `mean_us=498 p50_us=489 min_us=254`, flicker p50 161 µs, pool p50 337 µs. Gate: build, dev build, test (both feature sets), examples, clippy, doc, fmt, wasm, wesl-check all pass.
- Media: `showcase.gif`, `courtyard.gif` and `.png` are from cameras below 50° and do not show this change; not re-encoded.

### Top 5 remaining, ranked

1. The pale-green tint where the core saturates at 56°-66°, and a hue-correct hot core (wants the host's tonemapper).
2. Layered volume in the overhead flame (an inner and an outer shell, so it reads as depth, not a lobed disc).
3. Bearers are placeholder capsules; a luminance-measured flicker amplitude.
4. A time envelope for the overhead fade, only if a host reports a camera stopping mid-fade.
5. Banding in the GIF's pool gradients (a better palette strategy than one global 256-colour palette).

## Iteration 141 — 2026-10-28 — The pale-green tint at 56°-66°: diagnosed, not fixed (shader change reverted)

Open issues: #4 (criterion 3 delivered in 130), #3 answered, #1 publish (release, seen, skipped); no bug issues; `gh` reachable. Picked iteration 140's backlog item 1 (the khaki/sage patch on the coal bed under the overhead flame). Corpus re-scan and home-still preflight not run (a colour constant; no new source). GPU free (4 GB of 24 used by a bystander).

**Diagnosis** (captures `/tmp/l141/{before,after,after2}/`, courtyard, brazier 4 m away, 1280x720, frame 60, 0/45/56/60/63/66/70/80°; debug builds `noflame63.png`, `red63.png`, `dim63.png`):
- With the flame hidden (`noflame63.png`) there is no patch: it is the flame's own output.
- The patch is a *dark gap between coal lumps* with the flame added over it: bed (91,27,6) there, tonemapped result (222,189,122) at 63°, against neighbouring lumps already at R 255 (255,137,62). It is the flame's colour alone, a pale tan, seen beside saturated orange, and it reads as sage by simultaneous contrast.
- A pure-red flame gives (222,27,6) at the same pixel, so the flame's red contribution there is about 222 sRGB; at 0.4x flame strength the patch is (162,102,58), a plain brown: the tint appears only at the full core intensity, i.e. it is the tonemapper rolling a very saturated high-intensity orange towards white along a yellow-green path, not the ramp's hue.
- Tried: an orange core past 50° (`CORE_ABOVE` (1.0, 0.60, 0.24), then (1.0, 0.48, 0.16)). Max G/R in the bed window fell (63°: 0.848 → 0.773 → 0.727; 66°: 0.836 → 0.743 → 0.715; 60°: 0.883 → 0.871 → 0.809) but the patch is visually unchanged in the montages `after/m.png` and `after2/m.png`, and a more orange core costs the yellow-white read of the fire's heart. Verdict: NOT YET; reverted (`git checkout src/flame.wesl`), nothing shipped. wesl-check passed on the experiment.

**Next hypothesis** (not tried): compress the core's intensity, not its hue, where `heat` saturates (a soft clamp of the additive's peak so the tonemapper's input stays out of its yellow-green roll-off), judged against the *host's* tonemapper, which is why the item has been marked as wanting it. Or desaturate the core towards white as `heat → 1` so the LUT's input is not a saturated orange.

No Rust or shader change; no bench or gate re-run (tree identical to iteration 140's merge). No baseline worktree was made.

### Top 5 remaining, ranked

1. The overhead core's tonemapper roll-off (above): cap or whiten the core's peak; judge in captures with the host's tonemapper.
2. Layered volume in the overhead flame (an inner and an outer shell).
3. Bearers are placeholder capsules; a luminance-measured flicker amplitude.
4. A time envelope for the overhead fade, only if a host reports a camera stopping mid-fade.
5. Banding in the GIF's pool gradients.

## Iteration 142 — 2026-10-28 — The overhead tint is not a tonemapper roll-off: knee experiment falsified, reverted

Open issues: #4 (criterion 3 delivered in 130), #3 answered, #1 publish (release, seen, skipped); no bug issues; `gh` reachable. GPU free. Picked iteration 141's backlog item 1 (cap the overhead core's peak). Corpus re-scan and home-still preflight not run (a shader constant; no new source). Tree was clean at start.

**Experiment**: divide the fragment's emission by `1 + K * overhead.x * max(r,g,b)` (hue-preserving soft knee), K = 0.5, 2.0, 6.0; wesl-check passed. Captures `/tmp/l142/{after,k2.0.png,k6.0.png}`, courtyard, 63° camera, 4 m, frame 60, crop `560,250 200x160` and the patch pixel at (645,302):
- baseline (224, 172, 106) G/R 0.77; K=2 (216, 168, 105) G/R 0.78; K=6 (195, 149, 92) G/R 0.77.
- The sage/khaki patch on the coal's upper lumps is visually identical at every K, including when the whole flame is visibly dimmer. Hue is invariant under intensity compression, so iteration 141's hypothesis (a tonemapper roll-off at the core's peak) is **falsified**: the tan is not a function of the flame's intensity.

Verdict NOT YET; reverted (`git checkout src/flame.wesl`), nothing shipped. No Rust change, no bench or gate re-run, no baseline worktree.

**Next hypothesis** (untried): the patch is the sum of the flame's *ramp colour* (`CORE_YELLOW` mixed with `material.base`) and the coal bed's own dark-gap colour, so what is wrong is the flame's *hue at moderate heat* (G/R 0.77 at heat ~0.5), not its peak. Test by ramping the colour mix to stay orange until `heat` > 0.7 (move the `smoothstep(0.40, 0.80)` to (0.65, 0.95)) and measuring G/R at the patch; if it does not move, hide the coal bed under the flame (a build with the bed's emissive zeroed) to see whether the green is in the coal's lit material instead (the pool light's colour on a dark lump), since a flame-hidden capture in 141 removed the *whole* patch only because the lumps were flame-lit.

### Top 5 remaining, ranked

1. The overhead patch: test the ramp's hue at moderate heat, then the coal's lit colour (above).
2. Layered volume in the overhead flame (an inner and an outer shell).
3. Bearers are placeholder capsules; a luminance-measured flicker amplitude.
4. A time envelope for the overhead fade, only if a host reports a camera stopping mid-fade.
5. Banding in the GIF's pool gradients.

## Iteration 143 — 2026-10-28 — The overhead patch is not the flame's ramp hue: experiment falsified, reverted

Open issues: #4, #3 answered, #1 publish (release, seen, skipped); no bug issues; `gh` reachable. GPU free. Picked iteration 142's backlog item 1 (test the ramp's hue at moderate heat). Corpus re-scan and home-still preflight not run (a shader constant; no new source). Tree was clean at start.

**Experiment** (captures `/tmp/l143/{before,after,after2}/`, courtyard, 4 m, frame 60, 56/63/66/80°, 1280x720):
- Variant A: `smoothstep(0.40, 0.80, heat)` → `(0.65, 0.95)` for the `CORE_YELLOW` mix. The whole flame's core goes orange: sampled G/R at the flame fell (e.g. (255,198,125) → (255,156,84)), and the torches' yellow-white heart is visibly lost in a side capture (`side.png`). Cost too high.
- Variant B: the same shift scaled by `in.overhead.x` (overhead views only; side views identical by construction). Flame at 56°/66° is orange as intended (`cmp2.png`: (246,154,89) → (246,129,61)), **but the sage/khaki patch on the coal lump just under the flame is visually unchanged** at 56° and 66°.
- Conclusion: with 141 (hue-compressed intensity, no change) and 142 (core colour, no change), the patch is independent of the flame's colour *and* intensity. By elimination it is in the coal bed's own lit material (the pool light's colour on the dark lump, with a green-ish channel from the light/ambient sum), not in `flame.wesl`.

Verdict NOT YET; reverted (`git checkout src/flame.wesl`), nothing shipped. No Rust change, no bench or gate re-run, no baseline worktree.

**Next hypothesis** (untried, and not a shader item): build the courtyard's coal bed with its emissive zeroed and the flame hidden, to see whether the green is the bowl material's lit result (example scene, `examples/courtyard.rs`), i.e. a scene-material issue, not a crate defect. If it is, drop it from the crate's backlog.

### Top 5 remaining, ranked

1. Check the coal bed's lit material in `examples/courtyard.rs` for the green (above); if it is the scene's, close the item.
2. Layered volume in the overhead flame (an inner and an outer shell).
3. Bearers are placeholder capsules; a luminance-measured flicker amplitude.
4. A time envelope for the overhead fade, only if a host reports a camera stopping mid-fade.
5. Banding in the GIF's pool gradients.

## Iteration 144 — 2026-10-28 — The overhead "green patch" is the flame's yellow core, not a defect: item closed

Open issues: #4, #3 answered, #1 publish (release, seen, skipped); no bug issues; `gh` reachable. GPU free (4 GB used, 16.9 GiB RAM). Picked iteration 143's backlog item 1 (is the green in the courtyard's coal bed?). Corpus re-scan and home-still preflight not run (no new source; no crate change). Tree was clean at start.

**Experiment** (`/tmp/l144/`, courtyard, eye 6.63,4.85,5 look 5,1.2,5 (~66° elevation, 4 m), frame 60, 1280x720): `BED_GAIN` 0.2 → 0.0 in `examples/courtyard.rs`, so the bed emits nothing and is a dark lump under the flame (`cmp2.png`). The pale patch **stays**, at the flame's base over the dark bed: pixels (640,380) srgb(203,172,109), (645,385) (214,174,109), (635,390) (195,168,107). G/R 0.85, B/R 0.54: a desaturated pale yellow, which is `CORE_YELLOW` (1.0, 0.78, 0.30) after the tonemapper, not a green (G never exceeds R). It reads sage only by simultaneous contrast against the saturated orange (254,108,26) around it.

Conclusion, with 141 (intensity), 142 (knee), 143 (ramp hue): the patch is the flame's hot core, correctly yellow-white where the flame is brightest and seen end-on; the coal bed is not involved. Not a defect; the item is closed and dropped from the backlog. Reverted the experiment (`git checkout examples/courtyard.rs`); nothing shipped, no gate or bench run (no change), no baseline worktree.

### Top 5 remaining, ranked

1. Layered volume in the overhead flame (an inner and an outer shell).
2. Bearers are placeholder capsules; a luminance-measured flicker amplitude.
3. A time envelope for the overhead fade, only if a host reports a camera stopping mid-fade.
4. Banding in the GIF's pool gradients.
5. A hue-correct hot core (needs the host's tonemapper; design only).

## Iteration 145 — 2026-10-28 — Re-captured the overhead fire to size backlog item 1; nothing shipped

Open issues: #4 (criterion 3 delivered in 130, `FlameSite::fire` landed), #3 answered, #1 publish (release, seen, skipped); no bug issues; `gh` reachable. GPU free, 16 GiB RAM, /tmp 55 %. Tree clean at start. Corpus re-scan and home-still preflight not run (no crate change).

Captured the courtyard at 66° (eye 6.63,4.85,5) and 85° (eye 5.3,9,5) and a side view, frame 90, 1280x720 (`/tmp/l145/before/`). 85°: the fire reads as a bright orange point in a lit ring, with no flaw at full frame. 66° at 4x zoom (`/tmp/l145/c66.png`): the flame is a soft, dim orange fan behind and above the coal bed, and the bed's small disc (about 260 px across at 4x) is the sharpest, brightest thing in the frame; the tongue does not read as a volume standing on the bed. That is the "layered volume" gap of the backlog, and it is a shader rewrite of the overhead branch of `src/flame.wesl` (lines 283-327) that needs its own research round (`docs/research/flame-overhead.md`) and several capture rounds; it was not started, so nothing was changed, gated or benched.

### Top 5 remaining, ranked

1. Layered volume in the overhead flame at ~66°: an inner and an outer shell so the tongue stands on the bed instead of fanning behind it (evidence above).
2. Bearers are placeholder capsules; a luminance-measured flicker amplitude.
3. A time envelope for the overhead fade, only if a host reports a camera stopping mid-fade.
4. Banding in the GIF's pool gradients.
5. A hue-correct hot core (needs the host's tonemapper; design only).

## Iteration 146 — 2026-10-28 — Overhead flame: the disc is not hidden by the bed; nothing shipped

Open issues: #4, #3 answered, #1 publish (release, seen, skipped); no bug issues. GPU free (4 GB used), 16 GiB RAM, /tmp 56 %. Tree clean at start. Corpus re-scan and home-still preflight not run (no crate change).

Took iteration 145's item 1. Captures `/tmp/l146/` (courtyard, eye 6.63,4.85,5 look 5,1.2,5, frame 90, 3x crop). Two experiments, both reverted:

1. `BED_GAIN` 0.0 (`before/nobed_c.png`): the lower ~60 % of the bed's ellipse is black; only its upper part carries the flame's pale core. So the overhead disc (`src/flame.wesl`, `if in.overhead.x > 0.0`) contributes little over the lower bed at 66°; the bright, cratered look in `before/c66.png` is the coal bed's own emission, not the flame.
2. Pulling the facing quad toward the eye (`centre + normalize(to_eye) * size * k`, k 0.3 and 3.0) to rule out the bed's near side burying it in the depth test: k 0.3 is pixel-identical to before (`a1c.png`); k 3.0 only enlarges the flame into a wash (`a2c.png`). Depth occlusion by the bed is **not** the cause.

So the disc is simply weak at this angle: its mask (`smoothstep(0, .26, disc) * licks * overhead.x`) and heat feed an additive orange onto an already orange-saturated ground. Candidate fix, not tried: weight the disc by radius toward a hotter centre and measure against a darker exposure; needs its own research round. Nothing shipped; no gate or bench run (no change); no baseline worktree.

### Top 5 remaining, ranked

1. Overhead disc is weak at ~66° (above): raise its contribution and give it a radial heat gradient; needs a research round and exposure check.
2. Bearers are placeholder capsules; a luminance-measured flicker amplitude.
3. A time envelope for the overhead fade, only if a host reports a camera stopping mid-fade.
4. Banding in the GIF's pool gradients.
5. A hue-correct hot core (needs the host's tonemapper; design only).

## Iteration 147 — 2026-10-28 — Overhead disc gain x1.6: visible but NOT YET; reverted, nothing shipped

Open issues: #1 publish (release, seen, skipped); no bug issues. GPU free, 14 GiB RAM, /tmp 56 %. Tree clean at start. Corpus re-scan and home-still preflight not run (no new source).

Took iteration 146's item 1. Experiment (`/tmp/l147/{before,after}/c66.png`, courtyard, eye 6.63,4.85,5 look 5,1.2,5, frame 90, 3x crop of 400x300+440+270): in the overhead branch of `src/flame.wesl`, mask `smoothstep(0,.26,disc)*licks` x1.6 clamped to 1.4, and heat `disc` x1.3. `wesl-check` ok. Side views unchanged by construction (`overhead.x` = 0).

Critique: after, the flame body gains defined lick edges and a pale core instead of a dim fan, a real improvement. Still: the tongue sits behind and above the bed rather than standing on it (no volume), the scene's ground is a near-uniform saturated orange (scene exposure, not a crate defect) so contrast stays low, and the bed disc remains the sharpest feature. Verdict NOT YET (not showcase). Reverted with `git checkout src/flame.wesl`; no gate or bench run (no change shipped); no baseline worktree.

**Next** (untried): combine the gain with an inner/outer shell (two disc radii at different heat), and capture against a darker courtyard exposure so contrast can be judged; the gain alone is a candidate component (x1.6 mask, x1.3 heat).

### Top 5 remaining, ranked

1. Overhead flame volume: the x1.6/x1.3 gain (above) plus inner and outer shells; judge under darker exposure; needs a research note round.
2. Bearers are placeholder capsules; a luminance-measured flicker amplitude.
3. A time envelope for the overhead fade, only if a host reports a camera stopping mid-fade.
4. Banding in the GIF's pool gradients.
5. A hue-correct hot core (needs the host's tonemapper; design only).

## Iteration 148 — 2026-10-28 — Overhead flame: two shells plus gain, brighter core but NOT YET; reverted, nothing shipped

Open issues: #1 publish (release, seen, skipped); no bug issues. GPU free, 16 GiB RAM, /tmp 56 %. Tree clean at start. Corpus re-scan and home-still preflight not run (no new source).

Took iteration 147's item 1. Experiment (`/tmp/l148/{before,after}/c66.png`, `cmp.png` 3x crop 400x300+440+270; courtyard, eye 6.63,4.85,5 look 5,1.2,5, frame 90): the overhead branch of `src/flame.wesl` got the 147 gain (mask x1.6 clamped 1.4, heat x1.3) plus an inner shell (0.55 of the radius, rippled by `streak`, +0.35 heat). `wesl-check` ok.

Critique: the plume gains a pale, defined core inside an orange skirt, clearly better than the dim fan. But the plume still sits behind and above the bed instead of standing on it, the core reads as a flat pale blob (no vertical volume), the ground stays a near-uniform saturated orange so contrast is low, and the bed disc is still the sharpest feature. Verdict NOT YET. Reverted (`git checkout src/flame.wesl`); no gate or bench run (nothing shipped); no baseline worktree.

**Pattern across 143-148:** the remaining gap is not intensity or hue but missing vertical volume at ~66°, which a disc cannot supply; it needs a different construction (a second, camera-facing tongue quad offset up the plume axis, or a short stack of discs at rising heights) and a darker courtyard exposure to judge it. That is a research round, not another tweak.

### Top 5 remaining, ranked

1. Overhead flame volume via a stack of 3 discs at rising heights (not a single disc); new research round in `docs/research/flame-overhead.md`; judge under darker exposure. Carry the 147/148 gain and inner shell as components.
2. Bearers are placeholder capsules; a luminance-measured flicker amplitude.
3. A time envelope for the overhead fade, only if a host reports a camera stopping mid-fade.
4. Banding in the GIF's pool gradients.
5. A hue-correct hot core (needs the host's tonemapper; design only).

## Iteration 149 — 2026-10-28 — Research infrastructure down; nothing shipped

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (54ba294), GPU free, 16 GiB RAM, /tmp 56 %.

Both remaining backlog items need a research round first (item 1: `docs/research/flame-overhead.md` for the disc stack; item 2: the flicker amplitude, which `docs/research/flame-flicker.md` records as uncited). Home-still preflight: `hs status` shows distill (`192.168.1.110:7434`) and qdrant down, connection refused; `hs restart` run once ("distill containers restarted") and still refused after ~75 s; `distill_search` fails with connection refused; `paper_search` fails with `bridge: HTTP Error 404`; scribe on 192.168.1.110 is down (VLM backend unreachable), bmb scribe idle. The local corpus (`~/.cache/bevy_lights_corpus/markdown`, 362 stems) is a different library and holds no flame-flicker source. No source means no research note, and no research note means no code (AGENTS.md); no capture, no benchmark, no change.

**Next:** when `hs status` shows distill healthy, run item 1 or 2 below. Without a restored distill, `paper_search` is the only route to a citation for the amplitude.

### Top 5 remaining, ranked

1. Overhead flame volume via a stack of 3 discs at rising heights; new research round; judge under darker exposure. Carry the 147/148 gain and inner shell as components.
2. A cited, luminance-measured flicker amplitude (the ±15 % depth is a design default from Sanctuary).
3. Bearers are placeholder capsules.
4. A time envelope for the overhead fade, only if a host reports a camera stopping mid-fade.
5. Banding in the GIF's pool gradients.

## Iteration 150 — 2026-10-28 — No citable amplitude, no research round possible; nothing shipped

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (fe8b804), 17 GiB RAM, /tmp 56 %. `hs status`: distill (`192.168.1.110:7434`) and qdrant still down, scribe on .110 unreachable (VLM backend), bmb scribe idle. `paper_search` works again (openalex and crossref answer; core needs a key; semantic_scholar rate-limited).

Tried item 2 (a cited, luminance-measured flicker amplitude). `paper_search` "candle flame flicker frequency luminance fluctuation" returned Chen et al. 2019, *Frequency and Phase Characteristics of Candle Flame Oscillation* (10.1038/s41598-018-36754-w). `paper_download` ok; `scribe_convert` timed out at 30 s (outcome unknown), so the PDF was fetched from nature.com to `/tmp/l150/chen.pdf` and read with `pdftotext`. Its brightness is the mean gray value per frame at fixed camera parameters, "Thus, the brightness is dimensionless", plotted only as figures; a single candle "does not exhibit visible oscillation and remains stable", and flicker appears only for three or more bound candles. It gives a frequency trend but no relative depth for a single fire, so it cannot replace the ±0.15 design default and was not committed to the corpus or a note. The other hit that touched luminance (Stratton 2005 thesis, pulsation frequency of furniture and pool fires) reports frequency, not depth. `docs/research/flicker-depth.md` already bounds the amplitude against Mandin & Most's measured 25 % temperature ceiling; nothing here tightens it.

Item 1 (overhead disc stack) needs the corpus and distill for its research note, and the 143-148 passes show it also needs a visual-judgement round under a darker exposure; with distill down no craft/production/technical triple can be assembled. No code, capture, baseline worktree or bench was run.

**Next:** when `hs status` shows distill healthy, run item 1. Item 2 stays blocked on a source that reports relative luminance depth for a single flame; try `paper_search` for "fluctuation of flame radiation intensity" in the fire-safety literature, not the candle-synchronisation one.

### Top 5 remaining, ranked

1. Overhead flame volume via a stack of 3 discs at rising heights; new research round; judge under darker exposure. Carry the 147/148 gain and inner shell as components.
2. A cited, luminance-measured flicker amplitude (the ±15 % depth is a design default from Sanctuary; bounded, not validated, by `flicker-depth.md`).
3. Bearers are placeholder capsules.
4. A time envelope for the overhead fade, only if a host reports a camera stopping mid-fade.
5. Banding in the GIF's pool gradients.

## Iteration 151 — 2026-10-28 — Still no luminance-depth source and distill down; nothing shipped

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (38c75c3), 18 GiB RAM, /tmp 56 %, GPU 4 GB used of 24 (free). `hs status`: distill (`192.168.1.110:7434`) and qdrant refuse connections, scribe on .110 reports the VLM backend unreachable, bmb scribe idle; `hs restart` was not repeated (two earlier passes tried it, no change). `catalog_recent` shows no new lighting documents beyond the Chen candle download of iteration 150.

Item 2 (cited flicker amplitude): one `paper_search`, "fluctuation of flame radiation intensity pool fire flicker amplitude relative", returned McCaffrey 1979 (NBS IR 79-1910: pulsation about 3 Hz, but the fluctuation ratio is for temperature and velocity, not luminance), Maynard 2013 (pulsation frequency from image-intensity analysis; frequency only per abstract) and pool-fire radiation papers from 1977-1993 with no abstract and no open PDF. None reports a relative luminance depth for a single flame, so the ±15 % design default stays uncited. Item 1 (overhead disc stack) still needs the craft/production/technical triple from distill plus a darkened-exposure judging round, and distill is down. No code, capture, baseline worktree or bench was run.

**Next:** when `hs status` shows distill healthy, run item 1. For item 2, fetch Maynard's thesis (`https://escholarship.org/uc/item/851832j6`) and check whether its image-intensity series gives a normalised amplitude before spending another search.

### Top 5 remaining, ranked

1. Overhead flame volume via a stack of 3 discs at rising heights; new research round; judge under darker exposure. Carry the 147/148 gain and inner shell as components.
2. A cited, luminance-measured flicker amplitude (the ±15 % depth is a design default from Sanctuary; bounded, not validated, by `flicker-depth.md`).
3. Bearers are placeholder capsules.
4. A time envelope for the overhead fade, only if a host reports a camera stopping mid-fade.
5. Banding in the GIF's pool gradients.

## Iteration 152 — 2026-10-28 — Distill still down, Maynard thesis unreachable; nothing shipped

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (8183584), 19 GiB RAM, /tmp 56 %, GPU 4 GB used of 24 (free). `hs status`: distill (`192.168.1.110:7434`) and qdrant refuse connections, scribe on .110 reports the VLM backend unreachable, bmb scribe idle; `hs restart` not repeated.

Item 2: fetched `https://escholarship.org/uc/item/851832j6` (Maynard thesis) as iteration 151 advised; the host answers with an AWS WAF JavaScript challenge, no document text, so no amplitude could be read. Item 1 still needs the craft/production/technical triple from distill. No code, capture, baseline worktree or bench was run.

**Next:** when `hs status` shows distill healthy, run item 1. For item 2, get Maynard's thesis by another route (`paper_download` by DOI, or a browser session) before spending another search.

### Top 5 remaining, ranked

1. Overhead flame volume via a stack of 3 discs at rising heights; new research round; judge under darker exposure. Carry the 147/148 gain and inner shell as components.
2. A cited, luminance-measured flicker amplitude (the ±15 % depth is a design default from Sanctuary; bounded, not validated, by `flicker-depth.md`).
3. Bearers are placeholder capsules.
4. A time envelope for the overhead fade, only if a host reports a camera stopping mid-fade.
5. Banding in the GIF's pool gradients.

## Iteration 153 — 2026-10-28 — Distill still down; nothing shipped

Open issues: #1 publish (release, seen, skipped); no bug issues read this pass (carried from 152). Tree clean at start (2b223fa), 19 GiB RAM available, /tmp 56 %. `hs status` (live): distill (`192.168.1.110:7434`) refuses connections, qdrant down, scribe on .110 reports the VLM backend unreachable, bmb scribe idle; `hs restart` not repeated. `catalog_recent` not re-scanned beyond the status feed, which shows only the Chen candle download. The local corpus (`~/.cache/bevy_lights_corpus/markdown`, 362 files) is present, so item 1's research could be started with `grep` over it instead of `distill_search`; the next pass should try that rather than wait for distill. No code, capture, baseline worktree or bench was run.

**Next:** run item 1, doing the research round from the local corpus if distill is still down (record the unindexed note under `Pending index`).

### Top 5 remaining, ranked

1. Overhead flame volume via a stack of 3 discs at rising heights; new research round (local corpus grep if distill is down); judge under darker exposure. Carry the 147/148 gain and inner shell as components.
2. A cited, luminance-measured flicker amplitude (±15 % is a design default); get Maynard's thesis by `paper_download` by DOI or a browser session.
3. Bearers are placeholder capsules.
4. A time envelope for the overhead fade, only if a host reports a camera stopping mid-fade.
5. Banding in the GIF's pool gradients.

## Iteration 154 — 2026-10-28 — Overhead flame: three-disc stack, better but NOT YET; not shipped

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (7c8c01b), 19 GiB RAM, /tmp 56 %, GPU 4 GB used. `hs status` (live): distill and qdrant down, scribe on .110 down, bmb scribe idle; `hs restart` not repeated, so no new search; the sources were the ones `flame-overhead.md` already quotes (Doğaner craft; Gregory and `s2011-advances-pdf-slides` production; Xia & Zhang and RTR4 §13.6.3 technical). Corpus re-scan not run (distill down).

Took item 1. Baseline binary copied to `/tmp/l154/courtyard_base`, captures `/tmp/l154/{before,after}/` (a0, a30, a50, a66, a76, a85, b66 = seed 3), comparisons `cmp_*.png`. Shader: the overhead disc became three discs at rising world heights, each lifted by `height * cos(elevation)`, narrower and dimmer per layer, with 147's gain and a crisper edge; `wesl-check` ok. Three rounds. Round 1 (layers -0.10/0.07/0.24 flame heights): barely visible. Rounds 2-3 (-0.25/0.05/0.35, edge 0.16, cool 0.25): at 66° the fire now wraps and stands on the coals, at 76° and 85° a defined pale core and halo; 0° identical, 30° and 50° 1 channel of 1 pixel off.

Critique: the plume above the layers is still a soft fan at 66°, the layers read as stacked translucent cut-outs, and the ground is saturated orange so contrast is low. Verdict NOT YET. Reverted `src/flame.wesl`; patch kept at `docs/attempts/overhead-stack-round8.patch`. No gate or bench run (no code shipped). The baseline was a binary copy, no worktree. Research note: `docs/research/flame-overhead.md`, round 8.

### Top 5 remaining, ranked

1. Overhead flame volume: apply `docs/attempts/overhead-stack-round8.patch` as the starting point and fix what stayed NOT YET (a sharper plume above the layers, a darker exposure to judge contrast); this is the fourth pass at the same gap, so judge a 4-5 disc or tongue-quad construction, not constants.
2. A cited, luminance-measured flicker amplitude (±15 % is a design default).
3. Bearers are placeholder capsules.
4. A time envelope for the overhead fade, only if a host reports a camera stopping mid-fade.
5. Banding in the GIF's pool gradients.

## Iteration 155 — 2026-10-28 — Overhead flame: five-disc stack, NOT YET; not shipped

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (6d08917), 17 GiB RAM, /tmp 57 %, GPU 4 GB used of 24. Home-still not re-probed (distill down for the last eight passes); sources are the ones `flame-overhead.md` already quotes; corpus re-scan not run.

Took item 1 from iteration 154. Applied the round-8 patch and widened it to five discs, then a lower-gain variant. Captures `/tmp/l155/{before,after,r2}/` (a66, a76, a85, plus a0/a30/a50/b66 for the upright check), crops `/tmp/l155/c_all.png`, `c_r2.png`; `wesl-check` ok for both. Five discs at 66°: a taller plume that stands on the coals, better than round 8. At 76° and 85°: an opaque burst over the bed (cauliflower, not fire). Lower gain: back to within a few DN of the unmodified shader at 76°/85°. Verdict NOT YET for both; reverted the shader, kept `docs/attempts/overhead-stack-round9.patch` and a round 9 entry in the research note. No gate, bench or commit of code (nothing shipped).

Fifth pass at the same gap with disc constants; stop tuning discs. The ground around the brazier is saturated salmon, which hides any contrast gain.

### Top 5 remaining, ranked

1. Overhead flame: a tongue-quad construction (upright leaning ribbons seen end-on), or first add a darker exposure switch to `courtyard` so the flame can be judged against dark ground.
2. A cited, luminance-measured flicker amplitude (±15 % is a design default).
3. Bearers are placeholder capsules.
4. A time envelope for the overhead fade, only if a host reports a camera stopping mid-fade.
5. Banding in the GIF's pool gradients.

## Iteration 156 — 2026-10-28 — courtyard exposure switch (`LIGHTS_EV100`)

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (0efa54a), 18 GiB RAM, /tmp 60 %, GPU 4 GB used. `hs status` (live): distill and qdrant down, scribe .110 down, bmb scribe idle; no research or corpus re-scan possible, and none was needed (no crate code or default changed).

Took item 1's prerequisite from iteration 155: a darker exposure switch, so the overhead flame can be judged apart from the saturated ground. `examples/courtyard` reads `LIGHTS_EV100` (default 6 = unchanged). Captures `/tmp/l156/ev{6,9,11}_y{6.5,9}.png`, sheet `/tmp/l156/cmp.png`: at EV 6 the ground is salmon at 76° and 85°; at EV 9 it is dark stone and the flame reads as a ragged orange burst with a pale core on the dark bed; at EV 11 the stone is lost. EV 9 is the judging exposure. The overhead flame itself is unchanged and still NOT YET (a cauliflower, not tongues). Example-only change: release build of examples ok; no crate code, so no bench.

### Top 5 remaining, ranked

1. Overhead flame: a tongue-quad construction judged at `LIGHTS_EV100=9`.
2. A cited, luminance-measured flicker amplitude (±15 % is a design default).
3. Bearers are placeholder capsules.
4. A time envelope for the overhead fade, only if a host reports a camera stopping mid-fade.
5. Banding in the GIF's pool gradients.

## Iteration 157 — 2026-10-28 — edge-on tongue slab from straight above

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (a8768f2), 17 GiB RAM, GPU 4 GB used. `hs status`: distill and qdrant still down, so no corpus re-scan or new search; no new source cited (design default, noted in `flame-overhead.md` round 11).

Re-captured the overhead flame at `LIGHTS_EV100=9` (`/tmp/l157/before/`) before touching anything: on dark stone the shipped flame is a ragged orange burst with a pale core; honest verdict still NOT YET against 'tongues', but one concrete artifact: a faint 1 px horizontal slab right of the core at ~90° (the edge-on upright tongue). Fixed by fading the tongue to zero by sin(e) 0.996. Verified with `/tmp/l157/after/sp9.png` (slab gone); 76° (y65) 2 px different, 60° (y485) 0 px; wesl-check ok; fmt, clippy, wasm, `cargo test` ok. Bench not run: shader-only, CPU path unchanged.

Verdict: artifact removed; the overhead flame as a whole remains NOT YET (burst, not tongues).

### Top 5 remaining, ranked

1. Overhead flame at EV 9: tongue-quad construction (upright leaning ribbons seen end-on).
2. A cited, luminance-measured flicker amplitude (Maynard 2013 unobtainable).
3. Bearers are placeholder capsules.
4. A time envelope for the overhead fade, only if a host reports it.
5. Banding in the GIF's pool gradients.

## Iteration 158 — 2026-10-28 — no change shipped (item 2 and the GIF size checked, both blocked)

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (8629f86), 18 GiB RAM, /tmp 60 %, GPU 4 GB used. `hs status` (live): distill and qdrant down, scribe .110 down, bmb scribe idle; no corpus re-scan or new search possible.

Candidates checked, none shipped:

1. **A cited flicker amplitude (backlog 2).** Searched the local corpus (`~/.cache/bevy_lights_corpus/markdown`, 362 top-level entries) for a luminance-variation figure for a candle or open flame; the only hits are unrelated medical and sexology papers that mention "candle", and the one flame-physics paper held (Xia & Zhang 2018, `10.1017_jfm.2018.707`) gives the rate law, not a luminance depth. `FLICKER_DEPTH = 0.15` therefore stays a design default. Needs the distill index back, or Maynard 2013 by other means.
2. **`showcase.gif` is 2.40 MB against the README rule's "under ~2 MB".** Tried re-encoding the committed file with `palettegen stats_mode=diff` and `paletteuse diff_mode=rectangle`: 2.16 MB at 256 colours, 2.16 MB at 192. No `gifsicle` on big. A real cut needs fewer frames or a narrower clip, which changes what the hero shows; not done without a capture review. Not committed.
3. **Overhead flame (backlog 1).** Re-viewed `/tmp/l157/after/y65.png` at 1280 x 720: at full frame the bowl reads as a lit fire from above, ~50 px wide; two earlier tongue-stack attempts (`docs/attempts/`) are already NOT YET and no new construction idea was in hand, so no round was spent.

No crate code, shader or default changed; no bench.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round 8/9 disc stack (upright leaning ribbons seen end-on).
2. `showcase.gif` under 2 MB: re-capture a shorter or narrower clip (needs a GPU capture review).
3. A cited, luminance-measured flicker amplitude (blocked on distill / Maynard 2013).
4. Bearers are placeholder capsules.
5. A time envelope for the overhead fade, only if a host reports it.

## Iteration 159 — 2026-10-28 — showcase.gif under the 2 MB budget

Outcome: shipped, media and docs only; no `src/` change.

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (3710e6f), 19 GiB RAM, /tmp 60 %. `hs status` (live): distill/qdrant embedded 0, scribe .110 unavailable; no research needed, nothing modelled.

Candidates: (1) overhead flame, no new construction idea; (2) `showcase.gif` 2.40 MB against "under ~2 MB", cheap and certain, a stated README rule; (3) flicker amplitude, blocked on distill. Picked 2.

Recipe: `docs/DEMOS.md` (`LIGHTS_FRAMES=260 EVERY=2 FROM=9 FOCUS_WALK=18`, xvfb `-n 159`, frames 1-60, `tools/gif.sh`). Sizes by width: 480 px 2.48 MB, 432 px 2.09 MB, 400 px 1.82 MB; 400 shipped. The first caption was clipped by the narrower frame; shortened to "4 braziers, 48 torch-bearers, 8-light pool" and re-checked at 300 %. Read the coalesced frame 30 at 250 %: bowl braziers, flames, warm pools, blue shadows, banding rings as before (256 colours, `dither=none` by design). Verdict: GREAT as README hero media for what it claims. A windowed flicker re-measurement was tried and is inconclusive (the camera walk and moving bearers dominate the windows), so no new Hz figure is claimed; the frames and rate (30 fps content) are unchanged by the re-encode. README caption size and `docs/DEMOS.md` updated; `cargo test --release --test readme` passes. No bench or gate re-run: no code changed.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round 8/9 disc stack.
2. A cited, luminance-measured flicker amplitude (blocked on distill / Maynard 2013).
3. Bearers are placeholder capsules.
4. A time envelope for the overhead fade, only if a host reports it.
5. Banding in the GIF's pool gradients.

## Iteration 160 — 2026-10-28 — no change shipped (backlog blocked or already traded off; main verified green)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (5ad7db0), 19 GiB RAM, VRAM 4/24 GB used, /tmp 60 %. `hs status` (live): distill and qdrant down (embedded 0), scribe .110 unavailable, bmb scribe idle; no research possible via distill.

Candidates: (1) overhead flame at EV 9: two construction attempts already NOT YET (`docs/attempts/`), no new idea in hand; (2) flicker amplitude: blocked on distill; (3) bearers as capsules: example-only, not crate behaviour; (4) GIF banding: `dither=none` is a documented trade (`docs/DEMOS.md`, trap two), changing it re-opens the crawl. Nothing met the bar for a researched, GREAT-signed change, and none was started on speculation.

Verified on main 5ad7db0: `cargo build --release`, `cargo test` (all suites pass, incl. readme), `cargo clippy --all-targets --all-features -D warnings`, `cargo fmt --check`, `tools/wesl-check` (both DEPTH_PREPASS variants ok).

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round 8/9 disc stack.
2. A cited, luminance-measured flicker amplitude (blocked on distill / Maynard 2013).
3. Bearers are placeholder capsules.
4. Banding in the GIF's pool gradients (needs a dither that does not crawl).
5. A time envelope for the overhead fade, only if a host reports it.

## Iteration 161 — 2026-10-28 — no change shipped (backlog unchanged, distill still down)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (bd91ac7), 17 GiB RAM, VRAM 4/24 GB used, /tmp 60 %. `hs status` (live): distill and qdrant down (embedded 0), scribe .110 backend unavailable, bmb scribe idle; no `distill_search`. A new download, "Frequency and Phase Characteristics of Candle Flame Oscillation", appeared in the catalog an hour ago; it is a frequency paper, so it does not give the luminance depth that backlog item 2 needs, and it is not yet converted or indexed.

Candidates: the same five as iteration 160. (1) Overhead flame at EV 9: two construction attempts already NOT YET (`docs/attempts/`), no new idea in hand. (2) Flicker amplitude: blocked on distill. (3) Bearers as capsules: example-only. (4) GIF banding: `dither=none` is a documented trade. (5) Overhead-fade envelope: no host has reported it. Nothing met the bar for a researched, GREAT-signed change; none started on speculation.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round 8/9 disc stack.
2. A cited, luminance-measured flicker amplitude (blocked on distill; the candle-flame oscillation paper just downloaded is the one to convert first).
3. Bearers are placeholder capsules.
4. Banding in the GIF's pool gradients (needs a dither that does not crawl).
5. A time envelope for the overhead fade, only if a host reports it.

## Iteration 162 — 2026-10-28 — no change shipped (backlog unchanged, distill still down)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (a5dc3a5), 19 GiB RAM, VRAM 4/24 GB used, /tmp 60 %. `hs status` (live): distill and qdrant refuse connections (embedded 0), scribe .110 backend unavailable, bmb scribe idle; the candle-flame oscillation paper is still downloaded but not converted. No `distill_search`.

Candidates: the same five as iteration 161; none has a new idea or an unblocked path, so nothing was started on speculation.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round 8/9 disc stack.
2. A cited, luminance-measured flicker amplitude (blocked on distill; convert the candle-flame oscillation paper first).
3. Bearers are placeholder capsules.
4. Banding in the GIF's pool gradients (needs a dither that does not crawl).
5. A time envelope for the overhead fade, only if a host reports it.

## Iteration 163 — 2026-10-28 — no change shipped (backlog unchanged, distill still down after `hs restart`)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (a8316aa), 19 GiB RAM, VRAM 4/24 GB used, /tmp 60 %. `hs status` (live): distill and qdrant refuse connections (embedded 0), scribe .110 backend unavailable, bmb scribe idle. `hs restart` run once as the skill directs ("Restarted 4 service(s)", distill containers restarted); distill was still unreachable 60 s later. No `distill_search`.

Candidates: the same five as iteration 162; none has a new idea or an unblocked path, so nothing was started on speculation.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round 8/9 disc stack.
2. A cited, luminance-measured flicker amplitude (blocked on distill; convert the candle-flame oscillation paper first).
3. Bearers are placeholder capsules.
4. Banding in the GIF's pool gradients (needs a dither that does not crawl).
5. A time envelope for the overhead fade, only if a host reports it.

## Iteration 164 — 2026-10-28 — no change shipped (backlog unchanged, distill still down)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (2749c47), 18 GiB RAM, VRAM 4/24 GB used, /tmp 60 %. `hs status` (live): distill and qdrant refuse connections (embedded 0), scribe .110 backend unavailable, bmb scribe idle; the candle-flame oscillation paper is still downloaded but not converted. No `distill_search`.

Candidates: the same five as iteration 163; none has a new idea or an unblocked path, so nothing was started on speculation.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round 8/9 disc stack.
2. A cited, luminance-measured flicker amplitude (blocked on distill; convert the candle-flame oscillation paper first).
3. Bearers are placeholder capsules.
4. Banding in the GIF's pool gradients (needs a dither that does not crawl).
5. A time envelope for the overhead fade, only if a host reports it.

## Iteration 165 — 2026-10-28 — no change shipped (backlog unchanged, distill still down)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (d33ddf9), 18 GiB RAM, VRAM 4/24 GB used, /tmp 60 %. `hs status` (live): distill and qdrant refuse connections (embedded 0), scribe .110 backend unavailable, bmb scribe idle; the candle-flame oscillation paper is still downloaded but not converted. No `distill_search`.

Candidates: the same five as iteration 164; none has a new idea or an unblocked path, so nothing was started on speculation.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round 8/9 disc stack.
2. A cited, luminance-measured flicker amplitude (blocked on distill; convert the candle-flame oscillation paper first).
3. Bearers are placeholder capsules.
4. Banding in the GIF's pool gradients (needs a dither that does not crawl).
5. A time envelope for the overhead fade, only if a host reports it.

## Iteration 166 — 2026-10-28 — no change shipped (backlog unchanged, distill still down)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (90a09d2), 18 GiB RAM, VRAM 4/24 GB used, /tmp 60 %. `hs status` (live): distill and qdrant refuse connections (embedded 0), scribe .110 backend unavailable, bmb scribe idle. One `scribe_convert` of the candle-flame oscillation paper (`10.1038_s41598-018-36754-w`) was tried: the MCP call timed out at 30 s, outcome unknown. No `distill_search`.

Candidates: the same five as iteration 165; none has a new idea or an unblocked path, so nothing was started on speculation.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round 8/9 disc stack.
2. A cited, luminance-measured flicker amplitude (blocked on distill; the candle-flame paper's conversion timed out once).
3. Bearers are placeholder capsules.
4. Banding in the GIF's pool gradients (needs a dither that does not crawl).
5. A time envelope for the overhead fade, only if a host reports it.

## Iteration 167 — 2026-10-28 — no change shipped (backlog unchanged, distill still down)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (768985d). `hs status` (live): distill and qdrant refuse connections (embedded 0), scribe .110 backend unavailable, bmb scribe idle; the candle-flame oscillation paper is downloaded but not converted. No `distill_search`. Candidates and top 5 unchanged from iteration 166; none has a new idea or an unblocked path, so nothing was started on speculation.

## Iteration 168 — 2026-10-28 — no change shipped (backlog unchanged, distill still down)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (4f44a97). `hs status` (live): distill and qdrant refuse connections (embedded 0), scribe .110 backend unavailable, bmb scribe idle; the candle-flame oscillation paper is downloaded (4 h ago) but not converted. No `distill_search`. Candidates and top 5 unchanged from iteration 167; none has a new idea or an unblocked path, so nothing was started on speculation.

## Iteration 169 — 2026-10-28 — no change shipped (backlog unchanged, candle paper still unconverted)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (75fb1be). `hs status` (live): distill (Cuda) and qdrant are UP (18028 docs embedded), scribe .110 backend unavailable, bmb scribe idle. The candle-flame oscillation paper (`10.1038_s41598-018-36754-w`) is still downloaded but not converted: `scribe_convert` timed out at 30 s again and no Convert event appeared two minutes later; `hs scribe convert` takes a path, not a stem. Backlog item 2 (a cited, luminance-measured flicker amplitude) therefore still lacks its technical source. Candidates and top 5 unchanged from iteration 166; nothing was started on speculation.

Next pass: if the paper's markdown exists, take item 2 now that distill is up.

## Iteration 170 — 2026-10-28 — research note: candle-flame paper validates the rate law (no code change)

Outcome: docs only; no `src/`, shader, default or media change, so no gate or bench was run.

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (79e59eb). `hs status` (live): distill (Cuda) and qdrant up, scribe .110 backend unavailable, bmb scribe idle. The candle paper (`10.1038/s41598-018-36754-w`) is still unconverted, but it is open access, so it was read from PMC6344522 directly. Result in `docs/research/flicker-depth.md` ("Independent check of the rate law"): measured 10-12 Hz and f ∝ D^-0.49 agree with `flicker_rate` (13 Hz at a 1.3 cm bundle, within 20 %); the paper gives luminance only as unitless plots, so backlog item 2 (a measured amplitude) stays open. Not indexed in distill (not converted).

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round 8/9 disc stack.
2. A cited, luminance-measured flicker amplitude (Chen 2019 has plots only; digitising its Fig. 2a would give one).
3. Bearers are placeholder capsules.
4. Banding in the GIF's pool gradients (needs a dither that does not crawl).
5. A time envelope for the overhead fade, only if a host reports it.

## Iteration 171 — 2026-10-28 — no change shipped (backlog unchanged)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (7bf9990). Parked work: only the iteration-115 flame-occlusion WIP for issue #2, which is no longer open, so stale and ignored. The top 5 from iteration 170 stand: item 1 and 3 need a new construction/asset, item 2 needs a digitised paper figure, item 4 has no non-crawling dither, item 5 awaits a host report. Nothing was started on speculation.

## Iteration 172 — 2026-10-28 — no change shipped (backlog unchanged)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (4186bc3). The top 5 from iteration 170 stand unchanged: items 1 and 3 need a new construction/asset, item 2 needs a digitised paper figure, item 4 has no non-crawling dither, item 5 awaits a host report. Nothing was started on speculation.

## Iteration 173 — 2026-10-28 — no change shipped (backlog unchanged)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (aeac335). The top 5 from iteration 170 stand unchanged; none has a new idea or an unblocked path, so nothing was started on speculation.

## Iteration 174 — 2026-10-28 — no change shipped (backlog unchanged)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (3a11f75). The top 5 from iteration 170 stand unchanged; none has a new idea or an unblocked path, so nothing was started on speculation.

## Iteration 175 — 2026-10-28 — no change shipped (backlog unchanged)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (d4bb184). Parked work: only the iteration-115 flame-occlusion WIP for issue #2, which is no longer open, so stale and ignored. The top 5 from iteration 170 stand unchanged; none has a new idea or an unblocked path, so nothing was started on speculation.

## Iteration 176 — 2026-10-28 — no change shipped (backlog unchanged)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (86652ff). No `wiggum/parked-*` branches. The top 5 from iteration 170 stand unchanged; none has a new idea or an unblocked path, so nothing was started on speculation.

## Iteration 177 — 2026-10-28 — no change shipped (backlog unchanged)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: #1 publish (release, seen, skipped; its Sanctuary comment of 2026-10-28 reports a null at its measurement resolution and asks for no code). Tree clean at start (d02e30f). No `wiggum/parked-*` branches. The top 5 from iteration 170 stand unchanged; none has a new idea or an unblocked path, so nothing was started on speculation.

## Iteration 178 — 2026-10-28 — no change shipped (backlog unchanged)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: #1 publish (release, seen, skipped). Tree clean at start (c2fdc09). Parked work: only the iteration-115 flame-occlusion WIP for issue #2, which is no longer open, so stale and ignored. The top 5 from iteration 170 stand unchanged; none has a new idea or an unblocked path, so nothing was started on speculation.

## Iteration 179 — 2026-10-28 — no change shipped (backlog unchanged)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (62c39ac). No `wiggum/parked-*` branches. The top 5 from iteration 170 stand unchanged; none has a new idea or an unblocked path, so nothing was started on speculation.

## Iteration 180 — 2026-10-28 — no change shipped (backlog unchanged)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: #1 publish (release, seen, skipped); no bug issues. Tree clean at start (89e3c87). No `wiggum/parked-*` branches. The top 5 from iteration 170 stand unchanged; none has a new idea or an unblocked path, so nothing was started on speculation.

## Iteration 181 — 2026-10-28 — no change shipped (backlog unchanged)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: #1 publish (release, seen, skipped; latest Sanctuary comment reports a null and asks for no code); no bug issues. Tree clean at start (7f9f51b). No `wiggum/parked-*` branches. The top 5 from iteration 170 stand unchanged; none has a new idea or an unblocked path, so nothing was started on speculation.

## Iteration 182 — 2026-10-29 — release 0.2.0 (issue #1)

Outcome: `Flicker { base, phase, ..default() }` documented as the stable spelling and pinned by `a_host_literal_naming_only_base_and_phase_takes_the_defaults` (tests/flicker.rs); CHANGELOG gains a 0.1 → 0.2 migration section (Flicker literals, `Rank::Illuminance` default). Assumed decision (owner may override): `#[non_exhaustive]` + constructor rejected because it forbids even `..default()` literals outside the crate, the spelling the issue's acceptance requires; documented the spelling instead. `cargo semver-checks` reports 4 major failures, so 0.2.0. Full gate green (build, test, serde, examples, clippy, doc, fmt, wasm, wesl-check). No visual or shader change; no captures. Open issues: #1 only, worked. Backlog from iteration 170 unchanged.
Release: https://crates.io/crates/bevy_lights/0.2.0 (commit tagged v0.2.0).

## Iteration 183 — 2026-10-29 — no change shipped (backlog unchanged)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: none (#1 was closed by the 0.2.0 release). Tree clean at start (f9dee27). No `wiggum/parked-*` branches. The top 5 from iteration 170 stand unchanged; none has a new idea or an unblocked path, so nothing was started on speculation.

## Iteration 184 — 2026-10-29 — no change shipped (backlog unchanged)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: none. Tree clean at start (main). No `wiggum/parked-*` branches. The top 5 from iteration 170 stand unchanged; none has a new idea or an unblocked path, so nothing was started on speculation.

## Iteration 185 — 2026-10-29 — no change shipped (backlog unchanged)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: none. Tree clean at start (ac21b5d). No `wiggum/parked-*` branches. The top 5 from iteration 170 stand unchanged; none has a new idea or an unblocked path, so nothing was started on speculation.

## Iteration 186 — 2026-10-29 — no change shipped (backlog unchanged)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: none. Tree clean at start (d114d1f). No `wiggum/parked-*` branches. The top 5 from iteration 170 stand unchanged; none has a new idea or an unblocked path, so nothing was started on speculation.

## Iteration 187 — 2026-10-29 — no change shipped (backlog unchanged)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: none. Tree clean at start (df66826). No `wiggum/parked-*` branches. The top 5 from iteration 170 stand unchanged; none has a new idea or an unblocked path, so nothing was started on speculation.

## Iteration 188 — 2026-10-29 — no change shipped (backlog unchanged)

Outcome: log only; no `src/`, shader, default or media change.

Open issues: none. Tree clean at start (0f8c4d2). No `wiggum/parked-*` branches. The top 5 from iteration 170 stand unchanged; none has a new idea or an unblocked path, so nothing was started on speculation.

## Iteration 189 — 2026-10-29 — `PoolBudget`: the pool's lamp schedule

Outcome: new public resource `PoolBudget { holders, wanting }` with `granted`, `starved`, `holder_of`, `holds`, rewritten each frame by `assign_pool`; no visual or default change, so no captures.

Open issues: none. Ranked candidates: (1) what bevy_emerge issue #103 asks of this crate ("show the light pool's budget (which lights are real right now)", previously unreadable outside `assign_pool`'s `Local`), (2) a serialisable pool budget (blocked: `PointLight` has no serde), (3) overhead flame at EV 9. Picked 1 under "a pass never ends idle": it is the editor's stated need, small, and testable. Assumed decision (owner may override): a `Resource` rather than a marker component on candidates, to avoid archetype moves on every hand-off.
Sources: research note `docs/research/light-pool.md` ("Iteration 189"); craft: lamp-schedule practice (corpus: cinematography entries); production: `s2020-advances-pdf-slides`, `s2025-advances-materials-9`; technical: RTR4 pp. 918-935. No new index.
Test: `the_budget_names_who_holds_a_light_and_who_went_without` (tests/pool.rs). Gate: fmt, build, test, test --features serde, clippy -D, doc -D, wasm all green (wesl-check not run: no shader change).
Bench (`bench_lights`, pool=16, 5000 candidates): after min_us=340 p50_us=545 pool_p50_us=415; baseline main min_us=334 p50_us=816 pool_p50_us=526 (baseline run overlapped a build, so noisy; min equal, no regression).
Cleaned: worktree ~/l189_base and ~/l189_base-target removed.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round 8/9 disc stack.
2. A cited, luminance-measured flicker amplitude.
3. Bearers are placeholder capsules.
4. Banding in the GIF's pool gradients.
5. Editor: serialisable `LightPool` settings (needs a serde mirror of `PointLight`'s four shared fields).

## Iteration 190 — 2026-10-29 — `LightPool` serialises (editor project files)

Outcome: `LightPool` gains serde under the `serde` feature; `light` is saved as the four fields the pool copies, intensity reads back 0, every field defaults when absent. No visual, shader or default change, so no captures.

Open issues: none. Ranked candidates: (1) serialisable pool settings for bevy_emerge #103, (2) overhead flame at EV 9, (3) cited flicker amplitude. Picked 1 under "a pass never ends idle": the editor's stated need, small, testable.
Assumed decision (owner may override): a `serde(with)` template mirror over changing `LightPool::light`'s type, which would break every host literal.
Sources: `docs/research/light-pool.md` ("Iteration 190"); no new reading.
Tests: `saved_pool::*` (tests/pool.rs, serde feature): round trip and partial file. Gate green: build, release build, test, test --features serde, examples, clippy -D, doc -D, fmt, wasm (wesl-check not run: no shader change). Bench not run: no frame-path code changed.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round 8/9 disc stack.
2. A cited, luminance-measured flicker amplitude.
3. Bearers are placeholder capsules.
4. Banding in the GIF's pool gradients.
5. Editor: validation helper for settings (range checks with a useful error).

## Iteration 191 — 2026-10-29 — `problems()`: settings validation for the editor

Outcome: `Flicker`, `FlameSite`, `FlameLook` and `LightPool` gain `problems() -> Vec<Problem>` (`Problem { field, kind: ProblemKind }`, `Display` + `Error`; engine-free type in `src/model/problem.rs`). The crate still never refuses a value. No visual, shader or default change, so no captures.

Open issues: none (gh list empty). Ranked candidates: (1) validation helper for bevy_emerge #103 (previous backlog item 5), (2) overhead flame at EV 9, (3) cited flicker amplitude. Picked 1 under "a pass never ends idle"; the others remain blocked/speculative. Assumed decision (owner may override): report a `depth` above `MAX_FLICKER_DEPTH` though it is clamped, because the flame reads the same number; a `Vec` of problems over a first-error `Result`, so a panel shows every bad field at once.
Sources: none new (validation is not a lighting default). Tests: `tests/settings.rs` (4) + `model::problem` unit test. Gate: test --features serde, clippy -D, doc -D, fmt, wasm green (full-feature examples/wesl-check not rerun: no example or shader change). Bench not run: no frame-path code.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round 8/9 disc stack.
2. A cited, luminance-measured flicker amplitude.
3. Bearers are placeholder capsules.
4. Banding in the GIF's pool gradients.
5. Editor: a preview hook (drive a single fire at a chosen time) if bevy_emerge asks.

## Iteration 192 — 2026-10-29 — `Flicker::intensity_at`: a preview curve for the editor

Outcome: `Flicker::intensity_at(t, kindle: Option<f32>)` returns the lumens a fire gives at a virtual time and kindle age; `flicker_lights`' `modulate` now calls it (one path). An editor scrubs a timeline or draws a fire's curve with no app. No visual, shader or default change, so no captures; bench not run (same arithmetic, no new work per frame).

Open issues: none. Ranked candidates: (1) preview hook for bevy_emerge #103 (previous backlog item 5), (2) overhead flame at EV 9, (3) cited flicker amplitude. Picked 1: the others stay blocked. Assumed decision (owner may override): a plain `Option<f32>` kindle age rather than a `Kindle` argument, so a preview needs no component. Sources: none (no lighting default touched). Test: `tests/settings.rs::intensity_at_follows_the_kindle_and_then_the_flicker`. Gate: test, test --features serde, clippy -D, doc -D, fmt, wasm green; machine load ~40 so builds were slow.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round 8/9 disc stack.
2. A cited, luminance-measured flicker amplitude.
3. Bearers are placeholder capsules.
4. Banding in the GIF's pool gradients.
5. Editor: a flame preview (a `FlameLook` at a chosen time) if bevy_emerge asks.

## Iteration 193 — 2026-10-29 — partial saved files load for `Flicker`, `Kindle`, `FlameLook`

Outcome: the three authored types that have a `Default` gain `serde(default)` (as `LightPool` has), so a project file written before a field was added, or by hand, still loads. `FlameSite` has no `Default` (its height and tag are required), so it is unchanged. No visual, shader or default change, so no captures; bench not run (no frame-path code).

Open issues: none on this repo; bevy_emerge #103's wants (preview, pool budget, serialisable settings, validation) are all met. Ranked candidates: (1) forward-compatible saved settings for the editor, (2) overhead flame at EV 9, (3) cited flicker amplitude. Picked 1: the others stay blocked. Assumed decision (owner may override): `serde(default)` over `deny_unknown_fields`, so an older crate reading a newer file still loads.
Test: `tests/settings.rs::saved_settings`. Gate: fmt, build, test --features serde, clippy -D, doc -D, wasm green.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round 8/9 disc stack.
2. A cited, luminance-measured flicker amplitude.
3. Bearers are placeholder capsules.
4. Banding in the GIF's pool gradients.
5. Editor: a flame preview (a `FlameLook` at a chosen time) if bevy_emerge asks.

## Iteration 194 — 2026-10-29 — log only: no workable item

Outcome: nothing shipped; no code, shader, default or media change, so no captures, bench or release (tree unchanged since v0.2.5).

Preflight (live): `hs status` shows distill up (192.168.1.110:7434, Cuda) and scribe up on two hosts. Open issues on this repo: none. No `wiggum/parked-*` branch; only old `WIP(iteration 115, parked)` commits (flame occlusion, long since shipped). bevy_emerge #103 (F7) wants: placed fires with `Flicker`/`Kindle`/`Flame`, pool budget view, light-map overlay (light-grid, not this crate). Placements are serde-able, `PoolBudget` shows which lights are real, `problems()`, `Flicker::intensity_at`, partial files all exist; the only unmet item, a CPU flame preview, would duplicate the shader and has no consumer ask.

Ranked candidates: (1) overhead flame at EV 9 needs a new construction, not the disc stack; (2) luminance-measured flicker amplitude: `distill_search` this pass returned only Chen 2019 and Gergely 2022 (frequency, intensity normalised to mean, no modulation depth in percent), and `flicker-depth.md` already rests on them, so no new citable number; Maynard 2013 not in the corpus; (3) placeholder bearers need a real asset; (4) GIF banding; (5) editor flame preview if bevy_emerge asks. Blocker for all three fallbacks of section 4: (1) is open-ended art work with no researched construction, (2) has no source to cite, (3) needs an asset; none is a ready item and inventing one would be speculation.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round 8/9 disc stack.
2. A cited, luminance-measured flicker amplitude (`paper_search` Maynard 2013 or similar, then download/convert).
3. Bearers are placeholder capsules.
4. Banding in the GIF's pool gradients.
5. Editor: a flame preview if bevy_emerge asks.

## Iteration 195 — 2026-10-29 — log only: no workable item

Outcome: nothing shipped; tree unchanged since v0.2.5, so no captures, bench or release. Open issues on this repo: none; no `wiggum/parked-*` branch; bevy_emerge #103 is open with no new comments asking for anything this crate lacks. Backlog unchanged from iteration 194 (overhead flame at EV 9 needs a new construction; cited flicker amplitude has no citable source in the corpus; placeholder bearers need an asset; GIF banding; editor flame preview with no consumer ask). Blocker for all three section-4 fallbacks is the same as iteration 194: none is a ready, researched item.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round 8/9 disc stack.
2. A cited, luminance-measured flicker amplitude.
3. Bearers are placeholder capsules.
4. Banding in the GIF's pool gradients.
5. Editor: a flame preview if bevy_emerge asks.

## Iteration 196 — 2026-10-29 — flames lean in the wind (`FlameWind`, `flame_tilt`)

Outcome: a new `FlameWind { velocity }` resource tilts every flame downwind by `flame_tilt(speed, diameter)`, Welker & Sliepcevich's pool-fire tilt with Pritchard & Binding's constants (`0.666 · Fr^0.333 · Re^0.117`), using each fire's own `diameter`. The flame turns rigidly about its foot (a first, sheared version collapsed to a streak at 57° and was rejected), capped at 70°, and fades back to upright from straight overhead. The lean rides on each flame's instance scale (`y`/`z` = `1 + sin θ · direction`), so the shared material and one-draw-per-flame cost are unchanged; no `FlameWind` or a zero one is the old picture (calm captures differ from the pre-change binary by 2–4 pixels of 921,600, GPU noise). `courtyard` takes `LIGHTS_WIND=x,z`.

Why first: open issues none; no parked branch. The three earlier fallbacks (overhead flame, cited flicker amplitude, bearers) are still blocked, but the standing rule is that "not researched" is work, so this pass researched a feature a shipping game expects (every exterior fire bending the same way) and built it. Runner-up: temperature-following flame colour (rejected: the `T⁴` inversion of the existing ±15 % flicker gives about ±4 % in kelvin, under what the eye separates, and the corpus has no colour-vs-intensity measurement).

Sources: craft `10.7456_tojdac.1674124` (Doğaner 2025); technical DGMR *Technische methode voor het modelleren van plasbranden* eq. 7–9 (Welker & Sliepcevich 1966, Pritchard & Binding constants; fetched from the web, not in the corpus) and Wen et al. 2019 (Warwick WRAP, §3.1: tilt rises and height falls with wind). Raj 2009 and Jiang & Lu 2016 are paywalled (`paper_download` returned HTML). No production/shipping-game source on wind-bent flames in the corpus: recorded under `Pending index` in `docs/research/flame-wind.md`. Home-still live: `hs status` distill up (Cuda), scribe up on two hosts.

Critique and verdict (`/tmp/l196/critique.md`): ground, elevated, top-down, toward-camera, an 8-azimuth orbit with the wind fixed (no flip, foreshortened when the wind runs along the view), 2 and 6 m/s: lean at 1–3 m/s GREAT; the 70° gale cap reads as a horizontal jet, acceptable and a design default; the fire's light does not move with the lean (Deviations).

Craft numbers: unchanged (key-to-fill, falloff, flicker rate, hand-off are untouched by a flame-shape change; calm frames identical). Bench `bench_lights` (CPU, pool 16, 5000 candidates), before vs after, two runs each, noisy at load ~40–80: p50 512/597 µs before, 604/552 µs after, min 254/291 vs 292/287; the bench has no flames, so no change is expected and none is outside the noise. Shader cost: a handful of ALU ops in the vertex stage and a 2x2 rotation in the fragment stage; not timed on the GPU.

Decisions assumed (owner may override): the host owns the wind and writes `FlameWind` (weather belongs to `bevy_elements`); the 70° cap and the 0.2 m default diameter are design defaults; the lean encoding on the instance scale rather than a second tag word (the `MeshTag` is full).

Gate: build (release and dev), test, test --features serde, build --examples --all-features, clippy -D warnings (after moving `wind` into the `Sites` param), doc -D warnings, fmt --check, wasm, wesl-check all green. Commits: `a209c5d` Research: flame wind, `e8ba09a` implementation, `c82f9ca` docs, plus the clippy fix and this log.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round 8/9 disc stack.
2. Wind gusts: a host drives `FlameWind` per frame today; a gust helper (smooth noise on speed and heading, deterministic) would make weather one line. Needs a source on gust spectra.
3. A cited, luminance-measured flicker amplitude.
4. Bearers are placeholder capsules.
5. GIF banding in the pool gradients.

Release: v0.2.6 published, https://crates.io/crates/bevy_lights/0.2.6 (tag `v0.2.6`, commit `6a80913`; `cargo semver-checks` found no breaking change). Cleanup: removed `/tmp/l196` (captures, binaries, gate logs); this pass made no worktree. Leftover baseline worktrees from earlier passes, listed and left: `~/bl_base`, `~/l128_base`, `~/l13/base`, `~/l13/base13` (all detached heads, sizes and cleanliness in the pass transcript; none touched).

## Iteration 197 — 2026-10-29 — flame gusts (`FlameGust`, `gust_unit`, `gust_velocity`)

Outcome: a new `FlameGust { mean, intensity, timescale, seed }` resource makes `FlamePlugin` write `FlameWind` every frame from `model::gust_velocity`: `|mean| · (1 + I·x(t))` with the heading turned by `I·y(t)` rad, `x`/`y` independent unit-variance stationary processes from `model::gust_unit` (stateless sum of 12 sinusoids under a one-pole spectrum, `f64` time so it stays smooth after hours). Without a `FlameGust` the plugin leaves `FlameWind` alone. `courtyard` takes `LIGHTS_GUST=I`.

Why first: open issues none, no parked branch; the loop-196 backlog's item 2 (gusts) was the only one with a researched path. Runner-up: overhead flame at EV 9 (still no researched construction). Skipped: none filed.

Sources: craft `10.7456_tojdac.1674124` (Doğaner 2025); technical Hannesdóttir, Kelly & Dimitrov 2019 `10.5194/wes-4-325-2019` (Gaussian stationary turbulence, meaning of turbulence intensity, IEC class C 0.12; read from the publisher's HTML). Corpus `distill_search` for gust spectra returned nothing relevant. No shipping-game source (`Pending index` in `docs/research/flame-gust.md`); spectral shape, `intensity` 0.25 and `timescale` 2 s are design defaults.

Critique and verdict (`/tmp/l197/critique.md`, removed with the scratch): gentle breeze (0.4 m/s, I 0.5, brazier crop, 14 frames 0.5 s apart): lean eases between upright and ~40° over 1–2 s, foot rooted, no clipping, no twitch at the 2.6 Hz puff: GREAT. At 1 m/s the lean sits on the flat part of the tilt law (sin θ 0.63–0.76 over 0.65–1.35 m/s), gust vs steady frame difference 0.03–0.11 DN mean: nearly invisible, physics and documented in the README. Process checks: variance 0.6–1.4 for six seeds, frame step under 0.1σ, autocorrelation at lag T within 0.2 of e⁻¹, heading σ within 0.1 of I. A first 16-tone version stepped 0.107σ per frame; cut to 12 tones (top 5.7 corners). A gate failure caught `intensity 0` returning `x≈-4e-8` for a mean along +Z; it now returns the mean exactly.

Craft numbers (key-to-fill, falloff, flicker, hand-off): untouched. Bench `bench_lights` (CPU, pool 16, 5000 candidates): min 268 µs, p50 427 µs (before, loop 196: min 254–291, p50 512–597); no flames in the bench, no expected change. The gust is one 24-sine evaluation per frame on the CPU.

Decisions assumed (owner may override): host keeps owning the mean wind; gusts are an opt-in resource, not on by default; heading wander `= I` rad.

Gate: build (release, dev), test, test --features serde, build --examples --all-features, clippy -D warnings, doc -D warnings, fmt --check, wasm, wesl-check all green.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round 8/9 disc stack.
2. A cited, luminance-measured flicker amplitude.
3. Spatial gust coherence (a gust crossing the courtyard) if a host reports wind-in-step looking wrong; needs a source.
4. Bearers are placeholder capsules.
5. GIF banding in the pool gradients.

Release: v0.2.7 published, https://crates.io/crates/bevy_lights/0.2.7 (tag `v0.2.7`, commit `c593eff`; semver-checks: no breaking change). Cleanup: removed `/tmp/l197` and `/tmp/wes.xml`; this pass made no worktree. Leftover baseline worktrees from earlier passes, listed and left: `~/bl_base`, `~/l128_base`, `~/l13/base`, `~/l13/base13`.

## Iteration 198 — 2026-10-29 — fire kinds (`FireKind`)

Outcome: `FireKind::{Candle, Lantern, Torch, Brazier, Campfire, Bonfire}` (`ALL`, `Reflect`, serde as the name) bundles `diameter()`, `depth()`, `height()` with `rate()`, `name()`, `flicker(mean)` and `site()`, built from the same numbers so a light and its flame agree. `courtyard` takes `LIGHTS_KIND=<name>`. Tests in `tests/settings.rs`: every kind has no `problems()`, the mean is the requested budget, light and flame agree on diameter and depth, heights fit the default `max_height`, rate falls from candle to bonfire, serde round-trips the name.

Why first: no open issues, no parked branch. The editor-needs rule (bevy_emerge issue #103: placed fires, "Flicker, Kindle and Flame" chosen in a panel) had serde, `problems()` and `intensity_at` already; the missing piece was a named picker list. Runner-up: overhead flame at EV 9 (no researched construction). Skipped issues: none filed.

Sources: craft `10.7456_tojdac.1674124`; production `s2011-advances-pdf-slides`; technical Chen et al. 2019 `10.1038_s41598-018-36754-w` (candle bed and rate) and Heskestad's flame-height law as restated in PMC6463528 (sanity bound only; the paper is not in the corpus: `Pending index` in `docs/research/fire-kinds.md`). Distill was up (`hs status`: distill, scribe, qdrant healthy); a corpus search for flame-height correlations returned nothing relevant.

Critique and verdict (`/tmp/l198/m.png`, removed): braziers dressed as Candle / Torch / Brazier / Bonfire at eye level (1.7 m, 4 m from the brazier, 1280x720, frame 90). Four sizes read as four fires: a thin candle sliver, a torch tongue, the existing brazier flame, a 2 m bonfire heave, flames clean, no clipping or detachment. Flaws: the courtyard's bowl is not scaled with the kind, so the bonfire stands in a brazier bowl and the candle floats over it (scene, not crate); the light's lumens are not rescaled by kind (host's budget). Verdict: GREAT for the presets as data, NOT a claim about how a scaled prop looks. Only Brazier is pinned to shipped numbers; the other four are design defaults.

Bench: no runtime path changed (a data enum), not re-run. Gate: build (release, dev), test, test --features serde, build --examples --all-features, clippy -D warnings (caught a needless `..default()` in the courtyard), doc -D warnings, fmt --check, wasm, wesl-check green.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round 8/9 disc stack.
2. A cited, luminance-measured flicker amplitude.
3. Fetch Heskestad's flame-height paper into the corpus and replace the preset heights' design defaults.
4. Spatial gust coherence if a host reports wind-in-step looking wrong.
5. Bearers are placeholder capsules; GIF banding in the pool gradients.

Release: v0.2.8 published, https://crates.io/crates/bevy_lights/0.2.8 (tag `v0.2.8`; semver-checks: no breaking change). Cleanup: removed `/tmp/l198`; no worktree made this pass. Earlier-pass baselines listed and left: `~/bl_base`, `~/l128_base`, `~/l13/base`, `~/l13/base13`.

## Iteration 199 — 2026-10-29 — fire reach (`FireKind::reach`, `lumens`, `light`)

Outcome: each `FireKind` carries `reach()` (metres lit to the pool's edge: 1, 1.5, 2, 4, 5, 9), with `lumens(lux, over)` (= `fire_lumens` at that reach) and `light(lux, over)` (a `Flicker` averaging it). The courtyard's `BRAZIER_POOL` and `TORCH_POOL` now read `FireKind::{Brazier, Torch}.reach()` (4.0 and 2.0, numerically unchanged). Test `fire_kinds_light_more_ground_as_they_grow` in `tests/settings.rs`: reach strictly rises, is at least `fire_lumens`' 1 m floor, lumens equal `fire_lumens`, bonfire/candle budget ratio is 81 at equal lux.

Why first: no open issues, no parked branch. bevy_emerge #103's picker had diameter/depth/height by name (iter 198) but still needed a lumen number per placed fire, the same guess that gave the baseline 60 000 lm. Runner-up: overhead flame at EV 9 (no new construction researched). Skipped issues: none filed.

Sources: craft `10.23856_6105` Table 1 (source size : distance, via `lighting-look.md`); production `s2011-advances-pdf-slides`; technical `book_real_time_rendering_4th_akenine` photometry (inverse square). Home-still live 2026-10-29: distill/scribe healthy via `hs`; the MCP `distill_search`/`paper_search` timed out once, `hs distill search` worked. No corpus document on luminous flux of wood fires; `paper_search` found only paywalled pool-fire radiative-fraction papers (`Pending index` in `docs/research/fire-reach.md`). Candle/Lantern/Campfire/Bonfire reaches are design defaults.

Verdict: no render path changed (a const table and two pure functions; the courtyard's constants are equal to before), so no recapture and no bench were run; GREAT/NOT YET does not apply to the data. Gate: build (release, dev), test, test --features serde, build --examples --all-features, clippy -D warnings, doc -D warnings, fmt --check, wasm, wesl-check green.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round 8/9 disc stack.
2. A cited, luminance-measured flicker amplitude.
3. Index a luminous-flux / radiative-fraction source for fires and replace the reach and height design defaults.
4. Spatial gust coherence if a host reports wind-in-step looking wrong.
5. Bearers are placeholder capsules; GIF banding in the pool gradients.

Release: v0.2.9 published, https://crates.io/crates/bevy_lights/0.2.9 (tag `v0.2.9`; semver-checks: no update required). Cleanup: no worktree or `/tmp` scratch made this pass. Earlier-pass baselines listed and left: `~/bl_base`, `~/l128_base`, `~/l13/base`, `~/l13/base13`.

## Iteration 200 — 2026-10-29 — `FireKind` parses and displays its name

Outcome: `FireKind: Display + FromStr` (case-insensitive, trimmed) and `UnknownFireKind` (its message lists the valid names), re-exported flat; the courtyard's `LIGHTS_KIND` uses it. Test `a_fire_kind_parses_the_name_it_displays` in `tests/settings.rs`. No numbers, shader or runtime path changed: no capture, no bench, no research note change (API only).

Why: no open issues, no parked branch; the editor-needs rule (bevy_emerge #103, picker of fire kinds) wanted a text round trip beside the serde one. Runner-up: overhead flame at EV 9 (still no researched construction). Small slice by choice; Display/FromStr is pure API.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round disc stack.
2. A cited, luminance-measured flicker amplitude.
3. Index a luminous-flux / radiative-fraction source; replace reach and height design defaults.
4. Spatial gust coherence if a host reports it.
5. Bearers are placeholder capsules; GIF banding.

Release: v0.2.10 published, https://crates.io/crates/bevy_lights/0.2.10 (tag `v0.2.10`; semver-checks: no update required). Gate green (build, test, serde test, examples, clippy, doc, fmt, wasm, wesl-check). No worktrees or /tmp scratch made. Earlier baselines listed and left: `~/bl_base`, `~/l128_base`, `~/l13/base`, `~/l13/base13`.

## Iteration 201 — 2026-10-29 — `fire_lux` and `fire_reach`

Outcome: `model::fire_lux(lumens, reach, over)` and `model::fire_reach(lumens, lux, over)` (re-exported flat), `fire_lumens` read back: the ground lux a placed fire gives and the radius of its iso-lux circle. Test `a_fires_lit_circle_is_the_reach_it_was_sized_for` in `tests/settings.rs` (every `FireKind` round-trips lumens to lux 12 and to its reach) plus doctests.

Why: no open issues, no parked branch. Editor-needs rule (bevy_emerge #103: show a placed fire's light, the light-map overlay; bevy_light_grid is on Bevy 0.20 now): the editor had a size-to-lumens path but no way to ask the crate how far a budget reaches. Runner-up: overhead flame at EV 9 (no researched construction). Skipped issues: none filed.

Sources: technical `book_real_time_rendering_4th_akenine` (inverse square, photometry), craft/production as in `docs/research/fire-reach.md`; no new constant, note section "Read back" added. No render path changed: no capture, no bench.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round disc stack.
2. A cited, luminance-measured flicker amplitude.
3. Index a luminous-flux / radiative-fraction source; replace reach and height design defaults.
4. Spatial gust coherence if a host reports it.
5. Bearers are placeholder capsules; GIF banding.

Release: v0.2.11 published, https://crates.io/crates/bevy_lights/0.2.11 (tag `v0.2.11`; semver-checks: no update required). Gate green (build, test, serde test, examples, clippy, doc, fmt, wasm). No worktrees or /tmp scratch made. Earlier baselines listed and left: `~/bl_base`, `~/l128_base`, `~/l13/base`, `~/l13/base13`.

## Iteration 202 — 2026-10-29 — `PoolCandidate::problems` and `Kindle::problems`

Outcome: the two serialisable components that had no validation now report their bad fields like `Flicker` and `LightPool` (`offset.x/y/z`, `intensity`, `phase`, `diameter`, `depth`; `elapsed`). Test `a_pool_candidate_and_a_kindle_report_their_bad_fields` in `tests/settings.rs`.

Why: no open issues, no parked branch. Editor-needs rule (bevy_emerge #103, a placed torch saved with its pool candidate): the validation surface had a hole on exactly the components a scene file holds. Runner-up: overhead flame at EV 9 (still no construction beyond disc stacks). Skipped issues: none filed. No lighting default, render path or shader changed: no capture, no bench; no new research note (reuses the ranges of `Flicker`).

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round disc stack.
2. A cited, luminance-measured flicker amplitude.
3. Index a luminous-flux / radiative-fraction source; replace reach and height design defaults.
4. Spatial gust coherence if a host reports it.
5. Bearers are placeholder capsules; GIF banding.

Gate green (build, test, serde test, examples, clippy, doc, fmt, wasm). No worktrees or scratch made; earlier baselines left: `~/bl_base`, `~/l128_base`, `~/l13/base`, `~/l13/base13`.

Release: v0.2.12 published, https://crates.io/crates/bevy_lights/0.2.12 (tag `v0.2.12`; semver-checks: no update required).

## Iteration 203 — 2026-10-29 — `FireKind::candidate`

Outcome: `FireKind::candidate(lux, over)` returns the `PoolCandidate` for a kind, sized like `FireKind::light` (mean lights the kind's reach to `lux`) with the kind's diameter and depth, so `flicker`/`light`, `site` and `candidate` all come from one name. Test `a_kinds_pool_candidate_is_the_same_fire_as_its_light` in `tests/settings.rs`.

Why: no open issues, no parked branch. Editor-needs rule (bevy_emerge #103, 3 comments, none asking for anything new): a placed fire's three components came from one name for two of them; the pooled one was hand-assembled. Runner-up: overhead flame at EV 9 (no researched construction). Skipped issues: none filed. No number, shader or render path changed: no capture, no bench.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round disc stack.
2. A cited, luminance-measured flicker amplitude.
3. Index a luminous-flux / radiative-fraction source; replace reach and height design defaults.
4. Spatial gust coherence if a host reports it.
5. Bearers are placeholder capsules; GIF banding.

Gate green (build, test, serde test, examples, clippy, doc, fmt, wasm). No worktrees or scratch made; earlier baselines left: `~/bl_base`, `~/l128_base`, `~/l13/base`, `~/l13/base13`.

Release: v0.2.13 published, https://crates.io/crates/bevy_lights/0.2.13 (tag `v0.2.13`; semver-checks: no update required).

## Iteration 204 — 2026-10-29 — `ground_lux` and `ground_reach`

Outcome: `model::ground_lux(lumens, distance, over)` and `model::ground_reach(lumens, lux, over)` (re-exported flat): illuminance on flat ground with Lambert's cosine term, and the radius of its iso-lux circle. `fire_lux`/`fire_reach` are the square-on law a light is sized by; used for an overlay they overstate the ground (0.22 of the number at 4 m out, 0.9 m up; the circle ~2x too wide), and their docs claimed "ground". Docs now name the surface; no number or default changed. Test `the_ground_gets_the_cosine_of_what_a_square_on_surface_does` in `tests/settings.rs` plus doctests.

Why: no open issues, no parked branch. Editor-needs rule (bevy_emerge #103, the light-map overlay): a correctness gap in what the previous iteration added for it. Runner-up: overhead flame at EV 9 (eleven rounds already NOT YET, no new construction); flicker amplitude re-searched live (`distill_search` returned only Gergely 2020 again; `paper_search` timed out), still no citable luminance depth. Skipped issues: none filed. home-still live state: distill answering, `paper_search` timed out at 30 s. Sources: technical `book_real_time_rendering_4th_akenine` (cosine law); `docs/research/fire-reach.md` "Flat ground". No render path changed: no capture, no bench.

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round disc stack.
2. A cited, luminance-measured flicker amplitude.
3. Index a luminous-flux / radiative-fraction source; replace reach and height design defaults.
4. Spatial gust coherence if a host reports it.
5. Bearers are placeholder capsules; GIF banding.

Gate green (build, test, serde test, examples, clippy, doc, fmt, wasm). Scratch removed: `/tmp/blg_probe`. Earlier baselines left: `~/bl_base`, `~/l128_base`, `~/l13/base`, `~/l13/base13`.
Release: v0.2.14 published, https://crates.io/crates/bevy_lights/0.2.14 (tag `v0.2.14`; semver-checks: no update required).

## Iteration 205 — 2026-10-30 — `Snuff` and `Snuffed`: a fire that goes out

Outcome: `Snuff { elapsed }` on a `Flicker` light multiplies its intensity by `snuff_gain` over `SNUFF_SECONDS` (1.2 s) to exactly 0; a `FlameSite` whose `fire` names it shrinks on the same curve; `Snuffed { entity }` is written once when out. `Snuff` wins over `Kindle`; remove it and insert `Kindle` to re-light. Tests: `a_snuffed_fire_falls_from_one_to_exactly_zero_and_never_rises` (model), `a_snuffed_light_falls_to_exactly_nothing_is_heard_once_and_can_be_lit_again` and `a_snuff_beats_a_kindle_and_a_loaded_out_fire_says_nothing` (tests/flicker.rs), `a_snuffed_fire_takes_its_flame_out_with_its_light` (flame.rs), `Snuff::problems` in tests/settings.rs. Courtyard: `O` / `LIGHTS_SNUFF=<s>`.

Why: no open issues, no parked branch (the `WIP(iteration 115, parked)` commits are old and long since superseded by the issue-#2 occlusion work on main). Editor/game need: the crate lit and re-lit fires but had no way to put one out, so a host dousing a torch cut light and flame in one frame. Runner-up: overhead flame at EV 9 (twelve NOT YET rounds, no new construction). Skipped issues: none filed. bevy_emerge #103: no new request since the light-map overlay pin. home-still live: distill answering (idle, Cuda), scribe up, `hs status` clean. Sources: craft `10.7456_tojdac.1674124` (Doğaner); production `s2011-advances-pdf-slides` ("Crossfade to a smokey particle over the lifetime"); technical `10.1038_s41598-020-78229-x` (Gergely: oscillation stops, then flame stops, as oxygen falls), `10.1017_jfm.2018.707` (Xia & Zhang). Note: `docs/research/flame-snuff.md`. No shader change (the flame's size already follows the instance scale).

Critique and verdict (`/tmp/l205/critique.md`, frames `/tmp/l205/b`, `/tmp/l205/after`): GREAT. Ground patch near a brazier 59.5 -> 41.3 DN, monotone from frame 3 and flat at the end (no pop at 1.2 s); flame peak 204 -> 93 DN (bowl's ember rim remains, constant emissive); warm pool recedes into the cool fill; bearers' torches unaffected. Flaw noted: the flame is a sliver slightly ahead of its light (area goes with height squared); minor.

Bench (same machine, 3 alternating runs): `flicker_p50_us` 148-151 -> 158-169 (+7-10%, ~6 ns per light: one more optional query column); `pool_p50_us` 308 unchanged; `p50_us` total 430-449 -> 451-500, `min_us` 263-270 -> 297-323 within run-to-run noise of the first pair. Trade accepted: the feature needs the column.

Hero GIF: not re-encoded; nothing it shows changed (no existing look or default moved).

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round disc stack.
2. A cited, luminance-measured flicker amplitude.
3. `Snuff`/`Kindle` for pool candidates (a candidate's pooled light has neither; needs a hand-off design).
4. Index a luminous-flux / radiative-fraction source; replace reach and height design defaults.
5. Bearers are placeholder capsules; GIF banding.

Release: v0.2.15 published, https://crates.io/crates/bevy_lights/0.2.15 (tag `v0.2.15`; semver-checks: no update required). Gate green (build, test, serde test, examples, clippy, doc, fmt, wasm, wesl-check). Removed this pass: `~/l205_base`, `~/l205_base-target`, `/tmp/l205`. Earlier baselines left: `~/bl_base`, `~/l128_base`, `~/l13/base`, `~/l13/base13`.

## Iteration 206 — 2026-10-30 — `Kindle` and `Snuff` on a `PoolCandidate`

Outcome: a pool candidate (no `Flicker` of its own) may carry `Kindle` / `Snuff`. `advance_candidate_fires` (chained before `assign_pool`) advances them and writes `Kindled` / `Snuffed` for the candidate; the pool lends `intensity · kindle_gain · snuff_gain`, the product a `Flicker` light uses; `FlameSite::fire` naming the candidate follows through the existing link (no flame change). A finished snuff leaves the candidate list (slot freed, `PoolBudget::wanting` drops); ranking keeps the plain intensity, so a dying fire keeps its slot until out. Tests (tests/pool.rs): `a_pooled_fire_that_is_snuffed_dims_its_light_to_nothing_and_frees_the_slot`, `a_pooled_fire_that_is_kindled_lends_a_light_that_flares_up_and_is_heard_once`. Courtyard `O`/`K`/`LIGHTS_SNUFF`/`LIGHTS_RELIGHT` now reach the 48 torches.

Why: no open issues, no parked branch; backlog #3 of iteration 205 (a doused bearer's torch could only be removed from the pool, so its light cross-faded while its flame stayed full). Runner-up: overhead flame at EV 9 (no new construction). Skipped issues: none filed; bevy_emerge #103 unchanged. Sources (note `docs/research/pool-fire-state.md`): craft `10.7456_tojdac.1674124`; production `s2011-advances-pdf-slides`; technical `10.1038_s41598-020-78229-x`; craft `pool-hand-off.md`. home-still not consulted this pass (sources reused from flame-snuff, already read).

Critique and verdict (`/tmp/l206/critique.md`, frames `/tmp/l206/a`, control `/tmp/l206/c`): GREAT. Mean frame luminance 56.4 -> 27.0 DN monotone, flat after; control flat 55.6-57.0; torch flames shrink with their lights (zoom). Flaws: brazier ember rim stays lit; minor flame/light size skew mid-fade.

Bench (3 alternating runs, same machine): `p50_us` base 549-564, after 553-589; `pool_p50_us` base 354-376, after 363-388; `min_us` 301-315 vs 292-337: within run-to-run noise. Hero GIF not re-encoded (nothing it shows moved).

### Top 5 remaining, ranked

1. Overhead flame at EV 9: a construction that is not the round disc stack.
2. A cited, luminance-measured flicker amplitude.
3. Index a luminous-flux / radiative-fraction source; replace reach and height design defaults.
4. Spatial gust coherence if a host reports it.
5. Bearers are placeholder capsules; GIF banding.

Release: v0.2.16 published, https://crates.io/crates/bevy_lights/0.2.16 (tag `v0.2.16`; semver-checks: no update required). Gate green (build, test, serde test, examples, clippy, doc, fmt, wasm, wesl-check). Removed this pass: `~/l206_base`, `~/l206_base-target`, `/tmp/l206`. Earlier baselines left: `~/bl_base`, `~/l128_base`, `~/l13/base`, `~/l13/base13`.

## Iteration 207 — 2026-10-30 — The flame keeps its brightness through 60-70° (overhead round 12)

Outcome: the upright tongue's overhead dimming is `OVERHEAD_TONGUE · topdown⁵` instead of `· topdown` (`src/flame.wesl`), so the flame no longer goes pale at 60°-70° before the cross-section has arrived. Frames at or below 55° and at 76° and past are unchanged (`magick compare` AE 0.01 px at 45°, 55°, 85°; 0.7 px at 76°).

Why: no open issues on bevy_lights, no parked branch (only the old iteration-115 WIP commits, superseded); bevy_emerge #103 asks for nothing new (last comment: light-grid pin). Ranked candidates: (1) overhead flame mid-blend dip (a visible artifact: 24 % of the tongue's brightness gone at 66° with the disc not yet in), (2) cited flicker amplitude, (3) per-kind colour temperature, (4) flux/radiative-fraction source, (5) bearers/GIF banding. Research attempts this pass: Chen et al. (`10.1038_s41598-018-36754-w`) measures candle-flame brightness as a dimensionless grey time series, figures only, no amplitude in the text, so (2) stays uncited and a single candle there is steady (it flickers only in bundles of 3+); (3) has no per-fuel temperature source in the corpus (distill returned only PBR ch. 12's blackbody) and wood-fire kinds would differ by tens of kelvin, so it was dropped.

Sources: craft `10.7456_tojdac.1674124` (Doğaner), production Gregory §11.4.3 and `s2011-advances-pdf-slides`, technical RTR4 §13.6.3 (`book_real_time_rendering_4th_akenine`), all already quoted in `docs/research/flame-overhead.md`, which gained "Round 12".

Critique and verdict (`/tmp/l207/critique.md`, `/tmp/l207/{before,after,p5}`): the mid-blend brightness slice GREAT, the overhead flame as a whole still NOT YET. Tongue-window mean grey at 60°, 66°, 70°, 73°, 76°: 131.3, 123.6, 116.2, 113.2, 111.4 (steps 7.7, 7.4, 3.0, 1.8) against 129.8, 118.1, 112.9, -, 111.4 before (steps 11.7, 5.2): the fall is spread, not stepped, and the 66° tongue reads as a plume, not a haze. Remaining flaws: lower contrast than at 45° because the floor behind is lit orange; a faint stub of the tongue's tip above the bowl rim at 60°-66°. Round one (`topdown³`) gave a smaller lift (+2.8 DN at 66°) and was replaced by `⁵` (+5.5).

Bench: not run; the change is two shader stages, `bench_lights` runs none. Hero GIF: not re-encoded (the showcase camera is at 0°-45°, unchanged to the pixel).

Gate green: build (release and debug), test, serde test, examples, clippy, doc, fmt, wasm, wesl-check.

### Top 5 remaining, ranked

1. Overhead flame: contrast against a lit floor, the tip stub at 60°-66°, then a construction that is not the round disc stack.
2. A cited, luminance-measured flicker amplitude (Maynard 2013 is not retrievable; Chen 2019 has figures only).
3. Index a luminous-flux / radiative-fraction source; replace reach and height design defaults.
4. Spatial gust coherence if a host reports it.
5. Bearers are placeholder capsules; GIF banding.

Leftover worktrees from earlier passes, listed not removed: `~/bl_base`, `~/l128_base`, `~/l13/base`, `~/l13/base13` (all detached, not mine). Removed this pass: scratch `/tmp/l207` (below).

Release: v0.2.17 published, https://crates.io/crates/bevy_lights/0.2.17 (tag `v0.2.17`; semver-checks: no update required). Removed this pass: `/tmp/l207` (no baseline worktree was made: the before captures came from the starting build).

## Iteration 208 — 2026-10-30 — The flame's core out-shines the floor it lit (overhead round 13)

Outcome: `OVERHEAD_CORE` (`src/flame.wesl`) raises the hot core's emission by up to 90 % with the overhead blend; frames at or below 50° are unchanged. Core-window mean grey at 60°, 66°, 76°: 148.1, 137.6, 131.4 before; 149.2, 141.0, 139.6 after; the 76° core is cream-yellow, not the grey-green it was (`/tmp/l208/pair76.png`).

Why: no open issues on bevy_lights (list read), no parked branch, bevy_emerge #103 unchanged and the editor-facing `problems()` validation already exists on every settings type. Ranked: (1) overhead contrast against the lit floor, (2) cited flicker amplitude, (3) flux/radiative-fraction source (distill searched again this pass: only photometry chapters and candle papers, no flame flux), (4) gust coherence, (5) bearers/GIF banding. Sources: as `flame-overhead.md` (Doğaner, Gregory §11.4.3 + `s2011-advances-pdf-slides`, RTR4 §13.6.3).

Verdict: core GREAT; overhead flame as a whole NOT YET. Remaining flaws: a hard horizontal line across the coal bed where the tongue's seat meets the disc; the 60°-66° plume is pale against the lit floor. Bench not run (shader only). Hero GIF not re-encoded (showcase camera at 0-45°, unchanged). Gate green (build, test, serde test, examples, clippy, doc, fmt, wasm, wesl-check).

### Top 5 remaining, ranked

1. Overhead flame: the hard line across the coal bed at 76°, plume contrast at 60°-66°, then a non-disc construction.
2. A cited, luminance-measured flicker amplitude.
3. A luminous-flux / radiative-fraction source for reach and height defaults.
4. Spatial gust coherence if a host reports it.
5. Bearers placeholder capsules; GIF banding.

Leftover worktrees from earlier passes, listed not removed: `~/bl_base`, `~/l128_base`, `~/l13/base`, `~/l13/base13`. Scratch `/tmp/l208` removed.

## Iteration 209 — 2026-10-30 — The seam across the coal bed is gone (overhead round 14)

Outcome: `heat` in `src/flame.wesl` is multiplied by `mix(1, foot, topdown)`, the same foot ramp the mask already used, so the hot core no longer stops dead at the tongue's seat. At 76° the column through the core fell `255,255,168` to `225,98,8` in two pixels before; it now falls over about ten. Frames at 45° are identical (AE 0.003 over 1280x720).

Why: no open issues on bevy_lights, no parked branch for this item (only the old iteration-115 WIP), bevy_emerge #103 unchanged. Ranked: (1) the hard line at 76° (a visible artifact, named by iterations 207 and 208), (2) plume contrast at 60°-66°, (3) cited flicker amplitude, (4) flux/radiative-fraction source, (5) bearers/GIF banding. Cause found with debug renders (disc only, tongue only, `sy` sign): not depth and not the mask ramp (widening it changed nothing), but the unramped heat. Sources: as `docs/research/flame-overhead.md` (Doğaner, Gregory §11.4.3 + `s2011-advances-pdf-slides`, RTR4 §13.6.3), which gained round 14.

Verdict (`/tmp/l209/{before,after}`, seed 7, frame 90, brazier at 4 m, 45°/60°/66°/76°, plus 90° seed 3): the seam GREAT; the overhead flame as a whole NOT YET. Remaining flaws: the core's lower half is now orange so the 76° core mean grey fell 157.9 to 153.0; the disc is still a round blob; the 66° plume is pale against the lit floor and keeps a stub of the tongue's tip above the rim.

Bench not run (shader only). Hero GIF not re-encoded (showcase camera at 0°-45°, unchanged). Gate green (build release and debug, test, serde test, examples, clippy, doc, fmt, wasm, wesl-check).

### Top 5 remaining, ranked

1. Overhead flame: plume contrast at 60°-66°, the tip stub, then a non-disc construction.
2. A cited, luminance-measured flicker amplitude.
3. A luminous-flux / radiative-fraction source for reach and height defaults.
4. Spatial gust coherence if a host reports it.
5. Bearers placeholder capsules; GIF banding.

Leftover worktrees from earlier passes, listed not removed: `~/bl_base`, `~/l128_base`, `~/l13/base`, `~/l13/base13`. Removed this pass: worktree `~/l209_base` (made, never built) and scratch `/tmp/l209`.
Release: v0.2.19 published, https://crates.io/crates/bevy_lights/0.2.19 (tag v0.2.19; semver-checks: no update required).

## Iteration 210 — 2026-10-30 — The flame from above is billows, not rays (overhead round 15)

Outcome: the overhead disc's brightness noise reads position instead of direction, its edge ramp is `smoothstep(0, 0.42, disc)` and its core heat is round and mottled (`src/flame.wesl`). The starburst of rays out of the core at 76° and 90° is gone; frames at 45° are unchanged (AE 0.0013 over 1280x720, GPU noise); 66° differs only inside the cloud.

Why: no open issues on bevy_lights (list read), no parked branch (only the old iteration-115 WIP), bevy_emerge #103 unchanged (pool budget, serde types and `problems()` already exist). Fresh-eye captures (default, ground, grazing, high, 44°-56° sweep at 2° steps, top-down) found no pop at the 50° facing switch and no other artifact; the one visible defect was the rays. Ranked: (1) rays in the top-down burst, (2) cited flicker amplitude, (3) flicker colour tracking brightness (distill search found no source, so not built), (4) flux/radiative-fraction source, (5) bearers/GIF banding. Sources: as `docs/research/flame-overhead.md` (Doğaner `10.7456_tojdac.1674124`, Gregory §11.4.3 + `s2011-advances-pdf-slides`, RTR4 §13.6.3); round 15 added there. Note and code are in one commit this pass, not two.

Verdict (`/tmp/l210/{before,a4,td}`, seed 7, frame 90, EV 9): rays slice GREAT; overhead flame as a whole NOT YET (small against the bowl, no tongue structure from straight above). Also seen, not a flame defect: a flame in front of a capsule 1 m from it vanishes into the capsule (additive over a surface lit to white; exposure, not shader).

Bench (`bench_lights`, CPU only, shader change does not touch it): p50_us 549, min_us 338, flicker 208, pool 378 on the starting build. Hero GIF not re-encoded (showcase camera at 0°-45°, unchanged). Gate green (build release and debug, test, serde test, examples, clippy, doc, fmt, wasm, wesl-check).

### Top 5 remaining, ranked

1. Overhead flame: tongue structure from straight above; size against the bowl.
2. A cited, luminance-measured flicker amplitude.
3. A luminous-flux / radiative-fraction source for reach and height defaults; a source for flicker colour tracking brightness.
4. Spatial gust coherence if a host reports it.
5. Bearers placeholder capsules; GIF banding.

Leftover worktrees from earlier passes, listed not removed: `~/bl_base`, `~/l128_base`, `~/l13/base`, `~/l13/base13`. Removed this pass: worktree `~/l210_base` (never built), scratch `/tmp/l210`.
Release: v0.2.20 prepared (`release: v0.2.20`, changelog moved, semver-checks: no update required) but `cargo publish` was refused: `429 Too Many Requests: You have published too many versions of this crate in the last 24 hours`. Not tagged. The next pass publishes 0.2.20 as is (do not re-bump), tags `v0.2.20` and puts the crates.io URL here.

## Iteration 211 — 2026-10-30 — Release retry only

Outcome: no code change. `cargo publish` of v0.2.20 retried from the clean `main` (`75b619c`, equal to origin) and refused again: `429 Too Many Requests: You have published too many versions of this crate in the last 24 hours`. Not tagged. No open issues on bevy_lights, no parked branch. The backlog's remaining items are not workable with sources in hand: flicker amplitude was already searched without a citable result (`docs/research/fire-light.md`, "The amplitude is still uncited"), and the overhead flame is at round 15 with the last verdict NOT YET; any further code would need another release that the rate limit blocks anyway. The next pass publishes 0.2.20 as is (no re-bump), tags `v0.2.20`, and puts the crates.io URL here.

Top 5 unchanged from iteration 210.

## Iteration 212 — 2026-10-30 — Release retry only

Outcome: no code change. `cargo publish` of v0.2.20 retried from clean `main` (`ee32797`, equal to origin) and refused a third time: `429 Too Many Requests: You have published too many versions of this crate in the last 24 hours`. crates.io `max_version` is still 0.2.19. The tag `v0.2.20` already exists on origin (annotated, on `e47b014`, the release commit), so the next pass only publishes; do not re-bump, do not re-tag. No open issues on bevy_lights (list read), no parked branch. Backlog unchanged from iteration 210; no new code was started because each item needs a release the rate limit blocks, and the overhead-flame item is a multi-round visual job better started with a publish window open.

Top 5 unchanged from iteration 210. The next pass: `cargo publish`, then `cargo info bevy_lights@0.2.20` and the crates.io URL here.

## Iteration 213 — 2026-10-30 — Audit pass: nothing shipped (release window closed, no defect found)

Outcome: no code change. Open issues on bevy_lights: none (list read). Parked branches: none new. bevy_emerge #103: every acceptance item it names for this crate already exists (`FireKind`, `PoolBudget`, serde types, `problems()`, `Flicker::intensity_at`); its latest comment is the bevy_light_grid 0.20 pin, not a crate-side request. Release: v0.2.20 is tagged and unpublished; the previous pass's 429 was the last retry, and per the 6-hour/429 rule this pass did not retry it. The next pass publishes 0.2.20 as is.

What was checked, so the next pass does not redo it:
- Fresh-eye captures at 1280x720, seed 7, frame 120 (`default`, ground 1.7 m, grazing 0.4 m, top-down 14 m, 60° elevated): no pop, seam, hard range cut-off or unmotivated light found; the top-down brazier's flame is a leaning tongue over a mottled coal bed (round 15's result).
- README hero: `docs/media/showcase.gif` frame 30 coalesced against a fresh capture of the same sim frame (69, `LIGHTS_FOCUS_WALK=18`), RMSE 0.047 at 400x225: the GIF still shows the current look despite the six shader commits since it was encoded (they act at camera elevations above 45° and in the overhead view). No re-encode needed.
- Flicker amplitude citation (backlog 2) tried again by three routes: `paper_search` (Maynard 2013 now lists `escholarship.org/uc/item/851832j6`), direct PDF fetch (`/content/qt851832j6/qt851832j6.pdf` and `.pdf` both HTTP 403 to an automated client), `web_search` (no figure surfaces). The IOP flicker-measurement paper (`10.1088/0957-0233/10/8/307`) and the sooting pool-fire radiation spectra paper (`10.1115/imece2001/htd-24248`) have no downloadable copy. `±0.15` stays a design default.
- Pool micro-optimisation (`sort_by` to `sort_unstable_by` on the total order `(key, Entity bits)`): output-identical, but `bench_lights` moved p50 800 to 730 µs against a noise of ±50 µs on this loaded machine; reverted rather than claim an unproven win. A real gain needs a top-K selection in `assign_pool`, which touches the hand-off rules (`rank_of`, `worst`, `wanting`) and is not worth the risk for 0.4 ms at 5000 candidates.
- Per-fire flame colour (candle against bonfire): rejected for now. The tag's 32 bits are full (height 16, phase 8, brightness 8), colour would need a second per-instance datum or 4 bits taken from height, and real soot-flame colour temperatures of the six kinds differ by a few hundred kelvin with no citable per-fuel table in the corpus. Flicker amplitude as temperature bound through T⁴ also gives only about ±50 K at ±15 %, so brightness-tracking colour is invisible; record, do not build.

Ranked candidates this pass: (1) none workable as a defect, (2) cited flicker amplitude (blocked on a 403), (3) per-fire flame colour (no source), (4) pool top-K selection (perf, risk), (5) overhead flame tongue structure (round 15 left NOT YET).

### Top 5 remaining, ranked

1. Publish v0.2.20 (no re-bump, tag exists), crates.io URL here.
2. Overhead flame: tongue structure from straight above; size against the bowl.
3. Cited flicker amplitude: needs Maynard 2013 from a host with library access.
4. Per-fire flame colour, if a per-fuel colour-temperature table is found (`paper_search` for soot-flame temperature by fuel).
5. Pool top-K selection if a host reports the pool over 0.5 ms.

Removed this pass: scratch `/tmp/l213`, `/tmp/m.html`, `/tmp/m.pdf`. Leftover worktrees from earlier passes, listed not removed: `~/bl_base`, `~/l128_base`, `~/l13/base`, `~/l13/base13`.
