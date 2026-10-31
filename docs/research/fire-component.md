# A whole fire as one component (`Fire`)

A fire in this crate is up to six components on one entity (`FlameSite`, `PoolCandidate`, and `FlameEmbers`, `FlameSmoke`, `FlameTongues`, `FuelBed` for the kinds that have them), and the numbers they share (diameter, depth, height, the light's offset, the phase, the colour) have to agree. `FireKind` gave the numbers names (`fire-kinds.md`); it could not give the *entity*: `FireKind::fire` returns a `(FlameSite, PoolCandidate)` pair and says in its doc that the sparks, the smoke, the tongues and the bed "stay separate because a bundle cannot hold an absent component", so a host spawns a campfire as a bundle and then five `if let Some(x) = kind.x() { entity.insert(x) }`. The README's quick start is 20 lines of loose numbers for two fires. `bevy_emerge` (family issue F7, `Ladvien/bevy_emerge#103`) places fires as "placements with a family component" and stores **one** RON value of the crate's own serde type per placement; today that value is a `Flicker`, so a placed fire is a light with no flame. This note is the one component that is a whole fire.

## Sources

| source | what it is | use |
|---|---|---|
| `10.13140_marples.nav.lighting.thesis` — Marples 2017, PhD thesis (corpus; quoted in `carried-fire.md`, `fire-kinds.md`) | craft | a flame is read as one thing: the motion, the light and the smoke of it together are the cue |
| GPU Gems 1, ch. 6.2 — Nguyen, *Fire in the "Vulcan" Demo* (corpus; quoted in `flame-tongues.md`) | production | identical fires must differ by a per-instance start (phase), or the repetition is the first thing seen |
| `bevy_ecs-0.20.0/src/component/mod.rs`, "Required Components" (vendored source) | technical | the engine's own precedent for sugar that expands a marker into components: what the entity already carries wins |
| `10.1038_s41598-018-36754-w` — Chen, Guo, Jia & Xiao 2019 (corpus; quoted in `fire-kinds.md`) | technical | the flicker rate follows the bed's diameter, so a kind's diameter is the one number the light and the flame must share |
| `fire-kinds.md`, `fire-colour.md`, `lighting-look.md` (this repo) | the crate's own | the kind table, the luminance-matched ramp, the pool's lux figures this component's defaults come from |

## Craft principle

Marples, on what draws a player in a lit space: "one of the more commonly used examples of this is the motion of flame, this can be in the form of lanterns, flaming barrels, or burning objects. For instance, Left 4 Dead (Valve Corporation, 2008) uses both the motion and simultaneous illumination offered by flames". And in his own level study: "The exits all had torch sconces running down the outermost wall, in some cases, these glowed, and in others, an animated flame and smoke effect was added thus providing a motion cue." The viewer does not read a flame, a light and a smoke; they read **a fire**, and a fire with a part missing (a flame that lights nothing, a light with no flame, smoke from a fire that is out) is the first thing a cinematographer sees. So the parts that make one fire should be authored, saved and loaded as one object, and a fire that lacks one should lack it on purpose.

## How shipping games do it

Nguyen, on the fire of the Vulcan demo: "We started with a single animation, and it worked even better with two or more. In addition, we created particles with a random percentage of difference for a number of attributes: size, position, rate of decay, animation start frame, global transparency, and so on." A shipped fire is an authored archetype (a prefab, a blueprint, an effect asset: flame, light, sparks, smoke together) that a level designer drops in by name, and the archetype's own variation is **per instance** (a start frame), so a row of twenty identical torches does not breathe in unison. That is the shape adopted: a named kind, one component, and a per-entity phase the host never has to think of.

The engine's precedent for the expansion: "Components can specify Required Components. If some Component A requires Component B, then when A is inserted, B will also be initialized and inserted (if it was not manually specified)." A marker that expands into its parts, **where what the host wrote beside it wins**, is idiomatic Bevy. `Fire` cannot be a `#[require]` (the parts depend on the kind's value), so a system does it, with the same precedence.

## Model adopted

```
Fire { kind, lumens: Option<f32>, phase: Option<f32>, color: Option<Color>,
       embers, smoke, tongues, bed: bool }          (Component, Reflect, serde, Default)
```

`FirePlugin` runs one system, `expand_fires`, ahead of `LightsSystems::Pool`, over every `Fire` that is new or changed:

| part | from |
|---|---|
| `FlameSite` | `kind.site()`, `phase`, `color` → the flame's middle (`flame_mid`: the same hue at the shipped ramp's luminance), `fire: Some(this entity)` so the flame follows this entity's `Kindle` and `Snuff` as a pooled torch's does |
| `PoolCandidate` | `kind.candidate_at_mean(lumens)`, `phase`, and the light's colour: `color`, or a 1900 K blackbody (`FIRE_KELVIN`) when there is none |
| `FlameEmbers`, `FlameSmoke`, `FlameTongues`, `FuelBed` | `kind.embers()` etc., each only if its flag is set and the kind has one |

