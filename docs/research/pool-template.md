# The pool's template: every setting on every light, live, and saved

The pool spawns its lights from one template, `LightPool::light`. Until iteration 272 two different rules decided what of it a light carried. A light **spawned** took the whole template (every field but its intensity and its shadow flag, which the pool sets itself); a light that **already existed** took only three fields when the template changed (`color`, `range`, `radius`), and a saved project kept only those three and the shadow flag. So a host that changed the template's shadow bias, its contact-shadow flag or its shadow near plane at runtime — what an editor panel does — changed nothing on screen, until the pool grew and the new lights came up with the new setting beside old ones that kept the old. And a project saved with a bias came back without it.

This note started as the next item of `pool-shadows.md`: a **penumbra** for the pool's shadows. What it found is that Bevy 0.20's two levers for a softer, better-grounded shadow (percentage-closer soft shadows, and screen-space contact shadows) are both **settings of a light**, that neither should be the crate's default (measured below), and that the pool could not carry either of them live. The crate's job is therefore to carry whatever the host puts in the template to every light it owns, on the frame it is put there, and to save it.

## Sources

| source | what it is | use |
|---|---|---|
| `10.23856_6105` — *Harmony of light and shadow* (portrait lighting; Hameed 2023's table) | the size of a source against its distance, and the shadow edge it gives | craft: how soft a torch's shadow should be |
| `10.32657_10220_48312` — a thesis on colour and light in the cinematography of Christopher Doyle and Gregg Toland (catalog has no title; read off the markdown) | hard direct light and the shadow it draws | craft: a hard shadow is a choice, not a defect |
| `s2025-advances-materials-7` — *Fast as Hell: idTech 8 Global Illumination* (Indiana Jones; DOOM: The Dark Ages), SIGGRAPH 2025 *Advances* | compositing its lighting | production: "contact shadows for every light" |
| `s2025-advances-materials-12` — Narkowicz & Costa, *MegaLights: Stochastic Direct Lighting in Unreal Engine 5*, SIGGRAPH 2025 *Advances* | screen traces for many lights | production: what a contact shadow usually costs |
| `10.1007_978-1-4842-4427-2` — *Ray Tracing Gems*, ch. 25 (SEED's hybrid renderer) | shadows in a shipped real-time renderer | production: shadows ground objects |
| `book_real_time_rendering_4th_akenine` — RTR4 §7 intro, §7.5-7.6, screen-space shadows (the close of §7.9) | PCF, PCSS, contact-hardening and screen-space shadows | technical: the penumbra model, and that any shadow beats none |
| `10.3758_bf03200497` — Meng & Sedgwick, *Distance perception mediated through nested contact relations among surfaces* | perception of contact with the ground | technical: a cast shadow tells resting from floating |
| `bevy_pbr-0.20.0/src/render/shadow_sampling.wesl`, `shadows.wesl`, `pbr_functions.wesl`, `render/light.rs`; `bevy_light-0.20.0/src/point_light.rs` | Bevy's point-light PCSS, contact shadows and the light's fields | the engine's levers and their scale |

## Craft principle

A shadow's edge is set by the size of the source against its distance from the subject. The portrait table in `10.23856_6105`:

> "if the size of the light source is approximately equal to or close to the distance from it to the subject, the lighting will be soft and uniform"

and, at the other end of the same table, a ratio of size to distance of 1:6 to 1:10 is "Sufficiently contrasty lighting with fuzzy shadow contours" and 1:10 is "Contrast lighting with clear shadow boundaries".

A torch's flame is about 0.1-0.15 m across and the figures it lights stand 1-3 m from it: 1:10 to 1:30, **clear shadow boundaries**. A brazier's fire (a 0.34 m bed under half a metre of flame) at 2-4 m is 1:4 to 1:10, fuzzy contours. So the hard edge of a pool torch's shadow is the craft's answer, not a defect, and the place a penumbra would show is under the big fires, which are the host's own lights. Hard light is also a cinematographer's choice in its own right; the Toland chapter of `10.32657_10220_48312`:

> "Toland's signature cinematography techniques involve the usage of direct hard lighting to create texture through shadow, to create depth and drama."

What the craft does not allow is two lamps of the same unit behaving differently on the same set. A gaffer who changes a fixture's setting changes it on every unit of that fixture: the pool's lights are one fixture, so a setting given to the template is a setting given to every light, at once.

## How shipping games do it

- **A shadow for every light, cheaply.** id Software's idTech 8 composites "Contact shadows for every light." (`s2025-advances-materials-7`, *Compositing Results*), on top of its shadow maps. Unreal's MegaLights team describes the usual form: "typical contact shadows where we can usually get away with a small number of taps and short ray length" (`s2025-advances-materials-12`).
- **Grounding is what shadows are for.** SEED's hybrid renderer (*Ray Tracing Gems* ch. 25): "ray traced shadows are great because they perfectly ground objects in a scene, handling both small- and large-scale shadowing at once."
- **The shadow's quality is a per-light setting.** Every engine above sets it with the light (Bevy too: `shadow_depth_bias`, `shadow_normal_bias`, `shadow_map_near_z`, `contact_shadows_enabled`, `soft_shadows_enabled` with `radius`, all fields of `PointLight`), so a light pool that carries only some of its template's fields hides the rest from the host.

## Model adopted

The technical side has two penumbra models and one grounding model.

- **PCSS** (RTR4 §7.6): the filter width grows with the distance from occluder to receiver, `w_sample = w_light (d_r − d_o) / d_o`, so "the width of the surface area to the sample grows as the average occluder gets farther from the receiver and closer to the light". RTR4's reason for it over PCF: a constant width "appears incorrect where there is ground contact between the occluder and receiver".
- **Screen-space contact shadows** (RTR4, the close of §7.9): march the camera's depth buffer toward the light; "while costly ... doing so can provide high-quality results for closeups".
- **Any shadow beats none** (RTR4 §7 intro): "More important than having a penumbra is having any shadow at all ... it is usually better to have an inaccurate shadow than none at all, as the eye is fairly forgiving about the shadow's shape." Meng & Sedgwick on why: "Cast shadows can provide effective perceptual information that something is floating above the ground rather than resting on it."

Bevy 0.20 implements both penumbra levers as fields of the light, so the crate's model is:

1. **Every field of `LightPool::light` but two is every pool light's**: `color`, `range`, `radius`, `contact_shadows_enabled`, `affects_lightmapped_mesh_diffuse`, `shadow_depth_bias`, `shadow_normal_bias`, `shadow_map_near_z`, and `soft_shadows_enabled` when Bevy's `experimental_pbr_pcss` gives the light one. The two that are not: `intensity` (each light's is its candidate's, through its `Flicker`) and `shadow_maps_enabled` (the shadow budget's, per light).
2. **Applied when the template changes, to every light, reserves included**, on the next frame, the rule the three fields already had: when the template's shared part differs from what was last applied, not when the resource is touched (a host writes `gain` every frame). A light already matching is not written, so its change detection is not touched.
3. **The comparison is the light's own `Reflect` equality** over the shared part, so a field the crate does not name (Bevy's feature-gated `soft_shadows_enabled`, or one a later Bevy adds) is carried without the crate needing a feature of its own.
4. **Saved with the project**: the serde template writes every shared field. `soft_shadows_enabled` exists only with Bevy's `experimental_pbr_pcss`, which this crate does not turn on, so it is read and written by name through `Reflect`; in a build without it, it is saved as `false` and ignored on load.
5. **Checked**: `LightPool::problems` names a `light.shadow_map_near_z` that is not positive (it is the near plane of each cube face's projection; at 0 the face's depths are all 0) alongside the existing `light.range` and `light.radius` checks.
6. **No default changes.** The template's defaults stay Bevy's: contact shadows off, soft shadows off, radius 0, Bevy's biases. *Iteration 272's measurements* below say why neither penumbra lever is a default a crate should pick for its host.

## Parameters

| parameter | value | source |
|---|---|---|
| fields carried live and saved | every `PointLight` field but `intensity` and `shadow_maps_enabled` | design default: the craft's "one fixture, one setting"; the two excluded are the pool's own per-light state |
| change detection | `Reflect` equality of the template's shared part against the last applied | design default: the existing rule (`light-pool.md`, *Deviations*), widened from three fields to all |
| contact shadows on the template | off (Bevy's default) | measured: dotted without TAA (below); a host turns them on with its camera's `ContactShadows` |
| soft shadows on the template | off, `radius` 0 (Bevy's defaults) | measured: no effect at a flame's radius, and a radius large enough to blur wipes the specular (below) |
| `shadow_map_near_z` | must be > 0 (a `Problem` otherwise) | Bevy builds each cube face's projection with it as the near plane (`render/light.rs:1323`, glam's `perspective_infinite_reverse_rh`, which asserts it is positive); at 0 every depth the face writes is 0 |

## Validation targets

- **Live**: with a pool of two lights and one reserve, a change of the template's `contact_shadows_enabled`, `shadow_depth_bias`, `shadow_normal_bias`, `shadow_map_near_z`, `affects_lightmapped_mesh_diffuse`, `color` and `range` reaches all three lights on the next frame, and each light keeps its own `shadow_maps_enabled` (test; fails before for the five newly carried fields).
- **One fixture**: a light spawned after that change (the pool grown by one) and a light spawned before it carry the same settings (test).
- **The tint rule holds**: writing `gain` every frame still leaves a slot's own colour alone (the existing `a_change_of_gain_leaves_a_slots_own_colour_alone`).
- **Saved**: a pool round-trips every shared field through JSON (test), and a project file naming only some of them takes the defaults for the rest (the existing partial-file test).
- **Checked**: a template with `shadow_map_near_z: 0.0` reports `light.shadow_map_near_z` (test).
- **No visible change** with the courtyard's settings: before and after captures differ only by GPU noise.
- **CPU**: `bench_lights` `pool_p50_us` within run-to-run noise (one `Reflect` comparison a frame).

## What great looks like

- An editor's light panel moves the pool's shadow bias, or turns contact shadows on, and every torch on screen answers on the next frame: none lags until a respawn, and a light that joins later matches the ones already burning.
- A project saved with a tuned pool reloads lighting exactly as it was saved.
- The pool's torches keep the clear shadow edge the craft gives a small flame at a few metres; a host that wants a softer or better-grounded look turns Bevy's levers on in the template and on its camera, and the pool carries them.

## Deviations

- **No penumbra in the crate.** `pool-shadows.md` listed one as its next item. Bevy 0.20's point-light PCSS scales its filter by a fixed `POINT_SHADOW_SCALE = 0.003` and clamps it at 0.5 (`shadow_sampling.wesl`), so at a flame's physical radius it does nothing measurable and a radius large enough to blur also widens every specular highlight (`radius` is the sphere-light size); its blocker search is scaled by the same radius in radians. Contact shadows dot a lit flat surface unless the camera runs TAA. Measured below; both stay host choices, carried by the pool.
- **`soft_shadows_enabled` is reached by name** through `Reflect`, not a crate feature: a cargo feature mirroring Bevy's would leave a hole (a host turning Bevy's on directly would get a template field the pool silently dropped).

## Iteration 272's measurements

Throwaway switches in the courtyard (patch kept with the pass's captures, not committed), built with Bevy's `experimental_pbr_pcss`, frame 120 at a fixed 1/60 s step, 1280x720 on the RTX 3090. The same configuration rendered twice differs by 0 DN, so every difference below is the setting's.

- **PCSS on the braziers and the pool** (overhead camera 9 m over the brazier at (5,0,5)): `radius` 0.15 against PCSS off changes the frame by 0.18 DN mean (1.4 % of pixels over 2 DN), and the pillar shadow's edge profile not at all (35 px against 34 px for the 90→10 % step across one column, with the light's own falloff in the step); the change is the wider specular of a 0.15 m sphere light. `radius` 2: 1.14 DN mean, 9 % of pixels; `radius` 10: 2.87 DN mean, 32 % of pixels, a visible penumbra that hardens toward the pillar's base, and the floor's specular hot spot beside the brazier falls from 130.5 to 108.8 (Rec. 709 luma, max in a 160x110 window). So the soft edge comes only with a 10 m "flame" and its highlight.
- **Contact shadows** (`ContactShadows { length }` on the camera, the flag on the pool's template): on a flat lit pillar face, 1.92 % of pixels are dots more than 15 DN under their 5x5 median at `length` 0.3 and 0.68 % at 1.0, against 0.00 % with none; with TAA (`TemporalAntiAliasing`, `Msaa::Off`, `ShadowFilteringMethod::Temporal`), 0.00 % at 0.5 and 1.0. On the braziers, whose bowls are `NotShadowCaster` because the light sits inside them, a screen-space march still finds the bowl in the depth buffer and draws a dithered ring round its foot, so a host would leave them off for a light inside its own fixture. A figure's foot gains a thin contact darkening; the effect at the courtyard's default camera is small.
- **GPU**: Bevy's render diagnostics drop the main pass in this scene (five shadowed lights), so the cost of either lever was not measured; `auxiliary_views` (the shadow maps) is unaffected by both.

## Pending index

- `10.3758_bf03200497` and `10.1007_978-1-4842-4427-2` are in the local corpus but `catalog_read` returns 404 for both; cited by stem and by the title on their first page, with no DOI.
- `10.32657_10220_48312` has no title in the catalog; described from its markdown.
- Fernando 2005 (*Percentage-Closer Soft Shadows*) is cited through RTR4, not read.
