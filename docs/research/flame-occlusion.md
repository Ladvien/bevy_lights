# A flame whose torch is hidden: fade it by how much of its source the camera can see

Issue #2 (Sanctuary playtest reports 1792667477 and 1792667613): "What's this little flame here? Is that a torch on the other side of the wall?" A flame is a billboard drawn at its site. The depth test already hides the *fragments* behind a wall, but a flame is taller than the thing that holds it, so when the torch's body is behind a wall edge the flame's upper half still draws over the wall: a lone flame in the dark with nothing under it. The player asks "is something on fire there?" because the picture says so. The torch is the motivation (the flame's job, in the crate's own words, is to be the visible source of its light); a flame whose source cannot be seen has stopped being a source and become a fire in the world.

This note decides how a flame learns that its source is hidden. Scope: issue #2 ask 1 (`FlameSite::depth_fade`, the "smaller version" the issue offers). Ask 2 (`Flame::source`) shipped in iteration 113; ask 3 (`visible_from`) is deferred and recorded under Deviations.

## Sources

| source | what it is | use |
|---|---|---|
| `10.7456_tojdac.1674124` — Doğaner, *The narrational role of lighting design in The Lights of the Dark City* (2025), section "The Source of Light", quoting Bordwell and Thompson | craft: the visible source is what the viewer reads to find where the light comes from | why an unattached flame is wrong: it motivates nothing |
| `GPU Gems` (Fernando, ed., 2004), ch. 29, Sekulic (Croteam), "Efficient Occlusion Culling", section 29.8, *Serious Sam: The Second Encounter* lens flares | production: how a shipping game decided whether a glow on a light source is visible | the one-point depth test at the source's screen location and depth, and the point that a flare is "completely visible or completely invisible" because it is a lens effect |
| `bevy_pbr-0.20.0/src/prepass/utils.wesl`, `prepass_depth`; `render/mesh_view_bindings.wesl` (binding 20) | technical: the depth the main pass can read when a camera carries `DepthPrepass` | the read the shader does; the `DEPTH_PREPASS` shader def is the opt-in |

## Craft principle

The craft rule is that a visible source *motivates* the light, and that the audience reads it to place the light. Doğaner, on Bordwell and Thompson: "these objects determine the motivation of the light in the scene. For example, if there is a candle burning in a room, the director and cinematographer use professional equipment that mimics the warm tones and soft shadows created by the candle." And, a few lines on: "visible sources not only manage the viewer's perception, allowing them to logically find where the light comes from". A flame is the crate's visible source. A candle whose holder is behind a wall edge is not seen at all; a viewer who sees the flame without the holder reads fire, not candle. So when the holder is hidden the flame must be hidden too.

## How shipping games do it

Croteam's *Serious Sam* flares test one point: "We can use the occlusion query feature to test the visibility of a lens flare by 'rendering' one pixel (with the depth test enabled, but with depth writes and color writes disabled) at the screen location and depth of the lens flare's light source. Next, we check to see how many pixels passed the occlusion test. If one pixel passed, the lens flare is visible." The result is binary on purpose: "lens flares aren't just nice light-halos around light sources: they are a camera's lens effect, which means they cannot be partially occluded by objects in a scene. Therefore, a lens flare is either completely visible or completely invisible." That is true of a lens artefact and false of a flame, which is an object in the world: the moment a wall edge crosses a torch the flame ought to *go out of sight gradually*, not pop. The same chapter's costs are the other lesson: a depth *readback* to the CPU stalls the pipeline ("reading from the depth buffer will inevitably induce a stall"), so the test belongs on the GPU, in the pass that already has the depth.

## Model adopted

A **screen-space soft occlusion at the flame's source point**, in the flame's own fragment shader, with no CPU state.

