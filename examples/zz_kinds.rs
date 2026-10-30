//! THROWAWAY (iteration 274, never committed): the six `FireKind::fire` bundles on the ground in a row, in a wind
//! (`LIGHTS_WIND`), against a back wall, to see where each kind's pooled light stands.
//! `ZZ_SHADOWS=1` makes the pool's lights cast shadows. `ZZ_LUX` is the sizing lux (default 1000).

mod common;

use bevy::camera::Exposure;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy_lights::*;
use common::{ExampleHarness, camera, camera_placement, wind_env};

#[derive(Component)]
struct Focus;

fn main() {
    let (eye, look) = camera_placement(Vec3::new(0.0, 1.7, 14.0), Vec3::new(0.0, 1.4, -3.0));
    let shadows = std::env::var("ZZ_SHADOWS").is_ok();
    let [r, g, b] = blackbody_colour(1900.0, 1.0);
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((ExampleHarness, FlickerPlugin, LightPoolPlugin::<Focus>::default(), FlamePlugin))
        .insert_resource(LightPool {
            size: 6,
            reach: 60.0,
            max_shadowed: Some(if shadows { 6 } else { 0 }),
            light: PointLight {
                color: LinearRgba::rgb(r, g, b).into(),
                range: 9.0 * RANGE_MARGIN,
                intensity: 0.0,
                shadow_maps_enabled: shadows,
                ..default()
            },
            ..default()
        })
        .insert_resource(ClearColor(Color::srgb(0.010, 0.014, 0.034)))
        .insert_resource(FlameLook { color: LinearRgba::rgb(r, g, b), strength: 0.82, max_height: 4.0 })
        .insert_resource(FlameWind { velocity: wind_env().unwrap_or_default() })
        .insert_resource(GlobalAmbientLight { color: Color::WHITE, brightness: 0.0, ..default() })
        .add_systems(Startup, move |commands: Commands, meshes: ResMut<Assets<Mesh>>, materials: ResMut<Assets<StandardMaterial>>| {
            setup(commands, meshes, materials, eye, look)
        })
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    eye: Vec3,
    look: Vec3,
) {
    let stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.42, 0.38),
        perceptual_roughness: 0.85,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(60.0, 40.0))),
        MeshMaterial3d(stone.clone()),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(60.0, 8.0, 0.5))),
        MeshMaterial3d(stone.clone()),
        Transform::from_xyz(0.0, 4.0, -3.0),
    ));
    let lux: f32 = std::env::var("ZZ_LUX").ok().and_then(|v| v.parse().ok()).unwrap_or(1000.0);
    for (i, kind) in FireKind::ALL.into_iter().enumerate() {
        let x = (i as f32 - 2.5) * 4.5;
        commands.spawn((
            kind.fire(lux, kind.height() * 0.5),
            Transform::from_xyz(x, 0.0, 0.0),
        ));
        // A hearth stone at each foot, so the flame is read as standing on something.
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(kind.diameter().max(0.3) + 0.2, 0.12, kind.diameter().max(0.3) + 0.2))),
            MeshMaterial3d(stone.clone()),
            Transform::from_xyz(x, -0.04, 0.0),
            NotShadowCaster,
        ));
    }
    commands.spawn((
        camera(eye, look, 8.0),
        Exposure { ev100: 6.0 },
    ));
    commands.spawn((Focus, Transform::from_xyz(0.0, 1.0, 4.0)));
}
