# A bad resource is reported when it is written, not where it shows

Iteration 225 made the five settings *components* report a bad value as they arrive (`docs/research/settings-report.md`) and left the four settings *resources* out: `LightPool`, `FlameLook`, `FlameWind`, `FlameGust`. Their reason was "no single entry point". Bevy has one: a resource's change tick. This note adds the same single line for them. It sets no lighting number and refuses or clamps nothing.

These four are exactly what an editor panel edits live (bevy_grimoire configures this crate "from its own panels"), so a field typed as `NaN` or a negative `reach` is the likeliest bad value to reach the crate, and it shows as a dark pool or a flame leaning nowhere with no line in the log.

## Sources

| source | kind | use |
|---|---|---|
| `AGENTS.md`, non-negotiables: "Misconfiguration gets a loud `error!`/`warn_once!`, never a panic" | this crate's rule | warn, never panic, never refuse |
| `docs/research/settings-report.md` (iteration 225) | this crate | same message shape, same "one line, not one per frame" rule |
| `10.7456_tojdac.1674124`, Doğaner (craft) | craft | "these objects determine the motivation of the light in the scene": a pool that silently goes dark breaks the motivation |
| Bevy 0.20 `bevy_ecs` `Res::is_changed` | technical | the entry point: true on the frame the resource was inserted or any `DerefMut` happened |

No rendering-literature source applies (nothing here is a lighting model); recorded as a gap.

## Craft principle

The gaffer checks the lamp when it is rigged and again when someone touches the dimmer, not every frame of the take. A panel that drags a slider writes the resource every frame, so the report must fire when the *problem changes*, not when the resource does.

## How shipping games do it

Engine editors validate a property on edit and show the message beside the field, once, and clear it when the value is valid again. A log line that repeats at frame rate is the failure mode to avoid.

## Model adopted

One system per resource, `Option<Res<R>>` (a missing resource is not an error: `FlameGust` and `FlameWind` are optional), run only when `is_changed()`. It computes `problems()` and compares it with the list it last warned about (`Local<Vec<Problem>>`): equal or empty-and-already-clear says nothing; a new non-empty list warns once; an empty list resets the memory, so a fix followed by a new mistake warns again. Cost: a change-tick test per frame per resource; `problems()` runs only on a frame the resource changed. Registered by the plugin that owns the resource (`LightPoolPlugin`: `LightPool`; `FlamePlugin`: `FlameLook`, `FlameWind`, `FlameGust`).

## Parameters

None. No default changes.

## Validation targets

- a `LightPool` written with `reach = NaN` logs one warning naming `reach`, however many frames it stays bad;
- fixing it and then breaking another field warns again;
- an untouched sane resource logs nothing, and an absent `FlameGust` logs nothing;
- behaviour is unchanged.

## Deviations

None. Values are still never refused or clamped.

## What great looks like

Not visual. A panel dragging `reach` through a negative value gives one line, `bevy_lights: LightPool has unusable settings: `reach` must be greater than zero`.

## Pending index

No new document was indexed.