1. The vertex stage projects the flame's source point: the flame's foot, lifted by a third of its drawn height (where the fuel burns, not the quad's base), and a second point offset sideways by a fraction of the height. These give the pixel to test and a tap radius in pixels that scales with how large the flame is on screen. Both are flat across the quad.
2. The fragment reads the depth prepass at 16 taps on a Vogel disc of that radius (radius `sqrt((i + 0.5) / 16)`, turned by the golden angle each step), so every tap has its own x offset: a vertical wall edge, which is what a wall end is, flips them one at a time. (A first draft used the centre and two rings of eight, whose taps are 4-fold symmetric and flip in lumps of 2-4: measured per-frame steps of up to 0.26 on a pan, against 0.26 at best for the disc too, see Validation; the disc's win is the plateaus break up, 0.15 and 0.10 steps instead of 0.4.) A tap is *occluding* when the scene's depth is nearer than the source point's by more than `FADE_MARGIN` (15 % of the flame's drawn height, in linear view distance via `depth_ndc_to_view_z`; issue #6 — the first draft's 3 % of reverse-Z depth is a small distance for a perspective camera but 135 ft for an orthographic one with `far` 6000): a bracket, the wall the torch hangs on, or the torch's own head are at the source's depth or behind it and must not count.
3. The flame's emission is multiplied by the fraction of taps *not* occluding, run through `smoothstep`. A wall edge sliding across the source takes the flame down in steps of 1/16 smoothed, over as many pixels as the ring is wide; it never pops. A flame behind a wall edge by more than the ring radius is at zero.
4. The reach is opt-in twice. A site asks with `FlameSite::depth_fade`; and the host's camera must carry `DepthPrepass`, because without it the `DEPTH_PREPASS` shader def is absent, the depth read does not exist in the compiled shader and the flame is exactly today's. A site that does not ask draws with the shared material whose `params.z` is 0: the same instances and the same draw call as today, one uniform branch in the fragment.

No CPU raycast: 393 flames in a night frame would be 393 rays against the scene's meshes every frame, and the depth the pass has is the scene the player is looking at, including geometry the host never tagged as an occluder.

## Parameters

| parameter | value | source |
|---|---|---|
| source point height | 0.35 of the drawn flame height | design default: the fuel burns in the lower third |
| taps | 16 on a Vogel disc of radius `FADE_RADIUS` = 0.10 of the flame's height (a first value of 0.15 left a flame beside a wall end at 45 % for a metre of camera travel) | design default |
| radius clamp | 2 to 24 px | design default: under 2 px the taps coincide; over 24 px the ring samples unrelated geometry |
| `FADE_MARGIN` | 15 % of the flame's drawn height, in view-space distance (`scene_z > source_z + 0.15 * size`) | design default (issue #6; was 3 % of reverse-Z depth) |
| fade shape | `smoothstep(0, 1, visible fraction)` | design default |

## Validation targets

- **Default is unchanged.** A site without `depth_fade` has `params.z == 0` and the dressed flame carries the unfaded material; asserted in a test, and the capture of the courtyard with the switch off is compared pixel-for-pixel against the baseline.
- A flame whose source point is behind a wall: brightness at 0 (render measured, the flame's pixels over the wall under 2 DN above the wall's own).
- A flame beside the wall, unoccluded: within the run's spread of the baseline.
- A wall edge crossing the source in a frame sequence: the flame's luminance is monotone and has no step larger than 1/9 of its full luminance between adjacent frames at a slow camera pan.
- A torch mounted flush on a wall (the wall behind the source point) is not faded: the margin test.

## Deviations

- **Soft in space, not eased in time.** The issue asks for a fade "over `FlameLook`'s own short envelope". A shader has no memory, and a temporal envelope needs the CPU to know the occlusion, which is the raycast this note rejects. The spatial softness gives the stability the issue wants (no flicker between occluded and not when the camera is still) and a smooth transition when the camera or the occluder moves. A fully occluded flame that clears does return at full strength within the ring's width of motion, not a fixed time.
- **`FlameSite::visible_from(camera)` is not built.** It needs a CPU-side visibility answer; the depth lives on the GPU. Left to a host with a ray.
- **The fade is not a time envelope, and is steppy in sixteenths.** Measured above: no pop, but a 0.26 single-frame step at the worst. Tried 32 taps: the worst step went 0.25 to 0.20 on a threshold-free measure and the same step sits at the same camera x at half the pan speed, so it is a property of where the wall's depth edge falls against the taps, not of tap count or camera speed; cause open, hypothesis: sample 0 of the MSAA prepass flips whole pixel columns at a silhouette. Kept at 16.
- **A multisampled view reads sample 0** of the prepass depth. At the margin used this decides the same answer as any other sample except on a silhouette edge, where the fade is smoothed anyway.
- **`FADE_MARGIN` and the source height are design defaults**, not measured against a source.

## Pending index

Distill was down for search on 2026-10-27 (`gpu busy: 4994 MB free < 5000 MB required to load bge-m3`); the sources above were found by `grep` of the local corpus. No note was indexed. `10.7456_tojdac.1674124` and the GPU Gems stem are already in the corpus.

## What great looks like

A torch behind a wall edge goes dark with its holder, in a handful of pixels, with no stray tongue of fire over the wall. A torch in the open is unchanged. Walking past a pillar, a flame ducks behind it the way the candle would, smoothly, and returns on the other side. A bracket-mounted torch against its wall never dims.

## Amendment: iteration 133 (issue #6, orthographic cameras)

The margin was `scene_depth > source_depth * 1.03` on reverse-Z NDC depth. For a perspective camera depth is `near / distance`, so 3 % of it is about 3 % of the distance; for an orthographic camera depth is linear in distance and 3 % of 0.75 at `far` 6000 is 135 ft, so the fade never fired (Sanctuary's measurement on the issue). Both depths are now converted with Bevy's projection-aware `depth_ndc_to_view_z` and compared in view units with a margin of 0.15 of the flame's drawn height (design default, which keeps a torch 5 cm proud of a wall unfaded). `examples/occlusion.rs` takes `LIGHTS_ORTHO=1` (`LIGHTS_FAR`, default 6000). Captured 1280x720: with `depth_fade` on, the flame over the wall is gone at `far` 6000 and 1500 (max luma 13.7, the sky, against 158.8 with it off; wall 23.3), the torch on the wall's face is unchanged in both projections, and the perspective result is unchanged to the eye (behind-wall flame 171 → 24 max luma).

### Orthographic pan, per frame (iteration 134)

`occlusion` with `LIGHTS_ORTHO=1 LIGHTS_PAN=1`, 1280x720, a fixed 1/60 s step, every frame from 130 to 240, fade on against fade off. The ratio is the orange excess (r − 1.2·b − 8, summed over the left of the frame, where the behind-wall flame is) of one run over the other, so the pan's own geometry cancels. The flame's share goes 0.77, 0.70 (held 5 frames), 0.54, 0.45, 0.34, 0.32, 0.21, 0.08, 0.00 and stays 0.00 for the 35 frames measured after: gone in about 20 frames (0.33 s) with a largest single-frame step of 0.16, under the 0.26 the perspective pan measured above. Plateaus of 2 to 5 frames are the pan moving less than a pixel per frame, not the fade holding: the fade-off series has the same pairs. No pop and no re-appearance. The fade is still spatial, not a time envelope.
