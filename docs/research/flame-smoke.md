# Smoke: what a fire gives off above its flame

A flame is drawn, its light is real and its sparks climb (`flame.md`, `fire-light.md`, `flame-embers.md`), and above the tip the picture stops: the fire ends in a clean point against the dark. Every wood or pitch fire in a shipped game goes on up as smoke, and in the dark that smoke is visible for exactly one reason: the fire lights it. A lit column is also the one thing in a scene that shows which way the wind blows from fifty metres. This note is the model for `FlameSmoke`: a faint, fire-lit plume on one more camera-facing quad over the flame, a function of the clock alone, drawn with additive blending.

## Sources

| source | what it is | use |
|---|---|---|
| `10.7456_tojdac.1674124` — Doğaner, *The narrational role of lighting design in The Lights of the Dark City* (2025), after Milerson 2007 | craft: lighting that belongs to the scene, fire and "columns of light emitted in the atmosphere" | the atmosphere is what lets a source be *seen*; a lit column is a recognised practical-lighting subject |
| `10.13140_marples.nav.lighting.thesis` — Marples, *The Influence of Intrinsic Perceptual Cues on Navigation and Route Selection in Virtual Environments* (PhD thesis, Huddersfield, 2017), ch. 9.5.3 | craft and perception: torch sconces with "an animated flame and smoke effect" against a static glowing torch | smoke is a motion cue the viewer reads as a fire, measured (N = 76, p < 0.001) |
| `s2011-advances-pdf-slides` — SIGGRAPH 2011 *Advances in Real-Time Rendering in Games*, Fire/Flamethrowers/Thrusters | shipping production fire | the hot emission is an additive layer on top of a smoke layer, and a fire cross-fades into smoke over its life |
| `s2025-advances-materials-2` — SIGGRAPH 2025 *Advances*, the transparency (OIT) talk | shipping production: why sorted smoke is fragile | blended smoke sorts against everything else, and "manual sorting can result in a lot of side effects" |
| `10.5194/npg-21-269-2014` — Matulka, López, Redondo & Tarquis, *On the entrainment coefficient in a forced plume* (Nonlin. Processes Geophys. 21, 2014); abstract read through `paper_search`, not yet in the corpus | technical: a plume spreads linearly with height | the entrainment coefficient of Morton, Taylor & Turner (1956) "is about 0.13", which fixes how fast the column widens |
| `PBR3_11_Volume_Scattering` — *Physically Based Rendering* 3rd ed., ch. 11 | technical: smoke is a participating medium | "fog and smoke attenuate and scatter light": the column shines only by the light that reaches it |

## Craft principle

What the audience sees of a practical fire is the flame and the air it lights. Milerson, in Doğaner's reading of him, groups the light a story uses *as part of the scene* together:

> "The light emitted by fire, columns of light emitted in the atmosphere and appearing vertically, all kinds of lights from lightning to flashes that start suddenly and go out at the same speed, water or all kinds of surface reflections, all kinds of lights that are part of the scene and are part of the decor or lighting that gives the effect of light are included in this group." (`10.7456_tojdac.1674124`)

The air is what makes the source visible: a torch with nothing above it is a point, and the same torch with a faint lit column over it is a *fire*. Marples' navigation study put the difference to a test. Her exits carried torch sconces that either "glowed" or had "an animated flame and smoke effect", and:

> "The results demonstrated a difference between groups with regards to a preference for a path with a motion cue" (χ²(1, N = 76) = 19.000, `10.13140_marples.nav.lighting.thesis`)

The viewer reads the animated fire with its smoke as a living source and goes to it. A fire without smoke is the same fire with one cue taken away.

## How shipping games do it

The production fire the crate's flame and embers already follow draws its heat as an additive layer on top of its smoke:

> "Particle comp pass converts hotness to a blackbody color texture, adds it on top of smokey particles renderered with usual alpha blending." (`s2011-advances-pdf-slides`, Fire/Flamethrowers/Thrusters)

and ends each particle in smoke, "Crossfade to a smokey particle over the lifetime." That smoke is alpha-blended, and alpha-blended transparency is the weak point of a forward renderer; SIGGRAPH 2025's OIT talk, on sorting VFX, says:

> "Environments are usually well behaved and do not abuse physical logic of sorting. Same can't be said about VFX where the final effect is often achieved by smoke and mirrors." (`s2025-advances-materials-2`)

and of hand-ordered emitters, "manual sorting can result in a lot of side effects ... smoke tendrils look buggy". Two things carry over to a billboard on a crate that cannot own the host's transparency pass. **The glow layer needs no sorting**: additive blending is order-independent, which is why the flame and the sparks are additive, and a smoke that only *adds* the light the fire puts into it never fights another transparent surface for its place in the draw order. **Smoke is lit from below by the fire and by nothing else at night**, so at night the additive layer is the whole picture: the unlit, occluding half of a smoke particle is invisible against a dark sky.

## Model adopted

A third quad per fire, a child of the flame like the sparks' (`FlameEmbers`), with the same geometry: `RISE_MAX` 2.6 flame heights up, `HALF_WIDTH` 0.7 either side and the same margin, so it takes the fire's catch and the wind's lean from the flame's instance scale and **reuses the ember quad's culling box** (`ember_cull_extents`: nothing about the smoke's reach differs from the sparks'). It is drawn as a pure function of the shared clock, like the sparks: no state, no buffers, one draw per fire that asks, nothing for a fire that does not.

