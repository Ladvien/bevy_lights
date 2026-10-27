//! Throwaway probe: what display value does a known linear radiance reach at this exposure?
mod common;
use bevy::camera::Exposure;
use bevy::prelude::*;
use std::f32::consts::TAU;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window { resolution: (960, 540).into(), ..default() }),
        ..default()
    }));
    app.add_systems(Startup, setup);
    app.run();
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.spawn((
        Camera3d::default(),
        Camera { clear_color: ClearColorConfig::Custom(Color::BLACK), ..default() },
        Exposure { ev100: 6.0 },
        Transform::from_xyz(0.0, 0.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    let quad = meshes.add(Rectangle::new(0.36, 0.9));
    // Columns: linear radiance values. Rows: unlit (base_color) / emissive (with exposure).
    let values = [0.0f32, 0.01, 0.03, 0.1, 0.3, 1.0, 3.0, 9.0, 30.0, 100.0, 300.0, 1000.0];
    for (i, v) in values.iter().enumerate() {
        let x = -2.0 + i as f32 * 0.36;
        let c = LinearRgba::rgb(*v, *v, *v);
        // top row: unlit base_color
        commands.spawn((
            Mesh3d(quad.clone()),
            MeshMaterial3d(materials.add(StandardMaterial { unlit: true, base_color: c.into(), ..default() })),
            Transform::from_xyz(x, 0.5, 0.0),
        ));
        // bottom row: emissive, black albedo
        commands.spawn((
            Mesh3d(quad.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: LinearRgba::rgb(0.01, 0.01, 0.01).into(),
                emissive: c,
                ..default()
            })),
            Transform::from_xyz(x, -0.5, 0.0),
        ));
    }
    let _ = TAU;
}
