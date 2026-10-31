//! A campfire and a bonfire: the two fires that are wider than they are tall (`FlameTongues`).
//!
//! A flame is one teardrop, `0.4` of its height wide: right for a torch, wrong for a campfire's 0.8 m bed under a 0.7 m flame
//! or a bonfire's 3 m pyre under a 2 m one, which were drawn as one slender tongue in the middle of a heap of fuel. Here each
//! is a `FireKind` (its light, flame, sparks and smoke all the kind's own) with the kind's `FlameTongues`: a mound of flames
//! of different heights and clocks that part and rejoin as the one fire breathes (`docs/research/flame-tongues.md`).
//!
//! # Knobs
//!
//! `LIGHTS_TONGUES=0` draws the same two fires as one flame each, the comparison; `LIGHTS_TONGUES=n` gives both `n` tongues
//! (`1..=9`). `LIGHTS_WIND=x,z` blows a wind through them and `LIGHTS_SNUFF=s` puts both out at `s` seconds. The default
//! camera is a viewer's eye, 6 m from the pair; `LIGHTS_EYE` and `LIGHTS_LOOK` move it. The `LIGHTS_*` capture switches come
//! from `examples/common/mod.rs`.

mod common;

use bevy::camera::Exposure;
use bevy::prelude::*;
use bevy_lights::*;
use common::{ExampleHarness, GpuTiming, camera, camera_placement, render_plugin, wind_env};

/// The campfire's and the bonfire's places on the clearing, and how bright each pool is at its reach (lux, in the scene's exposure).
const CAMPFIRE: Vec3 = Vec3::new(-2.6, 0.0, 0.0);
const BONFIRE: Vec3 = Vec3::new(3.4, 0.0, -0.6);
const POOL_LUX: f32 = 30.0;

fn main() {
    let (eye, look) = camera_placement(Vec3::new(0.4, 1.6, 6.4), Vec3::new(0.4, 1.0, 0.0));
    let fire = blackbody_colour(1900.0, 1.0);
    App::new()
        .add_plugins(DefaultPlugins.set(render_plugin()))
        .add_plugins((ExampleHarness, FlickerPlugin, FlamePlugin, GpuTiming))
        .insert_resource(ClearColor(Color::srgb(0.010, 0.014, 0.034)))
        .insert_resource(FlameLook {
            color: LinearRgba::rgb(fire[0], fire[1], fire[2]),
            strength: 0.82,
            max_height: 4.0,
        })
        .insert_resource(FlameWind { velocity: wind_env().unwrap_or_default() })
        .insert_resource(GlobalAmbientLight { color: Color::WHITE, brightness: 0.0, ..default() })
        .insert_resource(View(eye, look))
        .add_systems(Startup, spawn)
        .add_systems(Update, snuff_both)
        .run();
}

#[derive(Resource)]
struct View(Vec3, Vec3);

/// `LIGHTS_TONGUES`: `0` is one flame each (the comparison), a number is that many tongues for both, unset the kinds' own.
fn tongues_for(kind: FireKind) -> Option<FlameTongues> {
    match std::env::var("LIGHTS_TONGUES").ok().and_then(|v| v.parse::<u8>().ok()) {
        Some(0) => None,
        Some(count) => {
            let spread = kind.tongues().map_or(0.5, |t| t.spread);
            Some(FlameTongues { count, spread: spread_env().unwrap_or(spread) })
        }
        None => kind.tongues().map(|t| FlameTongues { spread: spread_env().unwrap_or(t.spread), ..t }),
    }
}

/// `LIGHTS_SPREAD=m`: the width of the bed the tongues' feet are spread over, for both fires.
fn spread_env() -> Option<f32> {
    std::env::var("LIGHTS_SPREAD").ok().and_then(|v| v.parse().ok())
}

/// Every fire's light, so `snuff_both` can put them out.
#[derive(Component)]
struct Fire;

