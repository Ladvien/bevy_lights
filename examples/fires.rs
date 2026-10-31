//! One fire, one component: `Fire` on an entity, and nothing else.
//!
//! `LIGHTS_KIND=<name>` picks the kind (`candle`, `lantern`, `torch`, `brazier`, `campfire` or `bonfire`; default `torch`), and the
//! camera stands a fixed number of flame heights back, so every kind fills the frame alike. Each is `Fire::lit(kind, lux, over)`: the
//! kind's flame, the light the pool hands it, and its own sparks, smoke, tongues and bed of coals, with no component written by hand.
//!
//! `LIGHTS_COLOUR=r,g,b` (linear) gives the fire a colour of its own, the light's and, through `flame_mid`, the flame's;
//! `LIGHTS_SMOKE=0`, `LIGHTS_EMBERS=0`, `LIGHTS_TONGUES=0` and `LIGHTS_BED=0` turn that extra of the kind off (the `Fire` flags);
//! `LIGHTS_ROW=n` lays `n` of them side by side, each with its own phase (a frame sequence shows them breathing apart);
//! `LIGHTS_SNUFF=s` puts them all out at `s` seconds and `LIGHTS_WIND=x,z` blows a wind through them. `LIGHTS_WALK=v` carries every fire
//! along +X at `v` metres per second for `LIGHTS_WALK_FOR` seconds (default 1.5) and then stops, the camera standing back (a fire's
//! flame streams behind it while it moves and stands up again over about half a second when it stops: `FlameMotion`). The `LIGHTS_*` capture
//! switches come from `examples/common/mod.rs`.

mod common;

use bevy::camera::Exposure;
use bevy::prelude::*;
use bevy_lights::*;
use common::{ExampleHarness, camera, camera_placement, render_plugin, wind_env};

/// How bright the fire's pool is at its edge, in lux, in the scene's exposure.
const POOL_LUX: f32 = 30.0;

fn kind_env() -> FireKind {
    std::env::var("LIGHTS_KIND").ok().and_then(|v| v.parse().ok()).unwrap_or(FireKind::Torch)
}

/// The fire's own colour, from `LIGHTS_COLOUR=r,g,b`.
fn colour_env() -> Option<Color> {
    let v: Vec<f32> =
        std::env::var("LIGHTS_COLOUR").ok()?.split(',').filter_map(|p| p.trim().parse().ok()).collect();
    (v.len() == 3).then(|| Color::from(LinearRgba::rgb(v[0], v[1], v[2])))
}

/// `LIGHTS_<NAME>=0` turns an extra off.
fn off(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| v == "0")
}

/// `LIGHTS_WALK=v` (and `LIGHTS_WALK_FOR=s`): the speed the fires are carried at and for how long, if they are.
fn walk_env() -> Option<(f32, f32)> {
    let speed = std::env::var("LIGHTS_WALK").ok()?.parse().ok()?;
    let seconds = std::env::var("LIGHTS_WALK_FOR").ok().and_then(|v| v.parse().ok()).unwrap_or(1.5);
    Some((speed, seconds))
}

/// `LIGHTS_ROW=n`: how many fires side by side (`1..=40`).
fn row_env() -> usize {
    std::env::var("LIGHTS_ROW").ok().and_then(|v| v.parse().ok()).unwrap_or(1).clamp(1, 40)
}

fn main() {
    let kind = kind_env();
    let height = kind.height();
    // A fixed number of flame heights back and up, so a candle and a bonfire fill the frame alike.
    let (eye, look) = camera_placement(
        Vec3::new(
            0.0,
            height * 0.55 + 0.1,
            (height * 2.6).max(if walk_env().is_some() { 3.5 } else { 0.45 }),
        ),
        Vec3::new(0.0, height * 0.45, 0.0),
    );
    App::new()
        .add_plugins(DefaultPlugins.set(render_plugin()))
        .add_plugins((
            ExampleHarness,
            FlickerPlugin,
            LightPoolPlugin::<Viewer>::default(),
            FlamePlugin,
            FirePlugin,
        ))
        .insert_resource(ClearColor(Color::srgb(0.010, 0.014, 0.034)))
        .insert_resource(FlameLook { max_height: 4.0, ..default() })
        .insert_resource(FlameWind { velocity: wind_env().unwrap_or_default() })
        .insert_resource(GlobalAmbientLight { color: Color::WHITE, brightness: 0.0, ..default() })
        .insert_resource(LightPool {
            size: row_env().min(16),
            reach: 100.0,
            light: PointLight { range: kind.reach() * 3.0, shadow_maps_enabled: false, ..default() },
            ..default()
        })
        .insert_resource(View(eye, look))
        .add_systems(Startup, spawn)
        .add_systems(Update, (snuff_all, carry_all))
        .run();
}

#[derive(Resource)]
struct View(Vec3, Vec3);

/// What the pool measures from: the camera's place.
#[derive(Component)]
struct Viewer;

fn spawn(
    mut commands: Commands,
    view: Res<View>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        camera(view.0, view.1, 2.0),
        AmbientLight { color: Color::srgb(0.45, 0.55, 1.0), brightness: 40.0, ..default() },
        Exposure { ev100: 6.0 },
        Viewer,
    ));
    let ground = materials.add(StandardMaterial {
        base_color: Color::srgb(0.16, 0.14, 0.13),
        perceptual_roughness: 0.95,
        ..default()
    });
    commands.spawn((
        Name::new("Ground"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(60.0, 60.0))),
        MeshMaterial3d(ground),
    ));

    let kind = kind_env();
    let row = row_env();
    // Side by side, a flame height and a half apart (a bonfire's tongues are wider than that, so a row of them overlaps, as a row of
    // bonfires would).
    let gap = (kind.height() * 1.5).max(0.2);
    for i in 0..row {
        let x = (i as f32 - (row as f32 - 1.0) * 0.5) * gap;
        commands.spawn((
            Name::new(format!("{kind} {i}")),
            Fire {
                color: colour_env(),
                embers: !off("LIGHTS_EMBERS"),
                smoke: !off("LIGHTS_SMOKE"),
                tongues: !off("LIGHTS_TONGUES"),
                bed: !off("LIGHTS_BED"),
                ..Fire::lit(kind, POOL_LUX, kind.height() * 0.5)
            },
            Transform::from_xyz(x, 0.0, 0.0),
            Home(x),
        ));
    }
}

/// Where a fire was placed along X, so a walk is centred on it.
#[derive(Component)]
struct Home(f32);

/// `LIGHTS_WALK=v`: every fire is carried along +X at `v` for the walk's length, centred on where it was placed, then stands still.
fn carry_all(time: Res<Time>, mut fires: Query<(&mut Transform, &Home)>) {
    let Some((speed, seconds)) = walk_env() else {
        return;
    };
    let walked = time.elapsed_secs().min(seconds);
    for (mut at, home) in &mut fires {
        at.translation.x = home.0 + speed * (walked - seconds * 0.5);
    }
}

/// `LIGHTS_SNUFF=s`: every fire is put out at `s` seconds, the flame and the light it was handed together.
fn snuff_all(
    mut commands: Commands,
    time: Res<Time>,
    fires: Query<Entity, (With<Fire>, Without<Snuff>)>,
    mut done: Local<bool>,
) {
    let Some(at) = std::env::var("LIGHTS_SNUFF").ok().and_then(|v| v.parse::<f32>().ok()) else {
        return;
    };
    if *done || time.elapsed_secs() < at {
        return;
    }
    *done = true;
    for fire in &fires {
        commands.entity(fire).insert(Snuff::default());
    }
}
