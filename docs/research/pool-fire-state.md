# A pooled fire's life: Kindle and Snuff on a pool candidate

`Kindle` (iteration `flame-kindle.md`) and `Snuff` (`flame-snuff.md`) act on a `Flicker` light the host owns. A fire that
lives in the **pool** — a torch in a bearer's hand, one of thousands of virtual braziers — has no light of its own: the
pool lends it one. Today a host that douses such a torch has only two moves, both wrong: remove `PoolCandidate` (the
slot hands off at once, so the light cross-fades over `fade_seconds` while the flame drawn on it, with nothing to link to,
stays at full size) or despawn it. The flame and the light disagree for as long as the fade runs, which is the one
disagreement `flame-snuff.md` says the viewer can check.

## Sources

| source | what it is | use |
|---|---|---|
| `10.7456_tojdac.1674124` — Doğaner, *The narrational role of lighting design in The Lights of the Dark City* (2025) | craft: a visible practical motivates the light | the drawn flame and the light it motivates must go out as one |
| `s2011-advances-pdf-slides` — SIGGRAPH 2011 *Advances in Real-Time Rendering in Games*, Fire/Flamethrowers/Thrusters | shipping production fire | a fire's end is a cross-fade of size and emission, not a cut |
| `10.1038_s41598-020-78229-x` — Gergely et al. (2022) | technical: candle flame state | a starved flame quiets before it goes out; the amplitude of the swing is not the thing that changes |
| `docs/research/pool-hand-off.md` (Brown, *Cinematography* overlap quote) | craft: constant intensity across a move | a fire's change of state must not be confused with a hand-off |

## Craft principle

> "A table lamp or street lantern on a set is not usually the main source of light used by the technical crew, as these
> sources may be inadequate or uncontrolled. However, these objects determine the motivation of the light in the scene."
> (`10.7456_tojdac.1674124`)

The practical motivates the light, so the two change together. Which real light happens to be lent to the practical is
an implementation detail the audience never sees; a torch that is going out must dim whether or not the pool lent it a
light this frame.

## How shipping games do it

> "Crossfade to a smokey particle over the lifetime." (`s2011-advances-pdf-slides`)

The fire's end is a fade on the fire's own lifetime. A pooled light is the *rendering* of that fire, so the state belongs
to the fire (the prop), not to the slot: a slot that changes owner mid-snuff must still show the owner's fade.

## Model adopted

`Kindle` and `Snuff` are legal on an entity carrying `PoolCandidate` (and no `Flicker`). The crate advances them on
virtual time, writes `Kindled` / `Snuffed` for the candidate's entity, and the pool multiplies the candidate's
`intensity` by `kindle_gain(elapsed) · snuff_gain(elapsed)` — the product `flicker_lights` writes for a light of its
own. The candidate's `FlameSite::fire` names it and the flame follows through the existing link, so no flame code
changes. A candidate whose `Snuff` has finished (`elapsed >= SNUFF_SECONDS`) **no longer wants a light**: it leaves the
pool's list, its slot is freed for the next candidate and the cross-fade of its departing light starts from 0, so the
slot is never held by a dead fire.

The **rank** uses the candidate's un-scaled `intensity`: a fire mid-snuff keeps its slot rather than being handed off
while it dims (a hand-off layered on a fade is two changes at once), and a fire mid-kindle is judged by what it will
be. Re-lighting is the same as for a light: remove `Snuff`, insert `Kindle::default()`.

## What great looks like

A bearer's torch doused at the edge of the pool: the light on the stone and the flame in its hand fall together over a
second, no slot hand-off dip, the freed slot taken by the next-brightest torch with its own fade-in.

## Parameters

None new. `KINDLE_SECONDS` and `SNUFF_SECONDS` are those of `flame-kindle.md` and `flame-snuff.md`; the product rule
matches `flicker_lights`.

## Validation targets

- A pooled candidate with `Snuff` at `elapsed = t` lends a light of `intensity · snuff_gain(t)`; at the end exactly 0
  and `Snuffed { candidate }` is written once.
- A finished snuff frees its slot to the next candidate; `PoolBudget::wanting` drops by one.
- A pooled candidate with `Kindle` lends `intensity · kindle_gain(t)` and `Kindled` is written once.
- A host that never uses either pays nothing: no new column on the candidate scan (the advance system matches only
  entities that carry `Kindle` or `Snuff`).

## Deviations

None to a default.

## Pending index

None.
