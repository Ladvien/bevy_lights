# The light pool

A fixed number of real point lights (`LightPool::size`, default 16) is handed every frame to the
best-ranked `PoolCandidate`s around the entity carrying your focus marker (exactly one, unless the pool is shared, below), each light at the
candidate's position plus its own world-space offset. Everything else is a flame and nothing else. The
assignment is deterministic: rank, then `Entity`.

`PoolBudget` is the pool's lamp schedule, rewritten every frame: `holds(entity)` and `holder_of(slot)` say which candidates are real lights right now, `casts_shadow(entity)` (and `shadowed`) which of those lights throw a shadow, and `starved()` counts the candidates that wanted one and are only flames, so an editor can show the budget and a game can see `size` is too small.

**More than one viewer.** The pool measures from the entity carrying your focus marker, and by default needs exactly one: two are reported and leave the pool dark. A game with a split screen, or a map camera beside the player's, sets `LightPool::focus` to `FocusRule::Shared` and puts the marker on each viewer. The pool's `size` is then **shared**, not multiplied: each focus is entitled to `ceil(size / foci)` lights, ranked by `rank` among the candidates in `reach` of it, and what one viewer does not use goes to the others, so a viewer among twenty fires cannot take the lights of one standing by three (on a plain nearest-focus distance it would take all of them). A fire between two viewers is one light. A viewer joining or leaving re-divides the lights through the ordinary hand-off (give the pool `reserve` at least as large as the number of lights that move, and its total brightness holds at every frame). With one focus it is the default rule, to the bit; with two, the pool's own work on 5 000 candidates went from 490 to 619 µs (best of eight runs; four foci, 660 µs). Run `LIGHTS_SPLIT=1` on the courtyard example for two cameras on one window, and `LIGHTS_FOCUS_RULE=one` over them for the pool without the rule.

A candidate is ranked by the **light it puts on the subject** (`intensity / distance²`), not by its
distance alone, so a bright brazier at 3 m outranks a dim torch at 2 m; `LightPool::rank` switches to
`Rank::Distance` for the old rule. A slot keeps the candidate it holds until a challenger is
`hysteresis` times better (default 1.15, the overlap rule, measured in the same quantity the rank
uses), and only a candidate **no other slot holds** is ever a challenger, so a newcomer displaces the
pool's *worst* light rather than reshuffling all of them. A slot that does change owner fades back up
over `fade_seconds` (0.35 s) rather than popping, and `reserve` (default 1) both keeps a spare light so
the owner a hand-off leaves behind fades *out* at the same time — the subject's light is continuous
through the change rather than dipping, which is the craft's "overlapping key lights" — and caps how
many lights change owner in one frame, so a hand-off is never made without a cross-fade to hide it.

`PoolCandidate::phase`, `diameter` and `depth` let a slot adopt its winner's own fire clock, fuel-bed
rate and depth, so a light handed to a torch breathes as that torch's flame does. Each is optional:
`phase: None` leaves the slot's own `1.3 · slot` clock alone, `diameter: None` gives the pool's un-sized
rate, and `depth: None` restores `FLICKER_DEPTH` — so a host that models no per-prop fire keeps the
pool's uniform one.

**Shadows are budgeted apart from lights.** The template's `shadow_maps_enabled` turns the pool's
shadows on, and `LightPool::max_shadowed` caps them: `Some(3)` gives a shadow to the three best-ranked
lights the pool holds (the closest and brightest, held with the same `hysteresis`) and lets the rest
light without one; `None`, the default, shadows every light, as before. Every shadowed point light
renders six shadow views a frame — about 0.6 ms a light on an RTX 3090 in the courtyard — so sixteen
lights can afford three shadows where they could not afford sixteen. A light that earns or loses its
shadow because the ranking moved is cross-faded to itself through a reserve light, so the shadow
deepens over `fade_seconds` while the lit ground around it holds still; give a pool with shadows
`reserve: 2`, since hand-offs and shadow changes share the reserves. A change to the cap or the flag
applies at once, as a graphics setting should: shadows turned off, by the flag or a cap of `Some(0)`, are
off on that frame for the reserve lights fading a light out too.

**One template, every light, live.** Every field of `LightPool::light` but `intensity` and
`shadow_maps_enabled` is every pool light's: colour, range, radius, the shadow's biases and near plane,
`contact_shadows_enabled`, and Bevy's `soft_shadows_enabled` where its `experimental_pbr_pcss` feature
gives the light one. A change to the template reaches every light, reserves included, on the next frame,
so an editor's slider moves the whole pool and a light spawned later matches the ones already burning.
Contact and soft shadows are off by default: Bevy's contact shadows need a `ContactShadows` camera (and
TAA, or they dot a lit wall), and its point-light soft shadows only soften at a `radius` of metres, which
also widens every highlight.
