# A bad setting is reported where it enters, not where it shows

`problems()` (iteration 191) names an unusable field, but only to a host that asks. A host that reads fires from a scene file, a BRP call or a panel and never asks gets a dark light or a flame that renders as nothing, with no line in the log. This note adds the one thing `problems()` lacked: the crate calls it itself, once, when a component arrives, and says so with a `warn!`. It sets no lighting number.

Scope: components `Flicker`, `Snuff`, `FlameSite`, `FlameEmbers`, `PoolCandidate`, checked on `Add`. Not in scope: resources (`LightPool`, `FlameLook`, `FlameWind`, `FlameGust` are edited live and have no single entry point), refusing or clamping a value (the crate "never refuses a value", `src/model/problem.rs`), any change to behaviour.

## Sources

| source | kind | use |
|---|---|---|
| `AGENTS.md`, "non-negotiables": "Misconfiguration gets a loud `error!`/`warn_once!`, never a panic" | this crate's rule | warn, do not panic, do not refuse |
| `docs/research/flame.md`, `docs/loop-log.md` iteration 194 and AGENTS "A WESL error is silent at build time" | this crate's experience | silent failure that renders as nothing is the costly kind |
| `10.7456_tojdac.1674124`, Doğaner (craft, quoted in `flame-catch-size.md`) | craft | "these objects determine the motivation of the light in the scene": a fire that silently drops out of the frame breaks the motivation |
| Bevy 0.20 `examples/ecs/observers.rs` (`On<Add<Mine>>`) | technical | the lifecycle hook used |

No rendering-literature source applies: nothing here is a lighting model. Recorded as a gap, not filled.

## Craft principle

The scene is read through its practical lights; a fire whose `phase` is `NaN` writes a `NaN` intensity and the light vanishes, which the audience reads as an unmotivated dark corner. The gaffer's equivalent is a lamp check before the shot: the fault must be found on the bench, not in the rushes.

## How shipping games do it

Engines warn at the point an authored asset is loaded, not each frame (Bevy itself: B0004 on a missing visibility component, at the add). Once per kind, with the offending entity, is the convention that keeps a thousand identical torches from filling a log.

## Model adopted

An observer `On<Add, C>` per component type reads the new value, calls its `problems()`, and when non-empty emits one `warn!` naming the component, the entity and each problem (`Problem`'s `Display`). Each observer warns for its **first** bad component only (a `Local` flag): a scene of a thousand identical bad torches is one line, and a second, different bad kind still warns. Registered by the plugin that owns the component (`FlickerPlugin`: `Flicker`, `Snuff`; `FlamePlugin`: `FlameSite`, `FlameEmbers`; `LightPoolPlugin`: `PoolCandidate`). Cost: nothing per frame; one `problems()` call per add.

## Parameters

None. No default changes.

## Validation targets

- an app that spawns a `Flicker` with a `NaN` phase logs exactly one warning naming `phase`, however many bad ones follow;
- a sane fire logs nothing;
- behaviour (intensity written) is unchanged.

## Deviations

None. Values are still never refused or clamped. A value mutated after the add is not re-checked (a host that edits live calls `problems()` itself).

## What great looks like

Not visual. A host's first bad `Flicker` produces one line, `bevy_lights: Flicker on 12v1 has unusable settings: `phase` must be a finite number`.

## Pending index

No new document was indexed.
