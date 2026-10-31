# bevy_lights

Practical lights for Bevy: the fires a scene's props actually carry — a budgeted pool of real lights
over thousands of virtual ones, with shader flames and the flicker that makes them read.

> ⚠️ **Vibe Coded.** Written by AI agents working from a human's direction, covered by tests rather than a line-by-line human review. Read the code before you rely on it for something you care about.

[![crates.io](https://img.shields.io/crates/v/bevy_lights.svg)](https://crates.io/crates/bevy_lights)
[![docs.rs](https://docs.rs/bevy_lights/badge.svg)](https://docs.rs/bevy_lights)
[![CI](https://github.com/bevy-coven/bevy_lights/actions/workflows/ci.yml/badge.svg)](https://github.com/bevy-coven/bevy_lights/actions/workflows/ci.yml)
[![Bevy](https://img.shields.io/badge/bevy-0.20-blue.svg)](https://bevyengine.org)
[![licence](https://img.shields.io/badge/licence-MIT%20OR%20Apache--2.0-blue.svg)](#who-and-licence)

![Four braziers, each with a real light of its own, and 48 torch-bearers sharing an 8-light pool in a dark courtyard](https://raw.githubusercontent.com/bevy-coven/bevy_lights/main/docs/media/showcase.gif)

*`examples/courtyard`: four braziers burning with real lights and a 0.34 m bed of coals, and 48 walking torch-bearers whose flames lean back as they walk, sharing eight pooled lights of which three cast shadows.*

**Status:** usable · Bevy 0.20 · MSRV 1.97.1

**Why:** Bevy will happily shade a hundred `PointLight`s and give you no way to choose which hundred of
your thousand torches they are — this crate ranks candidates by the light they put on you, hands the
budget over without a pop, and draws the flame that motivates each one.

## Quick start

```sh
cargo add bevy_lights
```

```rust,no_run
use bevy::prelude::*;
use bevy_lights::*;

#[derive(Component)]
struct MyFocus;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((FlickerPlugin, LightPoolPlugin::<MyFocus>::default(), FlamePlugin, FirePlugin))
        .add_systems(Startup, |mut commands: Commands| {
            let fire = Fire::lit(FireKind::Campfire, 250.0, 0.35);
            commands.spawn((fire, Transform::from_xyz(4.0, 0.0, 2.0)));
            commands.spawn((MyFocus, Transform::default()));
        })
        .run();
}
```

`Fire::lit(kind, lux, over)` is a whole fire in one component: the kind's flame, the light the pool hands
it, its sparks, smoke, tongues and bed of coals, each on a clock of its own. For every number spelled out
by hand, [`fires.md`](https://github.com/bevy-coven/bevy_lights/blob/main/docs/guide/fires.md) has the same fire as loose components.

`serde` is an optional feature (`cargo add bevy_lights --features serde`) that derives `serde` on the
components and resources a host saves, so a placed fire is one line of a scene file.

## Try it

```sh
cargo run --release --example courtyard
```

That is the courtyard the GIF above shows. `cargo run --release --example fires` is the same crate as six
single fires — a candle to a bonfire — one per run: `LIGHTS_KIND=bonfire cargo run --release --example fires`.

![The six fire kinds, each one Fire component on an otherwise empty entity, each with its flame, the pool its light throws on the ground, and its own sparks, smoke, tongues and bed of coals](https://raw.githubusercontent.com/bevy-coven/bevy_lights/main/docs/media/fires.png)

*`LIGHTS_KIND=<kind> cargo run --release --example fires`: a candle, a lantern, a torch, a brazier, a campfire and a bonfire, the camera standing the same number of flame heights back each time.*

## What you can do

- **Light a fire that flickers, flares and goes out** (`FlickerPlugin`): `Flicker` modulates a light below its base at a rate its fuel bed sets, `Kindle` flares it up over 0.6 s, and `Snuff` puts it out over 1.2 s with the linked flame — see [fires](https://github.com/bevy-coven/bevy_lights/blob/main/docs/guide/fires.md).
- **Spend eight real lights on a town of thousands** (`LightPoolPlugin`): ranked by the light each candidate puts on you, held with hysteresis, cross-faded on a hand-off, with a shadow budget of its own — see [the light pool](https://github.com/bevy-coven/bevy_lights/blob/main/docs/guide/light-pool.md).
- **Draw the flame that motivates the light** (`FlamePlugin`): one additive billboard per fire, breathing on its light's clock, in a colour of its own, shedding sparks, smoking, leaning in a wind and streaming behind a torch you carry — see [shader flames](https://github.com/bevy-coven/bevy_lights/blob/main/docs/guide/flames.md).
- **Let a fire leave coals behind:** `FuelBed` is a temperature field that breathes with the flicker, warms as the fire catches and goes on glowing, cooling, after it is put out — see [the fuel bed](https://github.com/bevy-coven/bevy_lights/blob/main/docs/guide/fuel-bed.md).
- **Size and colour a fire like a cinematographer:** `fire_lumens`, `fill_for`, `blackbody_colour` and `white_balanced` turn "1000 lx at 4 m" and "1900 K" into a light — see [the lighting look](https://github.com/bevy-coven/bevy_lights/blob/main/docs/guide/lighting-look.md).
- **Reuse every curve outside the engine:** `bevy_lights::model` is the same maths with no `bevy` in it — see [engine-free maths](https://github.com/bevy-coven/bevy_lights/blob/main/docs/guide/model.md).

## Part of the Coven

`bevy_lights` is one of the [Bevy Coven](https://github.com/bevy-coven) crates: the family table there
says which crate owns what, so two of them never both own the sun. Grimoire, the Coven's level editor,
takes it as an optional feature and edits a placed fire's own settings — see
[use it in Grimoire](https://bevy-coven.github.io/crates/bevy_lights.html#use-it-in-grimoire).

## Bevy compatibility

| `bevy_lights` | Bevy |
|---|---|
| 0.1 – 0.5 | 0.20 |

## Learn more

- [docs.rs](https://docs.rs/bevy_lights) — the API reference (`cargo doc --open`).
- [The book page](https://bevy-coven.github.io/crates/bevy_lights.html) — the same guide with the pictures.
- [`docs/guide/`](https://github.com/bevy-coven/bevy_lights/blob/main/docs/guide/README.md) — the detail the README used to carry: the flicker's law, the pool's hand-off, the flame shader, the coals.
- [`examples/`](https://github.com/bevy-coven/bevy_lights/tree/main/examples) — `courtyard` (the showcase), `fires` (one fire of each kind), `campfire`, `occlusion`, and the benches `bench_lights` and `bench_flames`.
- [`docs/DEMOS.md`](https://github.com/bevy-coven/bevy_lights/blob/main/docs/DEMOS.md) — how every capture in `docs/media/` is made.
- [`CHANGELOG.md`](https://github.com/bevy-coven/bevy_lights/blob/main/CHANGELOG.md) — what each release changed, and what it broke.

## Who and licence

Maintainer [@Ladvien](https://github.com/Ladvien). Questions and bugs go to
[the issues](https://github.com/bevy-coven/bevy_lights/issues), and a game asking for a change here uses
the `consumer` label. Contributions follow the org's
[`CONTRIBUTING.md`](https://github.com/bevy-coven/.github/blob/main/CONTRIBUTING.md); the crate's own
rules for changing it are in [`AGENTS.md`](https://github.com/bevy-coven/bevy_lights/blob/main/AGENTS.md).

MIT OR Apache-2.0, at your option.
