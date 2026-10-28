# A re-lit fire's flame catches with its light

Iteration 130 let a flame read its breath from the light it is the fire of (`FlameSite::fire`) and deferred the catch: "the light's `Kindle` is a component the host adds and removes; reading it through the link would make the flame's growth depend on a host re-lighting, a separate decision." This is that decision. A host re-lights a fire by inserting `Kindle::default()` over the light, and the light flares from nothing while the flame, which caught once at spawn, burns on at full height: a source that precedes its own light.

Scope: through `FlameSite::fire`, a flame's drawn size follows the linked entity's `Kindle`. Not in scope: a `PoolCandidate`'s catch (the pool's light is a different entity, and its `Kindle` is the pool's), `visible_from`, origin-from-light.

## Sources

| source | what it is | use |
|---|---|---|
| `10.7456_tojdac.1674124` | Doğaner, *The narrational role of lighting design in The Lights of the Dark City* | craft: the visible source motivates the light |
| `s2011-advances-pdf-slides` | SIGGRAPH 2011 *Advances in Real-Time Rendering*, Fire / Flamethrowers / Thrusters | production: the visible fire and its light are one source |
| `10.1017_jfm.2018.707` | Xia & Zhang 2018, J. Fluid Mech. | technical: a small burning area gives a short flame; a young fire is short |
| `docs/research/flame-kindle.md`, `docs/research/flame-fire-link.md` | this crate | the catch envelope (`kindle_gain`) and the link |

## Craft principle

Doğaner, quoted in `flame-kindle.md`: "these objects determine the motivation of the light in the scene." A flame that is full height while its light is at zero precedes the light it motivates, the one direction of the relationship the audience can check by eye because both ends are on screen. A fire put out and lit again must be born small again, in step with its light.

## How shipping games do it

The 2011 *Advances* fire talk, quoted in `flame-kindle.md`: "Ramp the alpha to fade in/out over particle lifetime" and "shrink particles as they fade": size is a lifetime channel driven by the fire's state, not a constant set at spawn. The drawn fire and its light read one state; here that state is the light's `Kindle`.

## Model adopted

1. `FlameSite::fire` names an entity that has a `Kindle`: the flame's scale is `kindle_gain(kindle.elapsed)`, the same function the light's intensity uses, floored at 0.001, so flame and light catch on the same frames. The site's own `kindle` seconds do not apply while the link owns the catch (the light's envelope is `KINDLE_SECONDS`, fixed).
2. Once `kindle.elapsed >= KINDLE_SECONDS` the scale is exactly 1.
3. No `fire`, a despawned entity, or an entity with no `Kindle`: the site's own `kindle` catch, exactly as before. A flame whose scale is left off 1 by a link whose `Kindle` is gone returns to 1 on the next frame.
4. A fire that kindles at spawn through its own `FlameSite::kindle` and has a linked `Kindle` already running follows the light, not its own clock.

## Parameters

None added.

## Validation targets

- Re-inserting `Kindle::default()` on the linked light collapses the flame's scale to the floor and it is back to 1 once the kindle ends, matching `kindle_gain` frame by frame.
- A site with `fire: None`, or whose `fire` has no `Kindle`, is unchanged: default `courtyard` capture is pixel-identical.

## Deviations

- The site's `kindle` seconds are ignored while a `Kindle` is linked: design default, so flame and light cannot disagree about the length of the catch (the cause of the defect `flame-fire-link.md` fixes for depth).

## What great looks like

Relight a brazier: the flame shrinks to a point as the light drops out, and both flare back together, no frame where the flame is full and the ground is dark.

## Pending index

Distill was not queried; sources are those already quoted in `flame-kindle.md` and `flame-fire-link.md`. Nothing indexed.
