# Fires: flicker, kindle, snuff, and a whole fire in one component

Every `PointLight` and `SpotLight` carrying `Flicker` is modulated **below** its base intensity by a
summed sine: the multiplier runs `1 - 2·depth ..= 1`, so `Flicker::base` is the fire's brightest moment
and a deeper flicker is a darker trough rather than a brighter crest. At the default `Flicker::depth`
of `FLICKER_DEPTH` that is 0.70–1.00 of base with a mean of 0.85 — the window a lumen budget written
against the mean assumes. `Flicker::depth` is how far that fire dips, so a lantern can ask for 0.03 and
a gusting torch for 0.3; `flicker_depth` clamps the value to `0..=MAX_FLICKER_DEPTH` (0.5, the deepest
whose trough is still zero) and reads a `NaN` as the default. `CALM_FLICKER_DEPTH` (0.05) and `WHIPPING_FLICKER_DEPTH` (0.3) name the two ends, and `flicker_mean(depth)` is the multiplier's mean, so a budget is written with `Flicker::at_mean(lumens, depth)` (or `PoolCandidate::at_mean`) rather than a hand-divided `0.85`. A system that rewrites a light's lumens every frame calls `Flicker::set_mean`, and a fire that changes how it breathes calls `Flicker::set_depth`, which keeps the mean: a deeper depth never silently dims the fire.

Give a fire its fuel-bed size (`Flicker { diameter: Some(0.34) }`) and its flicker runs at the measured
puffing rate, `1.5/√D` Hz — a brazier at 2.6 Hz, a pyre at 0.75 — so a large fire breathes more slowly
than a small one. `diameter: None` is the crate's inherited un-sized rate, for a fire whose size is not
modelled.

**Fire kinds.** `FireKind::{Candle, Lantern, Torch, Brazier, Campfire, Bonfire}` (`FireKind::ALL`, serde as the name) bundles a fuel-bed diameter, a flicker depth and a flame height, so a picker or a project file stores one word: `FireKind::Torch.flicker(lumens)` is a `Flicker` averaging `lumens` and `FireKind::Torch.site()` a `FlameSite`, built from the same three numbers so the light and its flame breathe together (spell the rest with `..`: `FlameSite { phase: 1.3, ..FireKind::Torch.site() }`). `rate()` is the puffing law at that size, 13 Hz for a candle to 0.87 Hz for a bonfire. Only the brazier row is a number the crate shipped before; the rest are design defaults ([`docs/research/fire-kinds.md`](../research/fire-kinds.md)). `LIGHTS_KIND=<name>` dresses the courtyard's braziers as a kind. `FireKind::reach()` is the metres of ground a kind lights to its pool's edge (1 m for a candle to 9 m for a bonfire), and `FireKind::light(lux, over)` is the `Flicker` whose lumens put `lux` there, so a picker places a fire without typing a lumen number; the brazier's 4 m and the torch's 2 m are the courtyard's own pools, the rest design defaults ([`docs/research/fire-reach.md`](../research/fire-reach.md)).

A light with `Kindle` flares up over `KINDLE_SECONDS` (0.6 s) instead of switching on, on
`kindle_gain`'s curve: a smoothstep with a flare running ahead of it, so it is 0.85 of the way at half
time, tips nine tenths of one percent above its base at about 0.46 s, and settles at exactly 1.
`Kindled { entity }` is the message a host hears on the frame a fire catches, for the sound of it. An
invisible light is not rewritten (nothing is extracted for it) but its kindle still counts.

