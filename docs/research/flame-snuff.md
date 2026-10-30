# Snuffing: a fire that goes out must go out, not vanish

The crate lights a fire (`Kindle`) and re-lights one (`docs/research/flame-relight.md`), but it has no way
to **put one out**. A host that douses a torch, burns a brazier down or lets the rain take a lantern has to
remove the `Flicker` light and despawn the flame in one frame: the light the fire cast on the ground and the
flame drawn on it both vanish in a single frame. That is the one transition of a fire's life the audience
can check by eye at both ends, and it is the mirror of the catch the crate already gets right.

## Sources

| source | what it is | use |
|---|---|---|
| `10.7456_tojdac.1674124` — Doğaner, *The narrational role of lighting design in The Lights of the Dark City* (2025) | craft: a visible practical motivates the scene's light | the drawn source and the light it motivates are one object; the audience compares them |
| `s2011-advances-pdf-slides` — SIGGRAPH 2011 *Advances in Real-Time Rendering in Games*, Fire/Flamethrowers/Thrusters | shipping production fire | "Crossfade to a smokey particle over the lifetime": a production fire's end is a cross-fade of its size and its emission over a lifetime, not a cut |
| `10.1038_s41598-020-78229-x` — Gergely et al., *Flickering candle flames and their collective behavior* (2022) | technical: a candle's oscillation versus its oxygen supply | a starved flame stops flickering before it stops burning |
| `10.1017_jfm.2018.707` — Xia & Zhang (2018) | technical: flame size is set by the fuel bed | a flame that is failing is a smaller one; size is the channel that carries "dying" |

## Craft principle

The craft rule is the one `flame-flicker.md` and `flame-kindle.md` establish from Doğaner:

> "A table lamp or street lantern on a set is not usually the main source of light used by the
> technical crew, as these sources may be inadequate or uncontrolled. However, these objects
> determine the motivation of the light in the scene." (`10.7456_tojdac.1674124`)

The practical **motivates**, so when the practical goes out the lighting that it motivated goes out with
it, at the same rate, and the audience watches the two agree. A flame that is gone while its light still
lies on the stone, or the reverse, is the one disagreement the viewer can check.

## How shipping games do it

> "Crossfade to a smokey particle over the lifetime." (`s2011-advances-pdf-slides`)

Production fire has a lifetime whose end is a *fade*: emission falls and the extent shrinks together, and
what is left is smoke. A torch that is doused in a shipped game dims and shrinks over about a second and
then is a dead prop, with a message the game turns into a hiss and a smoke puff. The crate owns the dim
and the shrink; the sound and the smoke are the host's, so it announces the end (`Snuffed`) as it
announces the start (`Kindled`).

## Model adopted

`Snuff { elapsed }` on the light the crate already modulates (a `Flicker` light). The crate advances
`elapsed` on virtual time. While it runs, the light's intensity is multiplied by `snuff_gain(elapsed)` and
the flame linked to it (`FlameSite::fire`) is drawn at the same factor, the way a linked `Kindle`
scales it, so light and flame leave together. At `SNUFF_SECONDS` the gain is exactly 0, the crate
writes `Snuffed` once, and the fire stays out until the host removes `Snuff` (and inserts `Kindle` to
re-light it).

`snuff_gain(t) = 1 - s²(3 - 2s)`, `s = clamp(t / SNUFF_SECONDS, 0, 1)`: a smoothstep that holds the fire
near full at the start (the first instant after a dousing still reads as lit), falls fastest mid-way, and
arrives at 0 with no slope, so the last glow does not click off.

Gergely et al. observed that "for `n0 < 0.17` the oscillation stops, and for `n0 < 0.15` the flame also
stops": a starved flame goes **steady before it goes out**. The flicker is therefore left running through
the fall (the shaped fire is what the audience is watching dim, and a held flicker amplitude on a falling
mean reads as a dying flame), and no extra "gutter" wobble is added: the physical sequence is a quieting,
not a stutter.

## What great looks like

A doused brazier: the ground pool of warm light and the flame fall together over a second, the flame
shrinking toward its base, the last 10 % of the fall landing softly, and the stone ending in the cool
moon fill with no warm residue and no frame where one of the two is gone and the other is still there.

## Parameters

| parameter | value | source |
|---|---|---|
| `SNUFF_SECONDS` | 1.2 s | design default: a catch is `KINDLE_SECONDS`; a fire dies quicker than it catches, and a second is the length of a hand covering a flame |
| `snuff_gain` | smoothstep down, 1 at 0 and 0 at `SNUFF_SECONDS` | design default (the mirror of `kindle_gain`'s smoothstep, without its flare: nothing flares as it fails, per Gergely above) |
| flame size over the snuff | the same `snuff_gain`, floored at 0.001 as a catch is | `flame-kindle.md`: a flame's drawn size follows its fire's state |
| `Snuff` and `Kindle` together | `Snuff` wins | a fire put out is out; re-lighting removes `Snuff` |

## Validation targets

- `snuff_gain(0) == 1`, `snuff_gain(SNUFF_SECONDS) == 0`, monotone non-increasing in between.
- A light with `Snuff` reaches exactly 0 intensity and `Snuffed` is written exactly once.
- A linked flame's scale follows `snuff_gain` and is back at 1 once `Snuff` is removed.
- Captures: light mean over the courtyard ground and flame bbox height both fall together across the
  sequence.

## Deviations

None: nothing existing changes. `SNUFF_SECONDS` and the curve are design defaults, marked above.

## Pending index

None. The three kinds of source are in the corpus (craft Doğaner, production `s2011-advances-pdf-slides`,
technical Gergely and Xia & Zhang). A source measuring how long a doused flame takes to go dark was not
found; the duration is a design default.
