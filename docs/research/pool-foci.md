# The pool for more than one viewer: `LightPool::focus`

`LightPoolPlugin<F>` measured "near" from exactly one entity carrying `F`; two of them was a reported misconfiguration and left every pool light hidden (`docs/research/light-pool.md`, "no focus, and two foci each leave every slot hidden"). That is right for a game with one camera and wrong for the games that have more than one: local split-screen co-op, a map or picture-in-picture camera next to the player's, a cutscene camera while the player stands elsewhere, an editor with two viewports. This note decides what the pool should do for them and what it should not.

## Sources

- **Craft:** *Multiple-camera setup*, Wikipedia (read 2026-10-31 through the reader, https://en.wikipedia.org/wiki/Multiple-camera_setup). Web, not in the corpus (see `Pending index`).
- **Production:** `s2025-advances-materials` (Tatarchuk, *Advances in Real-Time Rendering in Games, celebrating 20 years*, SIGGRAPH 2025, the list of games that shaped the field, lines 862 and 918 of the on-disk markdown); Bevy's own `examples/3d/split_screen.rs` and `examples/stress_tests/many_cameras_lights.rs` (`bevy-0.20.0`, read from the vendored source).
- **Technical:** `book_real_time_rendering_4th_akenine` §20.4 (the many-light rule the pool already follows: shadow "the closest and brightest lights") and chapter 21 (a frame drawn from several viewpoints at once); Bevy's `bevy_light-0.20.0/src/cluster/mod.rs` (`Clusters`, per camera) and `bevy_pbr-0.20.0/src/render/light.rs` (`ViewLightEntities`, per camera).

## Craft principle

A cinematographer who has to light for several cameras at once does not get to light for each; the setup is a negotiated one. The multi-camera article states the cost plainly:

> "Drawbacks include a less optimized lighting setup that needs to provide a compromise for all camera angles"

That is the whole of the craft question for a pool with two viewers: the compromise is unavoidable, so what matters is **who pays for it**. The single-camera rig the pool was built for spends every light on the one subject. Given two subjects, a rig that spends them all on whichever subject has the most fixtures near it leaves the other camera in the dark, which is not a compromise but a failure of one shot. The craft answer is that each camera gets a share of the lights, and a light that serves both (a fixture between the two players) is one light, not two.

## How shipping games do it

Split-screen co-op is not a corner case of the games a lighting crate is meant for. The SIGGRAPH 2025 *Advances* course opens with the games that shaped the field in twenty years, and two of the entries are defined by it:

> "It Takes Two: narrative co-operative gameplay with forced split screen (Even if playing across network); unique business model that emphasized the teamwork of the game"

> "Split Fiction: expanded upon the forced split-screen co-op model with an industry relevant narrative and a wide assortment of game genres packaged as narrative levels"

The engine this crate is built on treats several cameras on one scene as ordinary: Bevy ships `examples/3d/split_screen.rs` ("Renders four cameras to the same window to accomplish 'split screen'") and `examples/stress_tests/many_cameras_lights.rs` ("Test rendering of many cameras and lights"), and `bevy_light`'s `Clusters`, the assignment of lights to the view's cells, is a component of each camera, and `bevy_pbr`'s `ViewLightEntities` is "the shadow cascade views for all shadow cascades associated with a camera": the light assignment and the cascade views are kept per camera, so a host with two cameras pays for its lights in each view and a pool sized for one view is the wrong size for two. [INFERENCE: the per-view cost of a shadow-casting point light was not measured here.] The lighting a host can afford is therefore a budget to be **split**, not to be doubled by accident.

Real-Time Rendering's many-light rule, which the pool already applies per frame (`docs/research/pool-shadows.md`), is the budget's policy: spend the real lights on "the closest and brightest lights" and let the rest leak. For one viewer "closest" is a distance to one point. For several it has to be a distance to *the viewer that fixture serves*.

## Model adopted

`LightPool::focus: FocusRule`, with two values:

- `FocusRule::One` (the default, and what the crate did before): exactly one entity carries `F`. None leaves the pool dark; more than one leaves it dark and `warn_once!`s, now naming the setting that serves several.
- `FocusRule::Shared`: any number of entities carry `F`, one per viewer. The pool's `size` real lights are **shared**: each focus is entitled to `quota = ceil(size / foci)` of them, and what one focus does not use goes to the others.

How `Shared` ranks. For each focus `f` and each candidate `c` within `reach` of `f`, `v_f(c)` is the pool's own rank quantity measured from `f` (`d² / intensity` under `Rank::Illuminance`, the distance under `Rank::Distance`). A focus's **cutoff** is the quota-th smallest of its `v_f` (the largest, when fewer than `quota` candidates are in its reach). A candidate's value is

```
value(c) = min over foci f in reach of c  of  v_f(c) / cutoff_f
```

so a value at or below 1 means "inside the share of the viewer it serves", and the one ordered list the pool already runs on (the hand-off, the reserve cross-fade, the shadow budget, `PoolBudget`) interleaves the viewers in proportion without knowing there is more than one. Three properties follow from the formula and are the validation targets below:

1. **One focus is the same rule.** With one focus every value is divided by the same constant, so the order and every hysteresis ratio are unchanged. The pool skips the division then, and `Shared` with one focus is bit-identical to `One`.
2. **No viewer is starved by a denser one.** A focus with three fixtures near it holds all three while another with twenty holds `size - 3`, because the twenty's 9th and later fixtures have values above 1 and the three's all have values at most 1.
3. **A shared fixture is one light.** The minimum is taken per candidate, and the pool holds a candidate once.

`reach` is per focus: a candidate beyond it of every focus wants nothing.

## Parameters

| name | default | why |
|---|---|---|
| `LightPool::focus` | `FocusRule::One` | design default. A behaviour change in a released crate should be something a host can switch off (the precedent is `Rank`, `docs/research/light-pool.md`), and the one-focus rule is pinned by an existing test (`two_foci_mean_no_lights`) that this change must not loosen. |
| quota per focus | `ceil(size / foci)` | design default: an equal share, the simplest compromise that can be explained to an artist. Unused share is not lost: the ordering hands it to whoever has fixtures. A host that wants unequal shares (the player the camera follows closely against a minimap) has no knob yet; see `Deviations`. |

## Validation targets

- `Shared` with one focus assigns exactly as `One`, frame for frame, over a moving scene (test).
- Two foci, one with far more fixtures near it than `size`, the other with a few: the sparse viewer's fixtures all hold lights (test; fails under a plain nearest-focus distance).
- A viewer with fewer fixtures than its quota leaves the remainder to the other viewer (test).
- A fixture within reach of both holds one light (test).
- The assignment does not depend on the order the foci or the candidates were spawned in (test).
- `One` with two foci still holds nothing and still reports (the existing test, unchanged).
- A pool saved with `focus: Shared` reads back as it was written, and a file saved before the field existed loads as `One` (test, `serde`).
- Frame cost: `bench_lights` with `One` within noise of before; the multi-focus path's own cost reported for 1, 2 and 4 foci at 5 000 candidates.
- Courtyard in split-screen (`LIGHTS_SPLIT=1`, two cameras, each following its own focus): each half is lit around its own focus and reads as well as the single-view courtyard, and the two foci with `One` leave both halves dark (the before).

## What great looks like

A split-screen frame in which each half is a lit scene in its own right: the braziers and torches nearest each player burn with real light, a pool on the stone and a shadow behind a pillar, while the stone between the two players is allowed to be dark. Walking one player past a torch changes its light by a cross-fade and not a pop, exactly as in the single-view courtyard, and a player dropping out (the focus is removed) hands its share to the other without a flash. Neither half looks like the compromise it is.

## Deviations

- The quota is equal. A host with unequal needs would want weights per focus; `F` is a plain marker with no data, so a weight would have to come from a second component. Not built: there is no consumer asking, and an equal share is the rule that cannot surprise.
- `FocusRule::One` stays the default and keeps its reporting, rather than turning the pool multi-focus for everyone. A host that has two `F` entities by mistake is still told so.
- The pool's `size` is shared, not multiplied: a host whose budget is per view raises `size` itself (two views at 16 lights each is `size: 32`). The crate does not guess what the GPU can afford twice.

## Pending index

- The craft source is a web article, not a corpus document, and no corpus document covers lighting for several cameras at once (searched on 2026-10-31: split-screen, multi-camera, per-view, local multiplayer, over `s*`, `book*` and `PBR3*`). A cinematography text on multi-camera lighting (for television) should be imported when one is found; `paper_search` does not index such texts.
- A production write-up of how a shipped split-screen game budgets its lights per viewport was not found in the corpus or on the web (searched on 2026-10-31); the production evidence above is that such games are central to the field and that the engine renders several views as a matter of course. If one is found it should replace the inference "the lighting a host can afford is a budget to be split".
