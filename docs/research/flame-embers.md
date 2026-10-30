# Embers: the sparks a fire throws up

A flame is the fire's body; the sparks are what tell the viewer it is *burning something*. A brazier or a torch at night in a shipped game sheds a few bright points that climb, drift downwind and go out. The crate draws the flame (`flame.md`), lights the ground with it (`fire-light.md`), catches, relights and snuffs it, and leans it in the wind, but the fire has no sparks: every flame is a clean teardrop, which reads as a lamp. This note adds them, opt-in, as a second billboard per flame with no particle state.

## Sources

| source | what it is | use |
|---|---|---|
| `10.7456_tojdac.1674124` — Doğaner, *The narrational role of lighting design in The Lights of the Dark City* (2025) | craft: the visible practical motivates the scene's light | a source the audience can see must be believable to carry the light it motivates; a spark is the cheapest evidence the fire is real fuel |
| `s2011-advances-pdf-slides` — SIGGRAPH 2011 *Advances in Real-Time Rendering in Games*, Fire/Flamethrowers/Thrusters | shipping production fire | how a shipped fire draws its particles: streaks along the velocity, a lifetime cross-fade, colour from temperature |
| `sig2024_Scintilla_Simulating_Combustible_Vegetation_for_Wildfires` — Scintilla (SIGGRAPH Asia 2023, ACM TOG), Sec. 5.7 *Sparks and Embers* | technical: an ember's temperature and trajectory | Newton cooling plus Stefan-Boltzmann loss, drag, and the wind's effect on the spread |
| `GPU Gems- Programming Techniques, Tips and Tricks for ...` (GPU Gems 1, ch. 6, Nguyen, *Fire in the Vulcan Demo*) | production: why the flame is a procedural billboard and the smoke and sparks are particles | the crate's own flame already follows ch. 6.1; this note is its particle half, done procedurally |

## Craft principle

The craft rule the crate already rests on (`flame-snuff.md`, `flame-kindle.md`):

> "A table lamp or street lantern on a set is not usually the main source of light used by the technical crew, as these sources may be inadequate or uncontrolled. However, these objects determine the motivation of the light in the scene." (`10.7456_tojdac.1674124`)

A practical fire motivates the key light on the ground. The audience checks that motivation by eye, and what makes a drawn flame read as a *fire* rather than an emissive teardrop is its behaviour against the dark: points that leave it, rise, and die. A gaffer's brazier on a night exterior is fed with ember-throwing wood for exactly that reason. No cinematography stem in the corpus describes sparks specifically; the craft source here is the motivation principle above, and the specific look is an art-direction call recorded under `Deviations`.

## How shipping games do it

> "Spawn lots of high velocity particles, pick up nice motion from fluid sim. Use vertex shader to extrude particle along velocity to do motion blur/hide strobing. Crossfade to a smokey particle over the lifetime." (`s2011-advances-pdf-slides`, Fire/Flamethrowers/Thrusters)

and, on how the fire's colour is made:

> "Particle comp pass converts hotness to a blackbody color texture." (same)

Two things carry over. A fast spark is a **streak along its velocity**, not a dot, which hides strobing at 60 Hz. A particle's **colour and emission are a function of how hot it still is**, which falls over its lifetime. The production fire's particles are simulated on a GPU; this crate has neither a compute pass nor a particle buffer in its ownership, so the sparks are evaluated in the fragment shader as a pure function of time (`Model adopted`).

## Model adopted

Technical basis, from Scintilla Sec. 5.7:

> "m_e C_p_e ∂T_e/∂t = σ A_e (T_amb⁴ − T_e⁴) + h A_e (T_a − T_e) − ΔH_c dm_e/dt" (`sig2024_Scintilla_...`, ember temperature)

An ember loses heat by radiation (Stefan-Boltzmann) and convection (Newton). Ignoring the combustion term, a small ember's temperature relaxes towards ambient: `T(t) − T_amb ∝ exp(−t / τ)`. The shader keeps that shape for the *share of its heat*: `heat = exp(−2.2 · age / life)`, so a spark is at 11 % of its birth heat when it dies, colour and emission following (`blackbody_colour`'s ordering: white-yellow hot, red cooler, dull red last).

Scintilla also reports the particle count: "the particles released per kilogram of fuel consumed are around 10³", which says sparks are many and small, not few and large; and that "at higher wind speeds the embers are carried a longer distance" (Fig. 6). The wind carries a spark further than it leans the flame: the drift is `1.5 ×` the flame's lean per flame height of rise (design default).

The shader, `src/embers.wesl`:

- One quad per flame, a child of the `Flame` entity, so the fire's catch (instance scale `x`) and the wind's lean (`y`, `z`) arrive unchanged, as the flame reads them. A fire not yet caught sheds nothing (`took < 0.05` returns, and brightness goes with `took²`); a snuffed fire's sparks go with its scale.
- Camera-facing at every elevation, with every length in flame heights: a world rise `h` lands `h · cos(elevation)` up the screen and a lane toward the camera `z · sin(elevation)` down it, so the sparks are a plume from the side and a scatter over the bowl from above, with no separate overhead path.
- Sixteen slots; each emits one spark per cycle of `16 / rate` seconds, born at a hashed moment in the cycle's slack, so the rate is exact and the emission is not a metronome. A slot's spark is a function of `(slot, cycle, flame phase)`: start height 0.45–0.80 of the flame, lane ±0.11, depth ±0.15, initial climb 0.7–1.5 flame heights a second under drag 1.1 /s (so a spark tops out at about 1.1–2.0 flame heights above the foot), a sideways weave of 4–12 % growing with age, life 0.9–1.8 s.
- A spark is a capsule from `head − velocity · 0.05 s` to `head` (the production streak), with a `1 / (1 + (d/r)²)²` glow cut at 5 r. The radius is 0.007–0.014 flame heights, floored at 0.9 px; the energy a sub-pixel spark would have had is kept, so a far fire's sparks dim instead of aliasing.
- The depth fade is the flame's own, copied (`source_visibility` of the same source point), so a torch behind a wall sheds no visible sparks.
- Cost: one draw per fire that asked, a vertex stage, and per-pixel 16 slots with an early `continue` for each slot not alive or too far (a handful of capsule distances); no buffers, no CPU per-frame work except one material clock write per frame, and a tag write only when the rate changes.

## Parameters

| name | value | basis |
|---|---|---|
| `FlameEmbers::rate` | 3.0 sparks/s, `0..=8` | design default: a hearth's few visible sparks; 8 is the rate at which all 16 slots are always busy with a 1.8 s life |
| `SLOTS` | 16 | shader's budget per fire |
| life | 0.9–1.8 s | design default; Scintilla's embers burn out over seconds, a sparking hearth's visible ones over about one |
| climb 0.7–1.5 fh/s, drag 1.1 /s | | design default, trajectory shape per Scintilla Fig. 6 (decaying climb, wind-dominated at height) |
| streak | 0.05 s of its velocity | SIGGRAPH 2011 (motion blur along velocity); the 50 ms is a design default (about 3 frames at 60 Hz) |
| cooling | `exp(−2.2 · share)` | Newton/Stefan-Boltzmann relaxation, Scintilla 5.7; the 2.2 is a design default (11 % at death) |
| wind drift | 1.5 × the flame's lean | design default, direction of Scintilla Fig. 6 |
| radius | 0.007–0.014 fh, floor 0.9 px | design default |
| gain | 3.0 × the flame core | design default, judged in captures |

## Validation targets

- With `FlameEmbers` absent, a scene renders exactly as before (no extra entity, draw or write): covered by a test that a site without the component gets no quad.
- The sparks per second are the `rate`, to within the hashed birth jitter: counted in a capture sequence at 4 s.
- A spark is brightest at birth and falls monotonically (no pop); from frame to frame it moves at most its streak length.
- Colour runs yellow → red → dull red, never white at death and never a pink plate.
- A spark is visible against the flame's own light at ground level, at an elevated angle, from straight above, and at 30 m (sub-pixel floor: dim, not crawling).
- A torch behind a wall (depth fade on) sheds no visible sparks.
- Wind: a gust moves the sparks further downwind than the flame's lean.

## What great looks like

A brazier at night with two or three bright points in the air above the flame at any moment, each a short orange streak that climbs, wavers sideways and reddens as it rises, gone before it passes a second flame-height above the tip; a torch in the wind sheds them in a slanting line; from overhead they are a few points scattered over the bowl. The sparks never read as snow, as a particle fountain, or as confetti, and they never outshine the flame's core.

## Deviations

New feature: every number above that is not cited is a design default, marked. The two non-cited behaviours worth stating: sparks are **not** physically simulated (no collisions, no wind field, no lofting by the plume's updraft; only drag and a sideways weave), and the wind drift is the host's `FlameWind` read through the flame's lean, so a host with no wind sheds straight-rising, weaving sparks.

## Pending index

No new documents were added this pass. A craft source that describes sparks from a gaffer's or cinematographer's view (practical fire on a night exterior) is missing from the corpus; Doğaner's motivation principle stands in for it.
