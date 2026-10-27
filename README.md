# bevy_lights

Practical lights for Bevy 0.20: fires that flicker and flare as they kindle, a budgeted pool of real
lights placed over thousands of virtual ones, and billboard shader flames.

[![CI](https://github.com/Ladvien/bevy_lights/actions/workflows/ci.yml/badge.svg)](https://github.com/Ladvien/bevy_lights/actions/workflows/ci.yml)

> ⚠️ **Vibe Coded**
>
> This crate was written with heavy LLM assistance, under human direction and review. Every model is
> researched and every default is cited or marked a design default; the code is tested, gated in CI
> and reviewed against captures. It is still code an agent wrote, and you should treat it as such.

![Fires that catch, flicker at their own rate, and a pool of real lights](docs/media/showcase.gif)

*Two seconds of `examples/courtyard`: the braziers catch (0.6 s, light and flame together), then the
fires breathe at the rate the puffing law gives their fuel bed — 2.6 Hz for a 0.34 m coal bed — and the
pool hands its real lights between torches without a pop. Longer run: [`courtyard.gif`](docs/media/courtyard.gif).*

## What you get

- **`Flicker` + `Kindle`** (`FlickerPlugin`). Every `PointLight` and `SpotLight` carrying `Flicker` is
  modulated around its base intensity by a summed sine (0.70–1.00 of base, mean 0.85 at the default
  `depth` of 0.15; `Flicker::depth` is how far that fire dips, so a lantern can ask for 0.03 and a torch for
  0.3, and its peak stays the base at any depth), and a light
  with `Kindle` flares up over 0.6 s instead of switching on. Give a fire its fuel-bed size
  (`Flicker { diameter: Some(0.34) }`) and its flicker runs at the measured puffing rate,
  `1.5/√D` Hz — a brazier at 2.6 Hz, a pyre at 0.75 — so a large fire breathes more slowly than a
  small one. `Kindled { entity }` is the message a
  host hears on the frame a fire catches, for the sound of it. An invisible light is not rewritten
  (nothing is extracted for it) but its kindle still counts.
- **The light pool** (`LightPoolPlugin::<YourFocusMarker>`). A fixed number of real point lights
  (`LightPool::size`, default 16) is handed every frame to the best-ranked `PoolCandidate`s around the one
  entity carrying your focus marker, each light at the candidate's position plus its own world-space offset.
  Everything else is a flame and nothing else. The assignment is deterministic: rank, then `Entity`.
  A candidate is ranked by the **light it puts on the subject** (`intensity / distance²`), not by its
  distance alone, so a bright brazier at 3 m outranks a dim torch at 2 m; `LightPool::rank` switches to
  `Rank::Distance` for the old rule. A slot keeps the candidate it holds until a challenger is
  `hysteresis` times better (default 1.15, the overlap rule, measured in the same quantity the rank uses),
  and only a candidate **no other slot holds** is ever a challenger, so a newcomer displaces the pool's
  *worst* light rather than reshuffling all of them. A slot that does change owner fades back up over
  `fade_seconds` (0.35 s) rather than popping, and `reserve` (default 1) both keeps a spare light so the
  owner a hand-off leaves behind fades *out* at the same time — the subject's light is continuous through
  the change rather than dipping, which is the craft's "overlapping key lights" — and caps how many lights
  change owner in one frame, so a hand-off is never made without a cross-fade to hide it.
- **Shader flames** (`FlamePlugin`). An additive billboard fire (`src/flame.wesl`) is drawn on every
  `FlameSite`, out of one material and one draw per flame, with each fire's phase, height and
  brightness packed into its `MeshTag` so two fires never burn in step. One shared shape clock, written
  once a frame, on `Time<Real>` — a paused world's shape still moves. A site with a `kindle` grows its
  flame from nothing on the same curve its light kindles by, so the two arrive together; and
  `FlameSite::diameter` given the same value as the light's `Flicker::diameter` makes the flame **breathe
  at the light's own rate and phase**, so the fire and the ground it lights are one fire rather than a
  still flame over a moving pool; `PoolCandidate::phase` does the same for a light the pool hands over.
- **The lighting look** (`bevy_lights::model::cinematography`). The numbers a cinematographer sets,
  as plain functions: `key_to_fill` and `stops` (with `LOW_KEY` 8:1, `MOON_KEY` 4:1 and `HIGH_KEY`
  2:1), `fill_for`, `fire_lumens` (the lumens a pool of a given size and brightness needs),
  `blackbody_colour` (a light's colour from its temperature), `blackbody_luminance` (that
  temperature's **magnitude** in cd/m², which a glowing *surface* needs and a light's colour does not
  carry) and `white_balanced` (the same as a camera balanced elsewhere records it). Size a fire by the
  pool it lights and colour it by the temperature it burns at, instead of picking lumens and an RGB by
  eye.
- **The fuel bed a fire sits in** (`bevy_lights::model::char`). A bed of coals is a *surface with a
  temperature field on it*, and a constant `emissive` is the one thing a real bed never is: it clips to
  a white slab the moment the camera looks down at it, and hides the flame standing in it. `char_field`
  is the field — a heap of overlapping coal shards, bright lumps with dark ash between them, seeded so
  a capture is reproducible — and `char_radiance` / `coal_radiance` map a sample to a colour through
  char, ember and fuel. `coal_radiance` writes those three stops as **temperatures**, so a bed's
  brightness and its hue come from one model and cannot drift apart at a different exposure. The
  example generates a `Rgba32Float` image from the field at startup and uses it as the bed's
  `emissive_texture`; the note's numbers were chosen by measuring the stops in display DN, because a
  ramp whose middle stop is past the display's white point cannot show the structure underneath it.
- **Engine-free maths** (`bevy_lights::model`). Every curve the crate runs, as plain `f32` functions with
  no `bevy` in them, so a host can reuse them in its own systems: `flicker_factor` and
  `flicker_factor_of` with `flicker_rate`, `flicker_swing` (the sized-or-un-sized choice the light and
  the flame both read), `kindle_gain`, `raised_light` and `flame_light_offset` (where a fire's light
  belongs, so a flame and the real light it motivates cannot drift apart), `pack_flame_tag` and its
  inverse; the pool's
  `hand_off` and `preferred` (the overlap rule, on distances and on any "smaller is
  better" quantity) with its `fade_factor` / `fade_out_factor`; the whole of `model::cinematography`
  above, and the whole of `model::char`. `tests/leaf.rs` keeps them engine-free.

### What a host touches

Everything public is `Reflect`, so a BRP front end can read and drive it. The components a game spawns or
marks: `Flicker`, `Kindle`, `PoolCandidate`, `FlameSite`; the ones the crate spawns and a host reads or
styles: `PoolLight` (recolour it for a palette; name its lights what you like), `Flame`; and the resources
it sets: `LightPool` (size, reach, gain, rank, hysteresis, fade, reserve, the slot light's template) and
`FlameLook` (colour, strength, the set-once `max_height`). `Kindled { entity }` is the message a host hears
when a fire catches.

## Quick start

```rust
use bevy::prelude::*;
use bevy_lights::*;

/// The one thing the pool measures distance from: your camera focus, your player, a squad.
#[derive(Component)]
struct MyFocus;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            FlickerPlugin,
            LightPoolPlugin::<MyFocus>::default(),
            FlamePlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    // A brazier: a real light, sized by the pool it lights (1000 lx at 4 m, from 0.9 m up) and
    // coloured by the temperature it burns at.
    let lumens = fire_lumens(1000.0, 4.0, 0.9);
    let [r, g, b] = blackbody_colour(1900.0, 1.0);
    commands.spawn((
        PointLight { color: LinearRgba::rgb(r, g, b).into(), intensity: lumens, range: 10.0, ..default() },
        // `diameter` sizes the flicker by the fuel bed (`f = 1.5/√D` Hz); `None` uses the un-sized rate.
        Flicker { base: lumens, phase: 0.0, diameter: Some(0.34), ..default() },
        Kindle::default(),
        // `kindle` grows the flame with its light, on the same 0.6 s envelope, and `diameter` matches
        // the `Flicker`'s so the drawn flame breathes at the same rate the light does. `depth` is how
        // far it dips: give the flame the same one as its light, or the two disagree.
        FlameSite { height: 1.1, phase: 0.0, kindle: KINDLE_SECONDS, diameter: Some(0.34), depth: FLICKER_DEPTH },
    ));

    // A torch-bearer: a virtual light, which gets one of the pool's real lights when it is near. A
    // hand-held flame is smaller, so its pool is sized from its edge.
    //
    // One entity holds both the flame and the candidate, so the light it is handed can never drift away
    // from the fire: `offset` is the flame's own centre (`flame_light_offset`), measured from the site's
    // origin, and the intensity is sized for the height the light will actually sit at.
    let torch_height = 0.35;
    commands.spawn((
        // `phase` is this torch's own fire clock: the real light it is handed later breathes with the
        // flame drawn on it, not with the pool slot that happens to own it.
        PoolCandidate {
            offset: Vec3::from(flame_light_offset(torch_height)),
            intensity: fire_lumens(250.0, 2.0, flame_light_offset(torch_height)[1]),
            phase: Some(1.9),
            ..default()
        },
        // The pool's lights flicker at the un-sized rate, so this flame does too.
        FlameSite { height: torch_height, phase: 1.9, kindle: 0.0, diameter: None, depth: FLICKER_DEPTH },
        Transform::from_xyz(4.0, 0.0, 2.0),
    ));

    commands.spawn((MyFocus, Transform::default()));

    // The fill: what the fires read against. The crate does not own the moon — a host adds the light and
    // asks the crate for the illuminance that makes the ratio it wants.
    commands.spawn(night_fill());
}

/// The night's fill: two stops under the fire. The crate does not own the moon, so the host adds it
/// and asks the crate for the illuminance.
fn night_fill() -> DirectionalLight {
    let [r, g, b] = white_balanced(8000.0, 2856.0, 1.0);
    DirectionalLight {
        color: LinearRgba::rgb(r, g, b).into(),
        illuminance: fill_for(1000.0, MOON_KEY),
        ..default()
    }
}
```

A light you want to be able to *re-light* is re-inserted with `Kindle::default()`, which writes
`Kindled` again. A fire you want dimmer or brighter changes its `Flicker::base`, never the light's
`intensity`: the crate writes the intensity every frame.

`examples/courtyard.rs` is the full showcase, `examples/bench_lights.rs` the CPU bench.

## Plays well with

This crate owns **practical lights** and nothing else.

| what | whose |
|---|---|
| Sun, moon, sky, exposure, weather, the lightning flash | `bevy_elements` (this crate never meters exposure and never adds a `DirectionalLight`) |
| A gameplay illuminance grid (how lit a tile is, for AI or stealth) | `bevy_light_grid` |
| Trees, forest-fire spread | `bevy_flora` |
| Who lights a lamp, and when | `bevy_society` or the game |
| Flicker, kindle, the pool of real lights, the shader flames, the placement helpers | **this crate** |

A pool light is a light in mid-air with no source of its own: a host styles it (recolour it for the
palette) and usually tags it so its flame is not drawn there — the flame belongs on the torch.

## Built on

| crate | why |
|---|---|
| [`bevy`](https://github.com/bevyengine/bevy) 0.20 | the engine: `PointLight`, `SpotLight`, `Material`, `MeshTag` |
| [`serde`](https://github.com/serde-rs/serde) | optional derives on every public component and resource (`serde` feature) |

## The craft and the science

Every default is researched and cited, or marked a design default. The notes:

| note | what it settles | key sources |
|---|---|---|
| [`docs/research/fire-light.md`](docs/research/fire-light.md) | the flicker's rate and amplitude, the kindle's flare | Sanctuary decisions 0127/0148; Team Fortress 2's illustrative ramp (SIGGRAPH 2007 *Advances*); the fire-physics literature this crate is still missing |
| [`docs/research/light-pool.md`](docs/research/light-pool.md) | the pool's budget, reach, sort and phases | Olsson, Billeter & Assarsson 2012 (clustered shading); Ghost of Tsushima (SIGGRAPH 2021 *Advances*); *Stochastic tile-based lighting* (SIGGRAPH 2025 *Advances*); RTR4's many-light chapter |
| [`docs/research/flame.md`](docs/research/flame.md) | the flame's shape, noise, ramp and packing | Nguyen, *GPU Gems* ch. 6 §6.1; RTR4 §6.3; Sanctuary decision 0228 |
| [`docs/research/flame-flicker.md`](docs/research/flame-flicker.md) | why a drawn flame breathes with the light it motivates, and how the two share one function | Xia & Zhang 2018; SIGGRAPH 2011 *Advances*' fire talk; Doğaner on motivated sources |
| [`docs/research/flicker-depth.md`](docs/research/flicker-depth.md) | how far a fire dips, who chooses, and a measured ceiling for the default | Mandin & Most (IAFSS 6) on flame fluctuation; Gergely et al. 2020; Doğaner |
| [`docs/research/corpus.md`](docs/research/corpus.md) | what the home-still library holds for cinematography, photography, CG lighting, game-lighting craft, many-light rendering, fire physics and perception — and what it does not | the live corpus |

## Bevy compatibility

| `bevy_lights` | Bevy |
|---|---|
| 0.1 | 0.20 |

## Documentation

`cargo doc --open`. The crate's own docs are the reference; `AGENTS.md` is the guide for anyone — human
or agent — changing it.

## Contributing

Read `AGENTS.md` (identical to `CLAUDE.md`). The short version: every feature lands with a research
note, every default is cited or marked a design default, `src/model/` stays engine-free, `src/` stays
free of game vocabulary (`tests/leaf.rs` enforces both), and a shader change is checked with
`tools/wesl-check` before any capture.

## License

MIT OR Apache-2.0, at your option.
