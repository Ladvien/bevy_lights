# The smoke of a fire that has been put out lifts off and thins; it does not shrink with the flame

`FlameSmoke` (`docs/research/flame-smoke.md`) draws a fire's plume as a function of the flame's catch, so a fire that is put out (`Snuff`,
`docs/research/flame-snuff.md`) takes its smoke down with it: over the 1.2 s the flame shrinks the whole column collapses toward the foot
and is gone, and the frame after the flame is out has nothing above the bowl. Nothing in a real fire does that. Smoke that has left a
fire is carried on by the air, and a wick that has just been put out goes on making smoke for a few seconds. The trail is the cue that
tells the viewer *how* a fire went out and that it was lit a moment ago; without it the douse is the flame vanishing.

## Sources

| source | what it is | use |
|---|---|---|
| Oseman, *How smoke can make your shots look magical* (RedShark News, 11 Feb 2024; web, not in the corpus) | craft: a working cinematographer on what smoke does to a frame and how it is lit | smoke is read by the light that reaches it, and it is read best when it is lit from behind or the side; it adds "movement and texture" |
| `10.13140_marples.nav.lighting.thesis` (Marples 2017, lines 6152 and 8060-8206) | production and perception: a torch sconce "with an animated flame and smoke effect" against a static glowing one, a measured preference for the animated one | the smoke is part of what makes a practical read as a fire |
| `GPU Gems- Programming Techniques, Tips and Tricks for ...` (GPU Gems 1, ch. 6, Nguyen, *Fire in the Vulcan Demo*) | production: a shipped fire demo's smoke is a generator of particles that each carry their own life | a smoke generator that stops emitting does not erase the particles it has already made |
| `s2011-advances-pdf-slides` (SIGGRAPH 2011 *Advances*, Fire/Flamethrowers/Thrusters) | production: the end of a production fire is a cross-fade to smoke | "Crossfade to a smokey particle over the lifetime." |
| `GPU Gems- Programming Techniques, Tips and Tricks for ...` (GPU Gems 1, ch. 38, Harris, *Fast Fluid Dynamics Simulation on the GPU*) | technical: the standard smoke model, from Fedkiw, Stam and Jensen 2001 | smoke density is advected by the flow, so a source that is switched off leaves the smoke it made travelling on |
| Saig, *Relight my Fire* (Davidson Institute of Science Education, 21 Mar 2020; web, not in the corpus) | technical: what a wick does after the flame is gone | the wick goes on boiling wax into a plume of visible smoke for "the first few seconds" |

## Craft principle

The cinematographer's rule is that smoke is not lit by itself: it shows what lights it.

> "Since smoke also obeys the Rayleigh Equation, it shows up best when it's backlit, a little when it's side-lit and barely at all when
> front-lit." (Oseman, RedShark News)

and the corollary a gaffer lives by, that it carries a scene's light and also reveals it:

> "Beware that volumising the light can give away hidden sources that aren't well motivated." (Oseman, RedShark News)

A trail that is lit by nothing is a trail the audience cannot see, and one that is lit by something that is not in the frame is a source
nobody motivated. The lit trail of a put-out fire has an honest motive in this crate's scenes: the **coals and the wick**, which stay hot
and glowing long after the flame (the courtyard's braziers keep their red bed; a torch's brand smoulders), and the dying flame itself for
the first second. So the trail is lit by the flame while there is one, and afterwards by a glow from the source that is broader and far
weaker than the flame's own light (it does not fall off as sharply: it is a coal bed, not a point).

## How shipping games do it

A production smoke is a particle effect, and a particle effect has a *lifetime*, which is the property this feature needs:

> "We gave the smoke particles a rolling motion on the x axis so the smoke appeared to be folding into itself ... we created particles
> with a random percentage of difference for a number of attributes: size, position, rate of decay, animation start frame, global
> transparency, and so on." (`GPU Gems` ch. 6)

and the end of a production fire is a hand-over to its smoke, not a cut:

> "Crossfade to a smokey particle over the lifetime." (`s2011-advances-pdf-slides`)

A fire in a shipped game that is doused stops its flame and its emitter; the particles it has already emitted finish their own lives, so
a column lifts off the fire and fades as it goes. The crate's smoke has no particles (one quad, a function of the shared clock), so the
lift-off has to come from the same place the plume does: the clock, and the history of the source that the column carries along it.

## Model adopted

