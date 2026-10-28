# A flame names the light it is the fire of, and reads its breath from it

Issues #4 (criterion 3) and #3, and Sanctuary's two measured instances of the same fault: a `FlameSite` carries its own `diameter`, `depth` and `phase`, the `Flicker` (or `PoolCandidate`) on the light it motivates carries another set, and nothing relates them. Sanctuary shipped the disagreement twice (a wall sconce's flame at the torch's 0.30 over a light at 0.15; every carried torch's flame at 0.30 over a pool light that adopted the 0.15 default). The crate's own docs say the flame and its light "cannot disagree"; the API lets them.

Scope: one optional link, `FlameSite::fire: Option<Entity>`, and the flame's brightness read through it. The `source` read of iteration 113 names the *prop*; this names the *light*. Ask 3 of #2 (`visible_from`) and #3's origin-from-light are not part of it.

## Sources

| source | what it is | use |
|---|---|---|
| `10.7456_tojdac.1674124` | Doğaner, *The narrational role of lighting design in The Lights of the Dark City* (2025) | craft: the visible source is what motivates the light; if the two do not move together the audience can check it by eye |
| `s2011-advances-pdf-slides` | SIGGRAPH 2011 *Advances in Real-Time Rendering*, the Fire / Flamethrowers / Thrusters talk | production: the drawn emission and the light come from one fluctuating source, not two tuned separately |
| `10.1017_jfm.2018.707` | Xia & Zhang 2018, J. Fluid Mech. | technical: the flame's body and its emission are one oscillator, one frequency |
| `docs/research/flame-flicker.md`, `docs/research/flicker-depth.md` | this crate's own measurements | the flame's brightness is already `flicker_swing(diameter, depth, t, phase)`, the light's function; the only open question is where the arguments come from |

## Craft principle

Doğaner, on Bordwell and Thompson, quoted in `flame-flicker.md`: "these objects determine the motivation of the light in the scene." A flame that dips by 30 % over a light that dips by 15 % is a source that does not motivate its own light, and it is the one comparison a viewer can make without a meter. The craft rule is *one fire*; the API should make two fires on one prop impossible to write, not merely documented.

## How shipping games do it

`flame-flicker.md` records the 2011 *Advances* fire talk: the emission is "splatted from the same simulation that moves it", i.e. the visible fire and the light are driven from one state and never from two copies of its parameters. A copy is what Sanctuary kept making, in four places per torch, and missed the fourth.

## Model adopted

`FlameSite::fire: Option<Entity>`, the entity whose fire this flame is.

1. `Some(light)` and `light` has a `Flicker`: the flame's brightness is `flicker_swing` of that `Flicker`'s `diameter`, `depth` and `phase`, read every frame, so a `set_depth` on the light moves the flame with it on the same frame.
2. `Some(prop)` and `prop` has a `PoolCandidate` and no `Flicker`: `diameter`, `depth.unwrap_or(FLICKER_DEPTH)` and `phase.unwrap_or(site.phase)`, the values the pool hands to the slot it gives that prop.
3. `None`, a despawned entity, or an entity with neither: the site's own `diameter`, `depth` and `phase`, exactly as today. A flame whose host does not set `fire` is unchanged, same bytes.
4. The link is read, never written: the host owns both entities' lifetimes. The site's own `depth`/`diameter` remain the fallback and the documented value for a fire with no light.
5. Only brightness follows. The clock phase that moves the silhouette's noise (packed in the tag at dress time) stays the site's own: it is shape, not breath, and a light's `phase` changing every pool hand-off would restart the flame's motion.

## Parameters

None added. The fallback depth is `FLICKER_DEPTH`, the pool's own rule for a candidate with none.

## Validation targets

- A site with `fire: None` writes the same tag byte as before at every frame.
- A site whose own `depth` is 0.15 and whose `fire` light has `depth` 0.30 draws the 0.30 swing, and a `set_depth` on the light is followed within the frame.
- A candidate with `depth: None` gives `FLICKER_DEPTH`; with `Some(0.3)` gives 0.3.
- A despawned or unrelated `fire` falls back to the site's own values and does not panic.

## Deviations

- **An `Option<Entity>` on the site, not a `Flame` read of the parent.** Sanctuary's topology has the site and the light as siblings under a torch, so the parent is not the light; an explicit link is the only form that serves it (issue #4's own conclusion).
- **Brightness only, not `kindle`.** The light's `Kindle` is a component the host adds and removes; reading it through the link would make the flame's growth depend on a host re-lighting, a separate decision.
- **Rendering is unchanged** (no shader or look change), so no capture; the validation is the tag byte.

## Pending index

Distill was not queried; the sources are those already in `flame-flicker.md`. No note was indexed.

## What great looks like

A host writes `FlameSite { fire: Some(light), .. }`, sets a depth once on the light, and cannot make the flame breathe differently from the ground it lights.
