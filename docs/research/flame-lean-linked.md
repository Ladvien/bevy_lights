# A linked fire's light leans too

`flame-lean-light.md` moves a pooled light to the middle of its leaning flame, but only when the pooled candidate is on the `FlameSite` entity itself. A torch whose flame and light are siblings, joined by `FlameSite::fire` (`flame-fire-link.md`), keeps its light upright beside a flame bent by the same wind. This note removes that exception.

## Sources

| source | kind | use |
|---|---|---|
| `10.7456_tojdac.1674124` (Doğaner 2025) | craft | A practical's light must come from the visible source: "the light has to come from the visible source". A sibling-linked torch is the common authored shape and was the one left wrong. |
| `s2011-advances-pdf-slides` (the fire talk) | production | The light belongs to the fire's body and is authored with it: "Rendering is working quite hard to try to hide the particle nature of the system", so the light and the drawn fire agree whatever the entity layout. |
| `docs/research/flame-wind.md`, `flame-lean-light.md`, Wen et al. 2019 and DGMR eq. 7-9 | technical | The tilt `flame_tilt(speed, diameter)` and `lean_light_shift`; unchanged. |

## Craft principle

A gaffer rigs the light on the fire, not on the prop that happens to hold the fire. Whether the host parented the light under the torch or beside it is bookkeeping; the pool of light still slides downwind with the flame.

## How shipping games do it

The fire talk authors the light as one part of the fire effect. The adopted shape follows: the light's lean is a function of the site that *draws* the flame (its `height`, its diameter), found through the link the host already declared.

## Model adopted

No new maths. `lean_light_shift` is applied for a pooled winner when the winner is a `FlameSite` entity (as before) or is named by a `FlameSite::fire` (new). Precedence: the entity's own site first, else the first site that links it. The diameter is the candidate's own, else that site's, else `DEFAULT_FIRE_DIAMETER`, exactly as the flame resolves it. Looking up the linking site scans the sites once per frame, only while a non-zero `FlameWind` exists, and only to find the winners' sites.

## Parameters

None new.

## What great looks like

A torch with a sibling light in a crosswind throws its pool downwind with the flame, identical to a torch whose light is on the site entity.

## Validation targets

- A candidate linked from a `FlameSite::fire` gets the same shift a candidate on the site entity gets.
- Still air and a candidate no site links are bit-identical to before.

## Deviations

Design default: if two sites link the same fire, the first in query order wins (the sites' heights are the host's to keep equal). The light leans by the full tilt although the flame fades its lean toward overhead (`flame-lean-light.md`).

## Pending index

No new documents.
