//! The depth fade: a flame whose torch the camera cannot see goes out with it (issue #2).
//!
//! A low wall across a dark yard and three torches, each a [`FlameSite`] on a post:
//!
//! - **behind the wall** (x = -2.5): its post is hidden and its flame, 1.4 m tall, reaches over the
//!   top. Without [`FlameSite::depth_fade`] that tongue of fire hangs over the wall with nothing under
//!   it, which is what Sanctuary's playtesters asked about ("is that a torch on the other side of the
//!   wall?"). With it the flame goes out as its source goes behind the stone.
//! - **on the wall's face** (x = 2.5): the source stands 5 cm in front of the stone. It never fades.
//! - **past the wall's end** (x = 4.6): seen past the edge when the camera swings round.
//!
//! # Knobs
//!
//! `LIGHTS_DEPTH_FADE=1` turns the fade on (and gives the camera the `DepthPrepass` it reads), so the
//! same scene runs both ways; `LIGHTS_PAN=1` swings the camera along x from -24 to -9 over
//! `LIGHTS_FRAMES` frames, aimed at the torch behind the wall, which carries the source past the wall's
//! end and back into view. The `LIGHTS_*` capture switches come from `examples/common/mod.rs`.

mod common;

use bevy::camera::Exposure;
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::prelude::*;
use bevy_lights::*;
use common::{ExampleHarness, FlyCamera, camera, camera_placement};

/// The torch behind the wall: its post's base, which the pan aims at.
const BEHIND: Vec3 = Vec3::new(-2.5, 0.0, -1.2);
/// The camera's height and distance in front of the wall, and the x range the pan sweeps.
const EYE_HEIGHT: f32 = 1.7;
const EYE_DISTANCE: f32 = 6.0;
const PAN: (f32, f32) = (-24.0, -9.0);

fn fade_on() -> bool {
    std::env::var("LIGHTS_DEPTH_FADE").is_ok_and(|v| v != "0")
}

fn main() {
    let (eye, look) = camera_placement(Vec3::new(0.0, 2.0, 8.0), Vec3::new(0.0, 1.0, 0.0));
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((ExampleHarness, FlamePlugin))
        .insert_resource(ClearColor(Color::srgb(0.010, 0.014, 0.034)))
        .insert_resource(FlameLook { color: LinearRgba::rgb(1.0, 0.45, 0.12), strength: 0.82, max_height: 2.0 })
        .insert_resource(GlobalAmbientLight { color: Color::WHITE, brightness: 0.0, ..default() })
        .insert_resource(Eye(eye, look))
        .add_systems(Startup, spawn)
        .add_systems(Update, pan)
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
    let mut view = commands.spawn((
        camera(eye.0, eye.1, 4.0),
        AmbientLight { color: Color::srgb(0.45, 0.55, 1.0), brightness: 40.0, ..default() },
        Exposure { ev100: 6.0 },
    ));
    if fade_on() {
        // The fade reads this: without it the flame has no depth to test and draws whole.
        view.insert(DepthPrepass);
    }
    let stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.30, 0.28),
        perceptual_roughness: 0.9,
        ..default()
    });
    let wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.08, 0.06),
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.spawn((
        Name::new("Ground"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(60.0, 60.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.12, 0.11, 0.13),
            perceptual_roughness: 0.95,
            ..default()
        })),
    ));
    commands.spawn((
        Name::new("Wall"),
        Mesh3d(meshes.add(Cuboid::new(8.0, 1.6, 0.4))),
        MeshMaterial3d(stone),
        Transform::from_xyz(0.0, 0.8, 0.0),
    ));
    let post = meshes.add(Cuboid::new(0.1, 0.8, 0.1));
    let site = |phase: f32, height: f32| FlameSite {
        height,
        phase,
        kindle: 0.0,
        diameter: None,
        depth: FLICKER_DEPTH,
        depth_fade: fade_on(),
    };
    // Behind the wall, and past its end: a post 0.8 m tall with a metre or more of flame on top.
    for (name, at, phase, height) in [("Torch behind the wall", BEHIND, 0.3, 1.4), ("Torch past the end", Vec3::new(4.6, 0.0, -0.8), 2.1, 1.0)] {
        commands.spawn((
            Name::new(name),
            Mesh3d(post.clone()),
            MeshMaterial3d(wood.clone()),
            Transform::from_translation(at + Vec3::Y * 0.4),
            children![(Name::new("Flame site"), site(phase, height), Transform::from_xyz(0.0, 0.4, 0.0))],
        ));
    }
    // On the wall's face, 5 cm proud of the stone.
    commands.spawn((
        Name::new("Torch on the face"),
        Mesh3d(post),
        MeshMaterial3d(wood),
        Transform::from_xyz(2.5, 1.1, 0.25),
        children![(Name::new("Flame site"), site(1.1, 0.5), Transform::from_xyz(0.0, 0.4, 0.0))],
    ));
}

/// `LIGHTS_PAN=1`: the camera's x runs from `PAN.0` to `PAN.1` over the run's `LIGHTS_FRAMES`, aimed at the torch
/// behind the wall.
fn pan(mut frame: Local<u32>, mut cameras: Query<(&mut Transform, &mut FlyCamera)>) {
    if !std::env::var("LIGHTS_PAN").is_ok_and(|v| v != "0") {
        return;
    }
    let frames: f32 = std::env::var("LIGHTS_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(300.0);
    *frame += 1;
    let x = PAN.0 + (PAN.1 - PAN.0) * (*frame as f32 / frames).min(1.0);
    for (mut transform, mut fly) in &mut cameras {
        fly.aim(&mut transform, Vec3::new(x, EYE_HEIGHT, EYE_DISTANCE), BEHIND + Vec3::Y * 1.3);
    }
}
