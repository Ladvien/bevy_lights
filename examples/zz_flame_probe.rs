//! Throwaway loop harness: flames alone on black, at a fixed camera, so the silhouette can be judged
//! and measured without a scene behind it. Deleted before the commit.

mod common;

use bevy::camera::Exposure;
use bevy::prelude::*;
use bevy_lights::*;

use common::ExampleHarness;

fn main() {
    let (eye, look) = common::camera_placement(Vec3::new(0.0, 2.2, 5.2), Vec3::new(0.0, 1.9, 0.0));
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((ExampleHarness, FlamePlugin))
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(GlobalAmbientLight { color: Color::WHITE, brightness: 0.0, ..default() })
        .insert_resource(FlameLook {
            color: LinearRgba::rgb(1.0, 0.55, 0.18),
            strength: 0.82,
            max_height: 4.0,
        })
        .add_systems(Startup, move |mut commands: Commands| {
            commands.spawn((Camera3d::default(), Exposure { ev100: 6.0 }, camera_pose(eye, look)));
            // A brazier's flame (1.1 m, bed 0.34 m) and a torch's (0.35 m, un-sized), side by side and
            // the same distance from the camera, so their shapes can be compared at one scale.
            for (name, x, height, bed) in
                [("brazier", -1.4, 1.1, Some(0.34)), ("torch", 1.4, 0.35, None)]
            {
                commands.spawn((
                    Name::new(name),
                    FlameSite { height, phase: 0.0, kindle: 0.0, diameter: bed },
                    Transform::from_xyz(x, 1.35, 0.0),
                ));
            }
        })
        .run();
}

fn camera_pose(eye: Vec3, look_at: Vec3) -> Transform {
    let dir = (look_at - eye).normalize_or(Vec3::NEG_Z);
    let yaw = (-dir.x).atan2(-dir.z);
    let pitch = dir.y.clamp(-1.0, 1.0).asin();
    Transform::from_translation(eye).with_rotation(Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0))
}
