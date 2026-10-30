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

1. **`LightPool::shadowed: usize`** — how many of the pool's lights cast shadows. The pool owns every
   slot's `shadow_maps_enabled`, as it owns its intensity: the template's flag is ignored, and
   `LightPool::problems` names it when it is set, so a host that set it hears why its pool has no
   shadows.
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
   move the pool cannot cross-fade). With `reserve: 0` there is nothing to fade behind, and a shadow
   change dips the light and brings it back with the new flag, as a hand-off does.
6. **`PoolBudget::shadowed`** lists the candidates whose light casts a shadow this frame, so an editor
   can show which fires are shadowed beside which are real.

## Parameters

| parameter | value | source |
|---|---|---|
| `LightPool::shadowed` | `0` | design default: the pool's cost and look are unchanged for every host that does not ask; RTR4's rule says *which* lights, not how many, and the count is the host's frame budget |
| who is shadowed | the best-ranked holders by `LightPool::rank` | RTR4 §20.4, "the closest and brightest lights" |
| hysteresis on the shadow set | `LightPool::hysteresis` (1.15) | the pool's own overlap rule (`pool-hand-off.md`); one rule for holding a light and holding a shadow |
| shadow fade | `LightPool::fade_seconds` (0.35 s), smoothstep | `pool-hand-off.md`: below the eye's fusion threshold, read as a change rather than a cut |
| reserve flag | the departing light's | a shadow leaves with its light |
| shadowed lights during a fade | up to `shadowed + reserve` | the cost of not popping, for one fade |

## Validation targets

- **The budget holds**: with a pool of 8 and `shadowed: 3`, exactly 3 of the slots' lights have
  `shadow_maps_enabled`, and they are the 3 best-ranked holders (test).
- **The template's flag is not a second path**: a template with `shadow_maps_enabled: true` and
  `shadowed: 0` gives no shadowed slot, and `problems()` names `light.shadow_maps_enabled` (test).
- **Hysteresis**: a shadowless holder 10 % better than the worst shadowed one does not take its
  shadow; one 20 % better does (test).
- **A shadow arrives by a cross-fade**: when a held light gains its shadow, a reserve light appears at
  the same place with the flag off and fades out while the slot fades in with the flag on, and the two
  bases sum to the slot's full base through the fade (test).
- **`PoolBudget::shadowed`** names exactly the shadowed holders (test).
- **Capture**: with `LIGHTS_SHADOWED`, the near torches throw pillar and figure shadows that the far
  ones do not, and a frame sequence across a shadow change shows the shadow deepening over the fade,
  not appearing in one frame (largest consecutive-frame difference inside the shadow region).
- **Cost**: GPU time of the shadow pass grows with `shadowed`, not with the pool's size; measured on
  the courtyard with `LIGHTS_GPU_TIMING`.
- **CPU**: `bench_lights` `pool_p50_us` within the run-to-run noise of the baseline at `shadowed: 0`.

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

## Pending index

- `10.1145_1008213.1008238` has no title or DOI in the catalog (`catalog_read` returns none); its title
  is read off the markdown. `catalog_backfill_title` cannot help without a DOI.
- GPU Gems' corpus file has no stem in the catalog; it is cited by chapter.