**A whole fire.** A fire is up to six components that must agree (`FlameSite`, `PoolCandidate`, and for the kinds that have them `FlameEmbers`, `FlameSmoke`, `FlameTongues`, `FuelBed`), and `Fire` is the one value that writes them: `Fire::lit(FireKind::Campfire, 250.0, 0.4)` (a kind, and the lux its pool puts on the ground at its edge from a light 0.4 m up) or `Fire { color: Some(blue), ..Fire::new(FireKind::Torch) }`. `FirePlugin` (add it with `FlamePlugin`, `FlickerPlugin` and `LightPoolPlugin`, which it reports missing rather than adds) gives the entity the kind's flame, a `PoolCandidate` the pool hands a light with the flame's own rate, depth and phase and the light standing in the flame, and the kind's sparks, smoke, tongues and bed. A `phase` of `None` is derived from the entity (`fire_phase`), so fires placed alike do not breathe together. `color` is the light's colour and the flame's middle at the shipped flame's brightness (`flame_mid`); `None` is a wood fire, a 1900 K light (`FIRE_KELVIN`; a `PoolCandidate` with no colour takes the pool template's, which is white) and the scene's own flame. `embers`, `smoke`, `tongues` and `bed` turn a kind's extra off (the bed stays coal-coloured under a coloured flame, so a magic fire turns it off). What a host spawns beside a `Fire` wins (`(Fire::new(kind), FlameSmoke { density: 0.2 })`), and a `Fire` that is changed afterwards (replaced, or edited through reflection or BRP) writes every part again and removes what it no longer names, so a picker that turns a campfire into a lantern leaves no tongues behind. Put it out and light it again with `Snuff` and `Kindle` on the same entity. It is `serde` with every field optional (`(kind: Campfire)` is a campfire). [`docs/research/fire-component.md`](../research/fire-component.md).

![The six fire kinds, each one Fire component on an otherwise empty entity: a candle, a lantern, a torch, a brazier, a campfire and a bonfire, each with its flame, the warm pool its light throws on the ground and its own sparks, smoke, tongues and bed of coals where the kind has them](../media/fires.png)

*`LIGHTS_KIND=<kind> cargo run --release --example fires` (1578 × 654, an 8-bit PNG: frame 240 of six 1280 × 720 runs, each scaled to 520 × 293 with the kind's name burned in). The camera stands the same number of flame heights back, so each kind fills its panel. The brazier has no bowl in this example, so its flame stands on its bed of coals, which is the small red disc; every light is a pooled one the `Fire` was handed.*

## A fire with every number spelled out

`Fire` is the way in, and the parts it writes are public: spawn them yourself when a scene needs its own
lumens, its own colour and its own light position. This is the same courtyard fire as the README's quick
start, as components.

```rust,no_run
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
            FirePlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    // A whole fire is one component. A campfire: the kind's flame, tongues, bed of coals, sparks and
    // smoke, and a light the pool hands it when it is near, sized so its pool puts 250 lx on the ground
    // at its edge (the light sits in the flame: 0.35 m up). Every fire gets a clock of its own.
    commands.spawn((Fire::lit(FireKind::Campfire, 250.0, 0.35), Transform::from_xyz(4.0, 0.0, 2.0)));
    // A torch in a colour of its own: the light is that colour and the flame is the same hue.
    commands.spawn((
        Fire { color: Some(Color::srgb(0.2, 0.9, 0.3)), ..Fire::new(FireKind::Torch) },
        Transform::from_xyz(-2.0, 1.5, 3.0),
    ));

    // The same parts by hand, when you want every number yourself. A brazier with a real light of its
    // own, sized by the pool it lights (1000 lx at 4 m, from 0.9 m up) and coloured by the temperature
    // it burns at.
    let lumens = fire_lumens(1000.0, 4.0, 0.9);
    let [r, g, b] = blackbody_colour(1900.0, 1.0);
    commands.spawn((
        PointLight { color: LinearRgba::rgb(r, g, b).into(), intensity: lumens, range: 10.0, ..default() },
        // `diameter` sizes the flicker by the fuel bed (`f = 1.5/√D` Hz); `None` uses the un-sized rate.
        Flicker { phase: 0.0, diameter: Some(0.34), ..Flicker::at_mean(lumens, FLICKER_DEPTH) },
        Kindle::default(),
        // `kindle` grows the flame with its light, on the same 0.6 s envelope, and `diameter` matches
        // the `Flicker`'s so the drawn flame breathes at the same rate the light does. `depth` is how
        // far it dips: give the flame the same one as its light, or the two disagree.
        FlameSite { height: 1.1, phase: 0.0, kindle: KINDLE_SECONDS, diameter: Some(0.34), depth: FLICKER_DEPTH, depth_fade: false, fire: None, color: None },
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

A light you want to be able to re-light is re-inserted with `Kindle::default()`, which writes `Kindled`
again. A fire you want dimmer or brighter is changed through the `Flicker` (`set_mean`, never the
light's own `intensity`: the crate writes the intensity every frame).

`examples/courtyard.rs` is the full showcase, `examples/bench_lights.rs` the CPU bench for the pool and
the flicker, and `examples/bench_flames.rs` the bench for thousands of flames with few of them on screen
(it renders off-screen and needs a GPU adapter).
