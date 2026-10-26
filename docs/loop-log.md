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