- **Where the column is.** The centreline leaves the flame's tip and bends downwind: at rise `r` (in flame heights) it is `lean · (r + 0.8125 · max(r − 1, 0))`, which is the flame's own tip offset at `r = 1` (no gap between flame and smoke) and the sparks' reach, `1.5 · lean · 2.6`, at the top. A slow meander grows with height so a calm column wanders.
- **How wide it is.** A turbulent plume spreads linearly. With the entrainment coefficient `α ≈ 0.13` (Morton, Taylor & Turner, as quoted by Matulka et al.) a plume's top-hat radius grows as `b = 1.2 α z ≈ 0.156 z`, measured from the plume's virtual origin, which is the flame's middle (`0.35`): the half-width is `0.05 + 0.156 · max(r − 0.35, 0)` flame heights (0.40 at the top).
- **What it looks like inside.** Wisps: three octaves of value noise advected up the column, warped sideways by a slow curl that grows with height, and an edge ragged by the same curl. The structure follows the centreline, so a bent column is bent *all the way through*.
- **How bright it is.** Lit by the flame: `1 / (1 + ((r − 0.35) / 0.7)²)`, an inverse square softened at the source, so a column is bright above the tip and gone by the top. Coloured by the fire's own hue (the sparks' orange stop going to the dull-red one with height, as the sparks cool), pulled 35 % toward its own luminance so it reads as warm grey smoke and not as a second flame. Faded in over the flame's tip (`r` 0.55 to 1.15) and out over `r` 1.4 to 2.6.
- **Whose light.** `FlameLook::strength` and the fire's catch (`took²`, as the sparks) scale it, so a fire that is lit grows its smoke as it catches and one that is put out stops. `FlameSite::depth_fade` applies: a fire whose source cannot be seen sheds no smoke that can.

## Parameters

| parameter | value | source |
|---|---|---|
| `FlameSmoke::density` | `0.0..=1.0`, default 0.6 | design default: the strength at which the column reads at 3 m and is a haze at 12 m (judged in captures) |
| plume half-width | `0.05 + 0.156 · (r − 0.35)` flame heights | Morton, Taylor & Turner via Matulka et al. (`α ≈ 0.13`, `b = 6α z / 5`) |
| virtual origin | the flame's middle, `0.35` | the light's own height (`flame_light_offset`) |
| rise speed | 0.32 flame heights a second | design default |
| lit falloff | `1 / (1 + ((r − 0.35) / 0.7)²)` | inverse square (PBR ch. 11: smoke scatters the light that reaches it), softened at the source |
| downwind lean | `lean · (r + 0.8125 · max(r − 1, 0))` | design default; `r = 1` is the flame's tip, `r = 2.6` the sparks' reach |
| quad extents | the ember quad's (`RISE_MAX` 2.6, `HALF_WIDTH` 0.7, margin 0.35) | `flame-embers.md`, `flame-culling.md` |
| `FireKind::smoke()` | candle 0.2, lantern none, torch 0.8, brazier 0.4, campfire 0.8, bonfire 1.0 | design default: pitch and wood smoke, a clean wick and charcoal smoke less |

## Validation targets

1. A fire with no `FlameSmoke` has no extra entity and no extra draw, and its flame, light and sparks are the same pixels as before (the default courtyard against the build before this change: zero differing).
2. The smoke quad is the flame's child, takes the catch and the lean, and carries a culling box of its own that is never smaller than what its vertex stage draws (the same port-of-the-vertex-stage test as the sparks'), so a fire's smoke is not culled while any of it is on screen.
3. The material is additive (`AlphaMode::Add`): it joins no sorting.
4. The column starts at the flame's tip and bends with the wind: at a 3 m/s wind the plume's centreline at the flame's tip is within a flame width of the flame's tip, and downwind of the foot at every height.
5. **At night it reads as lit smoke, not a second flame**: at 3 m it is a soft warm-grey column with a ragged edge that fades into the dark before its top; at 12 m a faint haze over the fire; never a hard-edged shape, never brighter than the flame's own body.
6. A fire of another colour has smoke of that colour (`FlameSite::color`).
7. Cost: with every fire smoking, `bench_flames`' off-screen cost is unchanged (culled), and an on-screen smoke costs fragment work only inside the column (a pixel outside the plume's lateral extent returns before any noise).

## What great looks like

The brazier at 3 m with a gentle wind: a column that is brightest just above the flame, a warm grey like the stone it will light, with a billowing, ragged edge, bending downwind more the higher it goes and thinning into the dark, never reaching the top of the frame as a block. From the ground at 9 m, the braziers' columns are a lit haze that makes the courtyard feel *burning*, and a torch-bearer's small fire has a faint wisp. In the wind the whole courtyard leans one way, flames, sparks and smoke together, and the smoke shows it from the far side. Not a cartoon puff, not a flame-coloured second flame, and never a hard edge where the quad ends.

## Deviations

None from an earlier default: the fire's flame, light and sparks are untouched, and `FlameSmoke` is opt-in (a site without it draws exactly as before). The smoke's colour, density and speed are design defaults, judged in captures.

Deliberately not built: an occluding (dark) half of the smoke. At night it cannot be seen against the sky, and in daylight it needs alpha-blended draws that sort against every other transparent surface; the OIT talk above is the reason this layer is additive only. A host that wants dark day smoke draws its own.

## Pending index

- `10.5194/npg-21-269-2014` (Matulka et al. 2014): read through its abstract in `paper_search`; `paper_download` and `distill_index` are pending, and the plume width law is quoted from it second-hand (Morton, Taylor & Turner 1956, whose paper is not in the corpus).
- A photometric measurement of how bright a wood fire's smoke is against its flame: not found; `density` is a design default.