**Smoke density is advected** (`GPU Gems` ch. 38: "The smoke density is advected by the velocity field, just like the dye we described
earlier."), so the smoke at height `h` above the source left the source `age(h) = (h - source) / v` seconds ago, `v` being the speed the
column's wisps already travel at in `src/smoke.wesl` (0.7 / 2.2 flame heights a second). When the source is put out `τ` seconds ago, the
smoke now at `h` was made `τ - age(h)` seconds after the douse began: negative for everything above the line `h = source + v τ` (the old
column, which is simply carried up), and positive below it, where it was made by a source that is dying. That is the whole model, and it
is stateless: the shader needs only `τ`.

- **The emission history** is `1` before the douse and falls to `0` by `EMISSION_SECONDS` after it (`1 - smoothstep(0, E, τ_made)`):
  the wick smoulders on for a short while, and the smoke it makes is thinner each second. The base of the column therefore lifts off the
  fire at `v` with a soft lower edge `E · v` thick, and nothing is made below it. A weak source also makes a *thinner* wisp, not only
  a fainter one (a candle's trail is a thread, a torch's plume a column): the half-width of what a dying source made is scaled by
  `0.5 + 0.5 · emission`, so the base of the trail is narrow and widens up to the old column's width.
- **The base of the visible column drops with the flame.** The steady plume is hidden below `rise 0.55..1.15` flame heights (the flame
  occupies that space); as the flame sinks (`1 - snuff_gain(τ)`) that window slides down to the wick (`0.10..0.45`), so the thin wisp of a
  smouldering wick is seen leaving the wick and not a flame's tip that is no longer there.
- **The light on the column** is the dying flame's, `snuff_gain(τ)` times the steady column's inverse square, plus the coals' glow:
  `AFTERGLOW / (1 + (away / 2)²)`, a broad, weak law (`away` as in the steady plume), weighted by how much of the flame is gone.
  At `τ = 0` this is the steady column exactly. The first build had a glow of 0.30, which left the packet as bright at 3 s as the
  steady plume while the ground beneath it had gone dark (smoke lit by a dead fire: the one disagreement a viewer checks); 0.18 is
  about a third of the steady column's light at the heights the packet climbs through.
- **The end** is a fade over the last 70 % of `SMOKE_TAIL_SECONDS` (`1 - smoothstep(0.3, 1, τ / T)`), so the trail thins as it ages, is
  exactly dark when its clock stops and nothing pops.
- **The clock** is `SmokeTail::elapsed` on the smoke quad, advanced on **virtual** time (the douse is the fire's state, as the flame's
  snuff is, so a paused world's trail holds still while the wisps in it keep moving on real time, the split `AGENTS.md` describes), and
  begun when a `Snuff` is added to the light a smoking `FlameSite::fire` names. It ends when its time is up, or when `Snuff` is removed (the
  fire is re-lit: the plume goes back to following the catch).
- **The quad carries `τ` in the scale it inherits**, the way it already carries the catch: the flame's scale `x` is its catch, which is
  `0.001` once the flame is out, so the trail quad's local scale is set to make its *world* `x` equal `1 / (1 + τ)` (a number, not a
  size; no less than `1 / (1 + SMOKE_TAIL_SECONDS)`, so its culling box stays finite, and the shader inverts it with no copy of the
  trail's length), and one bit of the quad's tag (the low bit of its height, which a steady quad's tag always clears) says "read it
  as a trail". A trail is drawn at the burnt size (the catch reads as 1): the column is where it was, it does not shrink.

## Parameters

| parameter | value | source |
|---|---|---|
| `SMOKE_TAIL_SECONDS` | 5.0 s | design default: Saig, "the first few seconds"; long enough for the lifted base to leave the top of the column (`RISE_MAX - 1` flame heights at `v` ≈ 5 s) |
| `EMISSION_SECONDS` | 2.4 s | design default: twice the 1.2 s flame snuff (`SNUFF_SECONDS`), the wick smouldering a while after its flame |
| `v`, the lift speed | 0.7 / 2.2 = 0.318 flame heights a second | the steady column's own wisp speed in `src/smoke.wesl`, so the cut edge travels with the pattern |
| `AFTERGLOW` | 0.18 | design default: the coals' glow against the flame's unit light; judged in captures (0.30 was too bright, above) |
| the base window while the flame sinks | `0.55..1.15` → `0.10..0.45` flame heights | design default: from the steady column's to the wick's (`SOURCE` is 0.35) |
| the end fade | from 30 % of the tail to its end | design default |

## Validation targets

1. **Continuity**: the frame a douse begins is the frame the steady plume drew (the trail at `τ = 0` is the steady column, the clock having
   moved by one step), and a scene that never douses a fire draws **zero pixels differently** from the build before.
2. **The smoke outlives the flame**: three seconds after the douse begins, with the flame gone (`snuff_gain` 0 after 1.2 s), the column
   box still holds smoke: its mean luminance is at least a fifth of the steady column's.
3. **It lifts**: the lowest visible row of the smoke rises through the sequence at about `v` flame heights a second (the cut edge travels
   with the wisps).
4. **It ends at zero**: the frame after `SMOKE_TAIL_SECONDS` has no smoke in the column box and the frame before it has almost none (no pop).
5. **Pause**: a paused world's trail holds still; re-light ends it at once; a fire loaded as out draws none.
6. **Cost**: a scene with no douse costs what it did; a douse costs nothing beyond the one quad that was already there for the five
   seconds it lasts.

## What great looks like

A douse the viewer reads: the flame sinks into the bowl over a second, the sparks stop, and a thin warm-grey wisp goes on rising from the
coals, its lower end lifting off and fading as the rest of it climbs, bending downwind in a breeze, until a few seconds later the air above
the bowl is empty. No frame in which the smoke vanishes, doubles in brightness or jumps; no wisp that is brighter than the glow that
lights it; and nothing of it in a scene that put no fire out.

## Deviations

None from an earlier default: a site without `FlameSmoke` and a fire that is never put out draw exactly as before. `SMOKE_TAIL_SECONDS`
(public), `EMISSION_SECONDS`, `AFTERGLOW` and the window and fade constants are design defaults, marked above.

## Pending index

None of the three kinds is missing. Two sources are web pages the corpus does not hold: Oseman (craft) and Saig (technical). A
measurement of how long a snuffed wick or a doused torch smokes was not found: the tail's length is a design default. The two candle
papers in the corpus (`10.1038_s41598-020-78229-x`, `10.1038_s41598-018-36754-w`) are about a lit candle's oscillation and say nothing of
the smoke of an extinguished one.