fn spawn(
    mut commands: Commands,
    view: Res<View>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        camera(view.0, view.1, 4.0),
        AmbientLight { color: Color::srgb(0.45, 0.55, 1.0), brightness: 40.0, ..default() },
        Exposure { ev100: 6.0 },
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
    let stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.28, 0.26),
        perceptual_roughness: 0.9,
        ..default()
    });
    let wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.11, 0.075, 0.05),
        perceptual_roughness: 0.85,
        ..default()
    });
    let coals = materials.add(StandardMaterial {
        base_color: Color::srgb(0.04, 0.02, 0.015),
        emissive: LinearRgba::rgb(0.9, 0.16, 0.02) * 1.4,
        perceptual_roughness: 0.9,
        ..default()
    });
    let colour = {
        let [r, g, b] = blackbody_colour(1900.0, 1.0);
        Color::from(LinearRgba::rgb(r, g, b))
    };
    for (kind, at) in [(FireKind::Campfire, CAMPFIRE), (FireKind::Bonfire, BONFIRE)] {
        let bed = kind.diameter();
        // A bed of coals and the stones round it (a campfire's ring; a bonfire's pyre has the logs).
        commands.spawn((
            Name::new("Bed"),
            Mesh3d(meshes.add(Cylinder::new(0.5 * bed, 0.06))),
            MeshMaterial3d(coals.clone()),
            Transform::from_translation(at + Vec3::Y * 0.03),
        ));
        let ring = if kind == FireKind::Campfire { 9 } else { 14 };
        for i in 0..ring {
            let a = i as f32 / ring as f32 * std::f32::consts::TAU;
            let r = 0.5 * bed + 0.14;
            commands.spawn((
                Name::new("Stone"),
                Mesh3d(meshes.add(Sphere::new(if kind == FireKind::Campfire { 0.13 } else { 0.28 }))),
                MeshMaterial3d(stone.clone()),
                Transform::from_translation(at + Vec3::new(a.cos() * r, 0.07, a.sin() * r))
                    .with_scale(Vec3::new(1.0, 0.7, 1.0)),
            ));
        }
        // Logs laid across the bed: three for a campfire, a stacked pyre of eight for a bonfire.
        let logs = if kind == FireKind::Campfire { 3 } else { 8 };
        for i in 0..logs {
            let turn = i as f32 * 2.399963;
            let (length, radius, lift) = if kind == FireKind::Campfire {
                (0.7, 0.045, 0.06 + 0.05 * (i % 2) as f32)
            } else {
                (2.2, 0.11, 0.12 + 0.2 * (i / 3) as f32)
            };
            commands.spawn((
                Name::new("Log"),
                Mesh3d(meshes.add(Cylinder::new(radius, length))),
                MeshMaterial3d(wood.clone()),
                Transform::from_translation(at + Vec3::Y * lift).with_rotation(
                    Quat::from_rotation_y(turn) * Quat::from_rotation_z(std::f32::consts::FRAC_PI_2 - 0.06),
                ),
            ));
        }
        // The light: one fire, a light at the middle of the flame, sized to its pool, and the flame that is its fire.
        let light = commands
            .spawn((
                Name::new("Fire light"),
                Fire,
                PointLight {
                    intensity: kind.lumens(POOL_LUX, kind.height() * 0.5),
                    range: kind.reach() * 3.0,
                    color: colour,
                    shadow_maps_enabled: false,
                    ..default()
                },
                kind.flicker(kind.lumens(POOL_LUX, kind.height() * 0.5)),
                Transform::from_translation(at + Vec3::Y * kind.height() * 0.5),
            ))
            .id();
        let mut site = commands.spawn((
            Name::new("Flame site"),
            FlameSite { fire: Some(light), ..kind.site() },
            Transform::from_translation(at + Vec3::Y * 0.06),
        ));
        if let Some(embers) = kind.embers() {
            site.insert(embers);
        }
        if let Some(smoke) = kind.smoke() {
            site.insert(smoke);
        }
        if let Some(tongues) = tongues_for(kind) {
            site.insert(tongues);
        }
    }
}

/// `LIGHTS_SNUFF=s`: both fires are put out at `s` seconds, light and every tongue together.
fn snuff_both(
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