- **Precedence.** A `Fire` that has just **arrived** writes each part with `insert_if_new`: `(Fire::lit(Brazier, 1000.0, 0.9), FlameSmoke { density: 0.2 })` keeps the host's smoke, as a required component's explicit value does. A `Fire` that is **changed** afterwards (replaced, or mutated through `get_mut`, reflection or BRP) rewrites every part, and removes an optional part the new fields no longer name, so a panel that flips `kind` from `Campfire` to `Lantern` leaves no tongues behind. What a host edits on a part afterwards stands until the `Fire` next changes.
- **Phase** is `fire_phase(entity)`: a SplitMix64 of the entity's bits mapped to `[0, TAU)` (engine-free, `model::fire::fire_phase`), so identical fires desynchronise by default (Nguyen's "animation start frame") and a seeded run is reproducible. `phase: Some(p)` is the host's.
- **Lumens** `None` is `kind.lumens(FIRE_EDGE_LUX, h/2)`: 250 lx at the edge of the kind's reach with the light at the flame's middle and the foot on the ground, the courtyard's own figure for a carried fire (`lighting-look.md`: the edge of a pool is two stops under its centre), in the crate's reference exposure (EV100 6). It is a starting point a host replaces with `Some(lumens)` or `Fire::lit(kind, lux, over)`; nothing converts units.
- **Colour** `Some(c)` is the light's colour, and the flame's middle is `flame_mid(c)`: `c` scaled so its luminance is the shipped middle's (`DEFAULT_MID`), the luminance match iteration 280 made by hand (the courtyard's blue needed 1.8x, its green 1.0x). The orange family is the identity. `None` is **a wood fire**: the flame is the scene's (`FlameLook::color`), and the light is `blackbody_colour(FIRE_KELVIN, 1.0)`, 1900 K (`lighting-look.md`: "fire colour temperature 1900 K, design default in the wood-flame band (1800-2200 K)"). A `PoolCandidate` with no colour takes the pool template's, and the template's default is white, so the first `Fire` the examples drew was orange flames over a white pool (found by looking, the first capture of `examples/fires.rs`).
- **Removal.** Removing `Fire` leaves the parts in place: the entity is the host's, and the parts are ordinary components it can keep, edit or remove.

## Parameters

| field | default | provenance |
|---|---|---|
| `kind` | `Torch` | the kind most often carried; a picker overrides it |
| `lumens` | `None` (250 lx at the kind's reach) | design default (`lighting-look.md`, the courtyard's carried-fire figure) |
| `phase` | `None` (from the entity) | Nguyen: a random start frame per instance |
| `color` | `None` (a wood fire: the scene's flame, a 1900 K light) | `fire-colour.md`, `lighting-look.md` |
| `embers`, `smoke`, `tongues`, `bed` | `true` | the kind's own (`fire-kinds.md`); `false` leaves the kind's part out |

`FireKind::fire` is removed (a `(FlameSite, PoolCandidate)` pair is the same two parts `Fire` writes, without the other four); `FireKind::candidate_at_mean` is added, the one place a candidate is built from a kind and a mean.

## Validation targets

1. A `Fire` of each of the six kinds, spawned bare, becomes exactly the parts its kind names (`FlameSite`, `PoolCandidate`, and the optional four per `FireKind::embers/smoke/tongues/bed`), with the kind's height, diameter, depth and light offset, in the first `update`.
2. A host component spawned beside the `Fire` is not overwritten; the same component edited after a `Fire` change is rewritten, and a part the new fields do not name is removed.
3. Twenty fires spawned alike do not share a phase; `fire_phase` is in `[0, TAU)`, deterministic, and its low bits are not the entity index's.
4. With a pool, a `Fire` is handed a real light whose colour and mean are the fire's, and a `Snuff` on the entity puts out the flame and the light together.
5. The same fire in the example (`examples/campfire.rs`, migrated to `Fire`) is **zero pixels different** from the build before, with the phases pinned; a fire with no colour is the default fire.
6. `Fire` round-trips through RON and JSON with the `serde` feature, and a partial value (`(kind: Campfire)`) reads with every other field defaulted.
7. `problems()` names a non-finite or non-positive `lumens`, a non-finite `phase` and a negative colour channel, and a bad `Fire` warns once on arrival like the other components.
8. Cost: one `Changed` filter over `Fire` per frame (nothing to do on a frame with no new fire), measured in `bench_flames`.

## What great looks like

A level author, or an editor's placement panel, writes `Fire::lit(FireKind::Campfire, 250.0, 0.4)` (or the one RON line `(kind: Campfire)`) on an entity and gets a campfire: a mound of tongues on a bed of coals, sparks, smoke, a light that is the flame's own colour and breathes with it, each of twenty such fires on its own clock, put out and lit again by `Snuff` and `Kindle` on that one entity. There is nothing to wire and nothing to forget: no fire without its light, no light without its flame.

## Deviations

- `FireKind::fire` is removed (clean cutover; a minor version).
- `FIRE_EDGE_LUX` (250) is a design default, from the courtyard's carried-fire figure, not a measured one.
- `flame_mid` reuses the luminance match of `fire-colour.md` as a function of the colour; it replaces a hand-set gain per hue in a host's palette, it does not change `retint`.
- A `Fire` with no colour sets its candidate's colour (1900 K) instead of leaving the template's: a host that styles pool lights with its own palette (`PoolLight`) and uses `Fire` sets `Fire::color`, or recolours the lights as before (a palette recolours every light that carries its marker, the template's colour included). `FIRE_KELVIN` (1900) is the courtyard's `FIRE_KELVIN` moved into the crate.

## Pending index

None. (Every source is already in the corpus; `fire-component.md` is a new note, indexed with the repository's notes.)
