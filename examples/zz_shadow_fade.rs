//! THROWAWAY (iteration 271): a shadow handed from one pooled torch to another, nothing else moving.
//! The focus jumps from near torch A to near torch B at frame 120, so the ranking swaps and the one
//! shadow moves from A to B. `LIGHTS_FADE=0` is the pop for comparison.

mod common;

use bevy::prelude::*;
use bevy_lights::*;
use common::*;

#[derive(Component)]
struct Focus;

#[derive(Resource, Default)]
struct Frame(u32);

fn main() {
    let fade = std::env::var("LIGHTS_FADE").ok().and_then(|v| v.parse().ok()).unwrap_or(FADE_SECONDS);
    let mut light = PointLight { range: 8.0, shadow_maps_enabled: true, intensity: 0.0, ..default() };
    light.color = Color::srgb(1.0, 0.62, 0.32);
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((ExampleHarness, FlickerPlugin, LightPoolPlugin::<Focus>::default()))
        .insert_resource(LightPool {
            size: 2,
            reach: 50.0,
            reserve: 2,
            max_shadowed: Some(1),
            fade_seconds: fade,
            light,
            ..default()
        })
        .insert_resource(GlobalAmbientLight { color: Color::WHITE, brightness: 0.0, ..default() })
        .init_resource::<Frame>()
        .add_systems(Startup, setup)
        .add_systems(Update, jump)
        .run();
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let stone = materials.add(StandardMaterial { base_color: Color::srgb(0.5, 0.48, 0.45), perceptual_roughness: 0.9, ..default() });
    commands.spawn((Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))), MeshMaterial3d(stone.clone())));
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.6, 3.0, 0.6))), MeshMaterial3d(stone), Transform::from_xyz(0.0, 1.5, 0.0)));
    // A: south of the pillar; B: east of it. Each a steady torch (no flicker depth), 1.4 m up.
    for at in [Vec3::new(0.0, 0.0, -2.0), Vec3::new(2.0, 0.0, 0.0)] {
        commands.spawn((
            PoolCandidate { offset: Vec3::Y * 1.4, intensity: 40_000.0, depth: Some(0.0), ..default() },
            Transform::from_translation(at),
        ));
    }
    commands.spawn((Focus, Transform::from_xyz(0.0, 0.0, -5.0)));
    let (eye, look) = camera_placement(Vec3::new(-5.0, 9.0, 5.0), Vec3::ZERO);
    commands.spawn((camera(eye, look, 8.0), AmbientLight { color: Color::srgb(0.4, 0.5, 1.0), brightness: 20.0, ..default() }));
}

fn jump(mut frame: ResMut<Frame>, mut focus: Query<&mut Transform, With<Focus>>) {
    frame.0 += 1;
    if frame.0 == 120 {
        for mut at in &mut focus {
            at.translation = Vec3::new(5.0, 0.0, 0.0);
        }
    }
}
