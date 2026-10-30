# The pool's shadow budget: the nearest fires cast shadows, the rest only light

The pool hands a dozen real lights to the props nearest the focus, and until now every one of them
was either shadowless (the default) or shadowed (the template's `shadow_maps_enabled`, all or none).
Neither is what a shipping game does. A shadowless torch lights the far side of a pillar and puts no
shadow under the figure it stands beside; sixteen shadowed point lights render ninety-six shadow
views a frame. This note is about the middle: **a budget of shadowed lights, given to the best-ranked
holders, and a shadow that arrives and leaves by a cross-fade rather than a pop.**

## Sources

| source | what it is | use |
|---|---|---|
| `10.1145_1008213.1008238` — El-Nasr & Horswill, *Automating Lighting Design for Interactive Entertainment* (the catalog has no DOI or title for this stem; both are read off its markdown) | lighting design for games, from film and theatre | craft: why a light without a shadow is half a light |
| `10.58709_niujhu.v8i3.1694` — Orisayemi & Afabor, *A Critical Scrutiny of Cinematic Lighting Technique* (Bordwell & Thompson quoted) | overlapping keys as actors move | craft: shadows must not jump |
| `s2016-advances-slides-pdf-15-mb` — Geffroy & Sousa, *The Devil is in the Details: idTech 666* (DOOM 2016), SIGGRAPH 2016 *Advances* | the shadow atlas | production: shadow cost scaled by distance |
| `s2025-advances-materials-9` — *Stochastic Tile-Based Lighting in HypeHype*, SIGGRAPH 2025 *Advances* | a shadow-map atlas for user-placed lights | production: shadow budget by distance, updates load-balanced |
| `s2025-advances-materials-5` — Kulikov, *Strand-based hair and fur in Indiana Jones and the Great Circle*, SIGGRAPH 2025 *Advances* | shading against the game's lights | production: games ship unshadowed lights to save performance |
| `s2017-advances-pdf-slides-3` — Drobot, *Improved Culling for Tiled and Clustered Rendering* (Call of Duty: Infinite Warfare), SIGGRAPH 2017 *Advances* | a stress scene of 128 shadowed lights | production: what many shadowed lights cost |
| `book_real_time_rendering_4th_akenine` — Akenine-Möller et al., *Real-Time Rendering* 4th ed., §7.4 and §20.4 | many-light shadows; cascade boundary blending | technical: shadow only the closest and brightest; blend across a boundary |
| GPU Gems ch. 12 — Gerasimov, *Omnidirectional Shadow Mapping* (corpus file `GP/GPU Gems- …`) | the cube shadow map a point light needs | technical: one cube (six views) per light, every frame |
| `bevy_pbr-0.20.0/src/render/light.rs:1170-1233` | Bevy's point-light shadow allocation | the engine's own cap: texture-array layers / 6, not a budget |

## Craft principle

A cinematographer does not separate a light from its shadow. El-Nasr & Horswill, writing on lighting
design for games:

> "Shadows are a very important elements of lighting design. In fact, most of cinematic and
> theoretic lighting design principles are designed based on shadows. There are two types of
> shadows: cast shadows of a character or an object on another, and shadows created by lack of
> lights hitting the surface of an object or area."

A pool light that casts no shadow does the second kind and not the first: the figure beside the torch
stands on ground the torch lights all the way round it, and the pillar between two torches is lit
from both sides through its own stone. The near fires are where that reads, because they are what the
shot is about; a fire forty metres down the street is a pool of warm ground, and nobody can see that
its shadow is missing.

The second principle is the one the pool already lives by. Bordwell & Thompson, quoted by Orisayemi
& Afabor, on why a film overlaps its keys as actors move:

> "By overlapping different key lights, the filmmaker can maintain a constant intensity as actors
> move around the set … it will prevent distracting shadows and highlights from moving across other
> actors."

A shadow that switches on is exactly a distracting shadow: the ground behind every occluder in that
light's range darkens in one frame. So a light gaining or losing its shadow is a hand-off, and it is
**cross-faded** like one: the shadowless light fades out while the shadowed light fades in at the same
place, so the shadow deepens over the fade instead of appearing.

## How shipping games do it

- **Shadow cost is budgeted by distance.** DOOM 2016's shadow atlas (Geffroy & Sousa): "Variable
  resolution based on distance", "Time slicing also based on distance", "Shadows are cached / packed
  into an Atlas", and the light "Can still animate [ e.g. flicker ]". HypeHype (SIGGRAPH 2025): "We also
  resize the shadow maps based on lights distance from the camera, to balance the shadow quality with
  the limited atlas storage", and "To avoid performance spikes when many shadow maps require updates,
  we load balance the updates with a priority queue system to spread the shadow map rendering across
  multiple frames."
- **Unshadowed lights are a shipping tool, not a bug.** Machine Games on *Indiana Jones and the Great
  Circle*: "the game can use lights that do not cast shadows to save performance." HypeHype names the
  failure it guards against: "our creators might just decide to add 50 shadowed lights to a room,
  because why not?"
- **Many shadowed lights are expensive.** Infinity Ward's culling talk measures a stress scene of
  "128 shadowed lights" at "Clustered F+ : 13ms", 7 ms with surface caching: a whole frame budget.

Bevy has no shadow cache and no atlas: every shadowed point light renders its cube every frame, and
`prepare_lights` caps the count only at `max_texture_array_layers / 6`. So the one lever a crate has
is *which* lights carry `shadow_maps_enabled`, and the pool is the one place that knows which lights
matter.

## Model adopted

The technical rule is RTR4's (§20.4, many lights):

> "Shadows are a challenge when a large number of light sources are present. One response is to
> ignore shadow computations for all but the closest and brightest lights, and the sun, at the risk of
> light leaks from the lesser sources."

"Closest and brightest" is what the pool already ranks by (`Rank::Illuminance`, intensity over
distance squared), so the shadowed lights are the best-ranked of the lights the pool holds.

1. **`LightPool::max_shadowed: Option<usize>`** caps how many of the pool's lights cast the shadow
   the template asks for (`LightPool::light.shadow_maps_enabled`). `None` is no cap, which is the
   pool's behaviour before this note: the template's flag on every slot. The pool now writes every
   slot's flag itself, within the cap, as it writes its intensity.
2. **Who gets one**: the best-ranked holders, by `LightPool::rank`'s own value. A holder keeps its
   shadow unless a shadowless holder is better than the worst shadowed one by `LightPool::hysteresis`,
   the same overlap rule (`preferred`) that holds a slot, so two torches at nearly the same distance
   never trade a shadow frame after frame. An empty place in the budget is filled at once by the best
   shadowless holder.
3. **A light that changes owner** takes its new owner's shadow on the frame it changes: the slot is
   fading in from nothing anyway, and the reserve light that fades the old owner out carries the old
   owner's shadow flag, so a departing shadow fades out with its light.
4. **A light that keeps its owner and gains or loses its shadow is a hand-off to itself.** A free
   reserve takes the light as it was (place, intensity, phase, flag) and fades it out while the slot
   restarts its fade-in with the new flag. In lit ground the two sum to one, as in any cross-fade; in
   the occluded ground the shadowed light contributes nothing, so the shadow deepens smoothly over
   `fade_seconds`. RTR4 §7.4 is the technical precedent: at a cascade boundary "samples taken in these
   overlap zones gather results from both adjoining shadow maps and are blended."
5. **It shares the hand-off budget.** A shadow change needs a free reserve, after the frame's owner
   changes have taken theirs; with none free it waits for the next frame (the same rule that stops a
   move the pool cannot cross-fade), and so does a light still fading from its last change. Losses are
   applied before gains, so a gain that waits never puts the pool over its cap. With `reserve: 0` there
   is nothing to fade behind, and a shadow change dips the light and brings it back with the new flag,
   as a hand-off does.
6. **A change of the settings is not a hand-off.** Turning the template's flag on or off, or moving the
   cap, applies to every light on the frame it is made, like the template's colour: a game's graphics
   menu changes everything at once, and fading sixteen lights through one reserve would take six
   seconds.
7. **`PoolBudget::shadowed`** lists the candidates whose light casts a shadow this frame, so an editor
   can show which fires are shadowed beside which are real.

## Parameters

| parameter | value | source |
|---|---|---|
| `LightPool::max_shadowed` | `None` (no cap: the template's flag on every light, as before) | design default: the pool's cost and look are unchanged for every host that does not ask; RTR4's rule says *which* lights, not how many, and the count is the host's frame budget |
| who is shadowed | the best-ranked holders by `LightPool::rank` | RTR4 §20.4, "the closest and brightest lights" |
| hysteresis on the shadow set | `LightPool::hysteresis` (1.15) | the pool's own overlap rule (`pool-hand-off.md`); one rule for holding a light and holding a shadow |
| shadow fade | `LightPool::fade_seconds` (0.35 s), smoothstep | `pool-hand-off.md`: below the eye's fusion threshold, read as a change rather than a cut |
| reserve flag | the departing light's | a shadow leaves with its light |
| shadowed lights during a fade | up to `max_shadowed + reserve` | the cost of not popping, for one fade |
| reserves for a pool with shadows | 2 (the courtyard's) | measured: at `reserve: 1` the courtyard's three shadows were all placed in 39 % of frames, at 2 in 99 % (see *Iteration 271's findings*); the crate's default stays 1 |

## Validation targets

- **The budget holds**: with a pool of 8 and `max_shadowed: Some(3)`, exactly 3 of the slots' lights
  have `shadow_maps_enabled`, and they are the 3 best-ranked holders (test).
- **The template's flag is the switch**: with it off no slot casts a shadow whatever the cap, and with
  it on and no cap every slot does, as before (tests, the second the template test that predates this).
- **Hysteresis**: a shadowless holder 10 % better than the worst shadowed one does not take its
  shadow; one 20 % better does (test).
- **A shadow arrives by a cross-fade**: when a held light gains its shadow, a reserve light appears at
  the same place with the flag off and fades out while the slot fades in with the flag on, and the two
  bases sum to the slot's full base through the fade (test).
- **`PoolBudget::shadowed`** names exactly the shadowed holders (test).
- **Capture**: with `LIGHTS_SHADOWED`, the near torches throw pillar and figure shadows that the far
  ones do not, and a frame sequence across a shadow change shows the shadow deepening over the fade,
  not appearing in one frame (largest consecutive-frame difference inside the shadow region).
- **Cost**: GPU time of the shadow views grows with `max_shadowed`, not with the pool's size; measured
  on the courtyard with `LIGHTS_GPU_TIMING`.
- **CPU**: `bench_lights` `pool_p50_us` within the run-to-run noise of the baseline at no cap.

## What great looks like

- **Shadows are where the eye is.** The two or three fires nearest the subject throw shadows: a
  figure standing by a torch has a dark shape on the ground on the side away from it, and a pillar
  between two torches has a dark side. Further out, fires are pools of light and nobody misses their
  shadows.
- **No shadow ever pops.** Frame by frame across the moment a torch earns its shadow, the shadow
  grows from nothing to full over about a third of a second; the lit ground around it does not
  change brightness at all.
- **One source, one shadow.** Each shadow is motivated by a visible flame, and it falls away from that
  flame.
- **The budget is visible as a number.** An editor reads `PoolBudget::shadowed` and can show which
  fires are shadowed now.

## Deviations

- **No resolution by distance.** DOOM and HypeHype shrink a far light's shadow map; Bevy's point-light
  shadow map size is one resource (`PointLightShadowMap`) for every light, so a crate cannot set it per
  light. The budget is a count instead.
- **No time slicing.** Bevy re-renders every shadowed cube every frame; the crate cannot ask it to
  refresh one light's shadow less often.
- **No penumbra.** A torch's shadow is as crisp two metres from a figure as at its feet, where a 0.35 m
  flame at that distance should give what `10.23856_6105` (Table 1, cited in `lighting-look.md`) calls
  "fuzzy shadow contours". Bevy's point shadows soften only through PCSS (`soft_shadows_enabled` with a
  light `radius`, behind Bevy's `experimental_pbr_pcss` feature), which the pool does not forward. The
  braziers' shadows have the same edge, so the frame is consistent; this is the next item, not part of
  this one.

## Iteration 271's findings

- **The design changed once, before it shipped.** The first draft made `shadowed: usize` the only path
  and ignored the template's flag (naming it as a problem). That broke a consumer-visible behaviour an
  existing test pins (the template's `shadow_maps_enabled` reaches every slot) and the meaning of a saved
  project, so the flag stayed the switch and the budget became a cap, `max_shadowed: Option<usize>`.
- **A settings change snaps.** Fading every light through one reserve after the flag is flipped took
  `size × fade_seconds` (5.6 s for sixteen lights), so a change of the cap or the flag applies at once;
  only a change of ranking cross-fades.
- **Measured cross-fade** (a throwaway still scene: one shadow handed from one torch to another by moving
  the focus, nothing else changing): the arriving shadow's region falls 24.08 → 19.27 DN over 21 frames,
  largest step 0.35 DN; at `fade_seconds: 0` the same 4.81 DN in one frame. The lit ground around both
  torches moved 0.000 DN once the two ends of the fade ran on one clock (it sagged 2.5 % before; see
  `pool-hand-off.md`, iteration 271).
- **Reserves**: at `reserve: 1` the courtyard's three shadows were all placed in 39 % of frames (hand-offs
  hold the one reserve), at 2 in 99 % (93 % with the focus walking at 14 m).
- **Cost** (RTX 3090, 1280x720, the courtyard, Bevy's `auxiliary_views` GPU time over frames 100-400):
  2.16-2.48 ms with no pool shadow (the four braziers), 2.88 ms with one, 3.74 with two, 3.92 with three:
  about 0.55-0.6 ms a shadowed point light. Above three pool shadows Bevy's render diagnostics drop
  passes, so eight and sixteen were not measured directly.
- **CPU** (`bench_lights`, three alternating runs each): `p50_us` 742/847/757 before, 714/756/788 after;
  `pool_p50_us` 542/546/520 before, 577/508/501 after — within the machine's run-to-run noise.
- **The look.** Figures had been `NotShadowCaster`; with them casting, the braziers' figure shadows
  radiate from the four bowls (the top view's strongest read), and a lantern bearer near the focus
  throws its own wedge away from its lantern (lit ground 116 DN, its shadow 39 DN, 52 DN with the pool's
  shadows off). A figure's shadow on a pillar face the moon misses was black until the courtyard's sky
  fill rose to 20 cd/m² (`lighting-look.md`).

## Pending index

- `10.1145_1008213.1008238` has no title or DOI in the catalog (`catalog_read` returns none); its title
  is read off the markdown. `catalog_backfill_title` cannot help without a DOI.
- GPU Gems' corpus file has no stem in the catalog; it is cited by chapter.
