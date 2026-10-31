//! Throwaway prototype: a campfire as a cluster of separate flame sites. Never committed.
mod common;

use bevy::camera::Exposure;
use bevy::prelude::*;
use bevy_lights::*;
use common::{ExampleHarness, camera, camera_placement};

fn envf(name: &str, default: f32) -> f32 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn hash(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9e37_79b1) ^ 0x85eb_ca6b;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2c1b_3c6d);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297a_2d39);
    x ^= x >> 15;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

fn main() {
    let (eye, look) = camera_placement(Vec3::new(0.0, 1.4, 5.0), Vec3::new(0.0, 0.6, 0.0));
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((ExampleHarness, FlamePlugin, FlickerPlugin))
        .insert_resource(ClearColor(Color::srgb(0.010, 0.014, 0.034)))
        .insert_resource(FlameLook { color: LinearRgba::rgb(1.0, 0.55, 0.18), strength: 1.0, max_height: 4.0 })
        .insert_resource(GlobalAmbientLight { color: Color::WHITE, brightness: 0.0, ..default() })
        .insert_resource(Eye(eye, look))
        .add_systems(Startup, spawn)
        .run();
}

#[derive(Resource)]
struct Eye(Vec3, Vec3);

fn spawn(
    mut commands: Commands,
    eye: Res<Eye>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        camera(eye.0, eye.1, 4.0),
        AmbientLight { color: Color::srgb(0.45, 0.55, 1.0), brightness: 40.0, ..default() },
        Exposure { ev100: 6.0 },
    ));
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(40.0, 40.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.16, 0.14, 0.13),
            perceptual_roughness: 0.95,
            ..default()
        })),
    ));
    let n = envf("TONGUES", 1.0) as u32;
    let spread = envf("SPREAD", 0.8);
    let height = envf("HEIGHT", 0.7);
    let kind = FireKind::Campfire;
    let lumens = kind.lumens(40.0, 0.5);
    // the bed
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(spread * 0.5, 0.05))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.05, 0.04, 0.03),
            emissive: LinearRgba::rgb(0.9, 0.2, 0.03) * 1.5,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.025, 0.0),
    ));
    commands.spawn((
        PointLight { intensity: lumens, range: 12.0, shadow_maps_enabled: false, color: Color::srgb(1.0, 0.58, 0.25), ..default() },
        kind.flicker(lumens),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    let mut centre = commands.spawn((Name::new("campfire"), Transform::from_xyz(0.0, 0.05, 0.0), Visibility::default()));
    for k in 0..n {
        let (offset, h, phase) = if k == 0 {
            (Vec2::ZERO, height, 0.0)
        } else {
            let a = k as f32 * 2.399963 + hash(k) * 0.6;
            let r = spread * 0.5 * (0.45 + 0.30 * hash(k + 40));
            let hk = height * (0.50 + 0.32 * hash(k + 80) - 0.15 * r / (spread * 0.5));
            (Vec2::new(a.cos(), a.sin()) * r, hk, hash(k + 120) * std::f32::consts::TAU)
        };
        let site = FlameSite { height: h, phase, ..kind.site() };
        centre.with_children(|p| {
            let mut e = p.spawn((site, Transform::from_xyz(offset.x, 0.0, offset.y)));
            if k == 0 {
                if let Some(embers) = kind.embers() {
                    e.insert(embers);
                }
                if let Some(smoke) = kind.smoke() {
                    e.insert(smoke);
                }
            }
        });
    }
}
