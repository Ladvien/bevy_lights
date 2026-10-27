# The flame's kindle: a lit flame must grow, and it does not

A `FlameSite` with a `kindle` is documented to grow its flame from nothing over that many seconds, on
the same `kindle_gain` envelope its light flares by. Measured on the crate's own showcase, it does not:
the flame appears at **full height on its first frame** and only brightens. This note records the
defect, why it matters to a game, and the model that fixes it.

## Sources

| source | what it is | use |
|---|---|---|
| `10.7456_tojdac.1674124` — Doğaner, *The narrational role of lighting design in The Lights of the Dark City* | a set light "mimics" the candle | the craft rule: the drawn source and the light it motivates are one object, and the audience checks them against each other |
| `s2007-advances-course-notes-0-4-mb` — Mitchell, Francke & Eng 2007, *Illustrative rendering in Team Fortress 2* | "Even when viewed only in silhouette with no internal shading at all, the characters are readily identifiable" | a source's **size** is its first read; a fire that ignores its own catch has the wrong size |
| `s2011-advances-pdf-slides` — SIGGRAPH 2011 *Advances*, Fire/Flamethrowers/Thrusters | "Ramp the alpha to fade in/out over particle lifetime"; "shrink particles as they fade" | production fire is born small and grows, and a fading fire shrinks: size is a lifetime channel, not a constant |
| `GPU Gems` ch. 6 §6.1 — Nguyen, *Fire in the Vulcan Demo* | "we placed emitters on the character. Each emitter can be adjusted for size, density …"; the 2D-flow flame "perfect for creating candle-like flames" | the shipped fire's size is an emitter parameter that changes, not a fixed sprite |
| `10.1017_jfm.2018.707` — Xia & Zhang, *A vortex-dynamical scaling theory for flickering buoyant diffusion flames* | a buoyant diffusion flame's length and its puffing are set by the fuel bed | the physical reason a young fire is short: a small burning area makes a short flame |
| `book_real_time_rendering_4th_akenine` §6.3 | the octave sum as turbulence | unchanged; this note touches only the flame's size, not its noise |

## Craft principle

The craft rule is the one `flame-flicker.md` already establishes from Doğaner: a set light *mimics* the
practical in shot, and the flame is the only part of that source the audience can see. Doğaner's line
is the whole argument:

> "A table lamp or street lantern on a set is not usually the main source of light used by the
> technical crew, as these sources may be inadequate or uncontrolled. However, these objects
> determine the motivation of the light in the scene. For example, if there is a candle burning in a
> room, the director and cinematographer use professional equipment that mimics the warm tones and
> soft shadows created by the candle." (`10.7456_tojdac.1674124`)

An audience watching a fire being lit reads its **size** before its brightness. TF2's own statement of
why silhouette leads everything else is the general form of the same claim:

> "Even when viewed only in silhouette with no internal shading at all, the characters are readily
> identifiable to players." (`s2007-advances-course-notes-0-4-mb`)

Applied to a fire: a flame that is full-height on the frame its light is at zero is a source that
**precedes** its light. That is the one relationship `lighting-look.md`'s motivated-source rule
forbids, and it is the direction the audience can check by eye, because both ends of it are on screen.

## How shipping games do it

Production fire treats size as a channel that moves over the fire's life, not as a constant:

> "Spawn lots of fairly short lived particles randomly from polygon cloud shapes with uniformish
> density. Ramp the alpha to fade in/out over particle lifetime." (`s2011-advances-pdf-slides`)

and, for the other end of the same life:

> "shrink particles as they fade" (`s2011-advances-pdf-slides`)

Nguyen's shipped fire goes further back — the emitter itself is what is sized:

> "Using our modeling tool, we placed emitters on the character. Each emitter can be adjusted for size,
> density (that is, particles per second) …" (`GPU Gems` ch. 6)

So the shipping pattern is: a fire's drawn extent follows its state. A catch is exactly a state the
crate already models (`kindle_gain`), so the flame's extent is the one place that state is not reaching.

## The defect, measured

`update_flames` writes `transform.scale = kindle_gain(...)` every frame. The vertex stage sizes the
billboard from the `MeshTag`'s height, **not** from the instance transform:

```wgsl
let size = tag.y * material.params.y;
let billboard = origin + right * (vertex.position.x * size * FLAME_ASPECT)
                     + up * (vertex.position.y * size + size * 0.5);
```

The transform's scale reaches the shader only through `length(world_from_local[0].xyz)`, which is read
as `took` and used to mix the ember colour in. It is never multiplied into the size. So the flame's
drawn height is constant through the catch; only its colour moves.

Measured on a flame spawned mid-run with `kindle: KINDLE_SECONDS` (`/home/ladvien/l18/kindle2/`, one
frame per capture from the spawn, `LIGHTS_SHOT_EVERY=1`), against a brazier-sized light with
`Kindle::default()`:

| frame after spawn | flame bbox height | flame width | flame area | peak luminance |
|---|---|---|---|---|
| 6 | 155 px | 68 px | 7006 px | 120 |
| 8 | 155 px | 69 px | 6988 px | 132 |
| 12 | 168 px | 69 px | 6990 px | 148 |
| 20 | 160 px | 70 px | 5012 px | 163 |

The height and the width are **flat** (the variation is the silhouette's own noise, ±6 %), the area
*falls* as the ember ramp darkens the rim, and the only number that rises with the catch is the peak
luminance. The crate's own showcase shows the same thing at the frame the braziers light: the flame's
bbox is 192 px tall on its first captured frame and 210 px once settled, a 9 % change that is the
`LIGHTS_POOL` hand-off rather than a growth.

The unit test `a_flame_lit_with_its_fire_grows_from_nothing_and_stops_at_full` passes anyway, because
it reads `transform.scale`, which *is* being written — the test pins the CPU-side value and cannot see
that the shader ignores it. That is the trap this note exists to close.

## Model adopted

`FlameSite::kindle`'s catch multiplies the **drawn size**, in the shader, where the size is computed:

```wgsl
let size = tag.y * material.params.y * in.took;
```

`took` is already in the vertex stage (`length(world_from_local[0].xyz)`, the instance scale), already
flat-interpolated, and already the exact number `update_flames` writes from `kindle_gain`. No new tag
bit, no new uniform, no second per-flame channel: the one number that was reaching the fragment now
also scales the quad.

The CPU side keeps writing `transform.scale`, because `took` **is** that scale — the crate's existing
per-flame channel, and removing it would need a tag bit the crate does not have.

## Parameters

| parameter | value | source |
|---|---|---|
| size over the catch | `kindle_gain(s)` × the site's `height`, `s = kindled / kindle` | `fire-light.md`'s envelope, Sanctuary decision 0127; applied to size here |
| width over the catch | the same factor (a flame is a billboard; width and height scale together) | design default: a flame that grew tall but stayed wide would read as a sheet (`flame.md`) |
| a site with no `kindle` | `took = 1`, size unchanged | existing behaviour; the common case costs nothing |

## Validation targets

- A flame whose site has `kindle: KINDLE_SECONDS` and whose `Flame` has just spawned draws at under
  20 % of its full height, and at full height once the catch has run. Measured as a **bbox in a
  capture**, not as `transform.scale`: the defect this note records is exactly the gap between those
  two.
- A site with `kindle: 0.0` draws at full size on its first frame.
- No flame's drawn height exceeds its site's `height` by more than the catch's own overshoot
  (`kindle_gain`'s 1.35 peak at ~85 % of the envelope), and none is zero.
- Brightness, phase, colour, flicker rate and clipping are unchanged by the fix.

## Deviations

- **The catch also scales the flame during the *overshoot*.** `kindle_gain` peaks at about 1.35 before
  settling to 1.0, so a flame that is catching is briefly **taller than its settled height**, by the
  same 35 % its light is briefly brighter. That is the same event in the other channel and is kept
  deliberately: `fire-light.md` calls the overshoot "the sound cue's visual half", and a light that
  flares while its flame holds its size would be the same incoherence in miniature.
- **The size is set in the vertex stage from the transform, not from a new tag field.** The `MeshTag`
  is full (height 16 | phase 8 | brightness 8), and `flame-silhouette.md` records that spending bits
  there is an allocation decision. `took` already carries the number.
- **Nothing in the physics is new.** Xia & Zhang give a flame's length as a function of its fuel bed,
  not of time since ignition; the catch's envelope remains Sanctuary's design default, as
  `fire-light.md` records. What this note adds is that the envelope reaches the *drawn* size, which it
  did not.

## Pending index

- A source that measured a **flame's own height against time after ignition** would replace
  `kindle_gain`'s shape with a measurement. `flame.md`'s `Pending index` already records this gap;
  Maynard 2013 (which measures light, not flame height) is unobtainable, and the corpus has no
  substitute. Unchanged by this iteration.
- `10.1017_jfm.2018.707` was re-read for a flame-length law and has none: it derives the **puffing
  frequency** from vortex shedding, and its length arguments are about the vortex sheet, not about a
  growing fire. The flame-length correlation this crate would need (Heskestad's `L/D` against the heat
  release rate) is **not in the corpus**; a search for `Heskestad` returns nothing. Listed here rather
  than cited, so no default rests on a number from memory.

## What great looks like

Judged at the crate's own showcase camera and at a close camera, on a frame sequence through the
catch:

- **A lit fire grows.** On the frame its light is at zero the flame is a point; at the end of the catch
  it is the height the host asked for. Read across the frames, the height rises monotonically with
  the light's intensity.
- **The flame and its light arrive together.** At every frame of the catch, the flame's drawn height
  and the light's intensity are the same fraction of their full values.
- **The overshoot is visible.** Around 85 % of the envelope the flame is briefly larger than it settles
  to, and the light is briefly brighter — the flare reads as one event in two channels.
- **Nothing else moves.** The settled flame is pixel-identical in size, shape, colour and brightness to
  the flame the crate draws today; the noise, the ramp and the flicker are untouched.
- **The base stays seated.** A shrinking flame must keep its foot at the site's origin, not sink into
  it or float above it.
