//! The showcase: four braziers, 48 torch-bearers, and a pool of eight real lights between them.
//!
//! A 40 × 40 m courtyard at night, in a metre world. Twelve pillars in two rows give the light
//! something to fall on and the pool something to hide behind; a moon fills the shadows cool and
//! dim; and the fires are the only warm light, which is the composition every night scene lives or
//! dies on.
//!
//! # What to look for
//!
//! - **The four braziers** (`Brazier`): each a box of coals with a shadow-casting `PointLight` at
//!   +0.9 m and a [`FlameSite`] on its rim. They all start with [`Kindle::default()`], so the first
//!   0.6 s is the whole courtyard catching at once; the `kindled` lines on stdout are the four
//!   [`Kindled`] messages.
//! - **The 48 torch-bearers** (`Bearer`): capsules walking circles of 3–15 m, each with a
//!   [`PoolCandidate`] at +1.8 m and a flame at the same place. Only the eight nearest the focus are
//!   real lights; the other forty are a flame and nothing else. Walk the camera and watch the pool
//!   hand its lights over.
//! - **The focus** (`Focus`): orbits at 6 m radius on a 20 s period, so a sixty-second capture sees
//!   the pool travel the whole courtyard.
//!
//! # Keys
//!
//! `WASD` + `Q`/`E` fly (Shift ×5), hold the right mouse button to look, and `K` re-lights every
//! brazier (re-inserting [`Kindle::default()`]), which is the fastest way to see the flare and hear
//! the `kindled` lines twice.
//!
//! # Knobs
//!
//! `LIGHTS_FRAMES`, `LIGHTS_SHOT`, `LIGHTS_SHOT_DIR`/`LIGHTS_SHOT_EVERY`/`LIGHTS_SHOT_FROM`,
//! `LIGHTS_EYE`, `LIGHTS_LOOK`, `LIGHTS_SEED` and `LIGHTS_POOL` come from `examples/common/mod.rs`.
//! The default camera is the eye at `(0, 9, 16)` looking at the origin.
//!
//! ```sh
//! LIGHTS_FRAMES=600 LIGHTS_SHOT=/tmp/court.png cargo run --release --example courtyard
//! ```

mod common;

use std::f32::consts::TAU;

use bevy::camera::Exposure;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use bevy_lights::*;

use common::{Draw, ExampleHarness, camera, camera_placement, seed};

/// The moon's illuminance, in lux: a full moon overhead is about 0.25 lx, so 0.3 is a cool key you
/// barely see and the braziers are the scene's only strong light. At `ev100` 3.0 a surface needs tens
/// of lux to read; the moon (a design default) is there for the cool tint it gives the shadow side,
/// not for the exposure.
const NIGHT_MOON_LUX: f32 = 0.3;
/// How many torch-bearers walk the courtyard.
const BEARERS: usize = 48;
/// A brazier's light and its range. The box is 0.7 m tall; its flame stands on the coals at the top,
/// 0.9 m tall, and the light sits at the flame's centre (`BRAZIER_LIGHT`, metres over the ground), not
/// at the rim: a light a hand's width over a box lights the ground around it from inside the box's
/// own shadow, which reads as a dark square under every brazier.
const BRAZIER_LUMENS: f32 = 60_000.0;
const BRAZIER_RANGE: f32 = 12.0;
const BRAZIER_TOP: f32 = 0.7;
const BRAZIER_FLAME_HEIGHT: f32 = 0.9;
const BRAZIER_LIGHT: f32 = BRAZIER_TOP + BRAZIER_FLAME_HEIGHT * 0.5;
/// A fire's colour: a flame is warm, about 1900-2200 K, which is the warm half of the scene's
/// warm-against-cool separation (the moon fill is the cool half). Design defaults.
const BRAZIER_COLOUR: Color = Color::srgb(1.0, 0.60, 0.30);
const TORCH_COLOUR: Color = Color::srgb(1.0, 0.70, 0.40);
/// A carried torch's light and the height it sits over the bearer's feet.
const TORCH_LUMENS: f32 = 8_000.0;
const TORCH_LIFT: f32 = 1.8;
/// The brazier boxes' places: the courtyard's four inner corners, 5 m out from the centre.
const BRAZIERS: [Vec3; 4] = [
    Vec3::new(5.0, 0.0, 5.0),
    Vec3::new(-5.0, 0.0, 5.0),
    Vec3::new(-5.0, 0.0, -5.0),
    Vec3::new(5.0, 0.0, -5.0),
];

fn main() {
    let (eye, look) = camera_placement(Vec3::new(0.0, 9.0, 16.0), Vec3::ZERO);
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((ExampleHarness, FlickerPlugin, LightPoolPlugin::<Focus>::default(), FlamePlugin))
        .insert_resource(CameraPlacement(eye, look))
        // The pool is inserted before the plugin's `init_resource`, so these numbers win: eight real
        // lights over a 20 m reach, each an 8 m point light with no shadow (the braziers are the
        // shadow casters).
        .insert_resource(LightPool {
            size: std::env::var("LIGHTS_POOL").ok().and_then(|v| v.parse().ok()).unwrap_or(8),
            reach: 20.0,
            light: PointLight { color: TORCH_COLOUR, range: 8.0, intensity: 0.0, ..default() },
            ..default()
        })
        // A cool near-black behind the courtyard's far edge, instead of the default flat grey.
        .insert_resource(ClearColor(Color::srgb(0.010, 0.014, 0.034)))
        .insert_resource(GlobalAmbientLight { color: Color::WHITE, brightness: 0.0, ..default() })
        .add_systems(Startup, (spawn_scene, spawn_camera).chain())
        .add_systems(Update, (walk_bearers, orbit_focus, relight_braziers).chain())
        .add_systems(Update, report_kindled)
        .run();
}

/// The one entity the pool measures distance from: an invisible point orbiting the courtyard, which
/// stands in for a camera that a real game would drive.
#[derive(Component)]
struct Focus;

/// A brazier: the coals, their light and their flame.
#[derive(Component, Clone, Copy)]
struct Brazier;

/// A torch-bearer: the figure and its carried fire.
#[derive(Component)]
struct Bearer {
    /// The circle it walks: its centre, its radius and its angular speed.
    centre: Vec3,
    radius: f32,
    rate: f32,
    angle: f32,
}

fn spawn_camera(mut commands: Commands, placement: Res<CameraPlacement>) {
    let CameraPlacement(eye, look) = *placement;
    commands.spawn((
        camera(eye, look, 8.0),
        // A cool fill the moon does not have to provide: without it the courtyard's pillars are
        // black silhouettes and the fire has nothing to read against.
        AmbientLight { color: Color::srgb(0.55, 0.65, 1.0), brightness: 2.0, ..default() },
        Exposure { ev100: 3.0 },
    ));
}

/// Where the camera goes, read once at startup so the systems do not read the environment again.
#[derive(Resource, Clone, Copy)]
struct CameraPlacement(Vec3, Vec3);

/// A grey stone the fire reads warm against.
fn stone(colour: Srgba) -> StandardMaterial {
    StandardMaterial { base_color: colour.into(), perceptual_roughness: 0.9, ..default() }
}

fn spawn_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let stone_material = materials.add(stone(Srgba::rgb(0.32, 0.30, 0.28)));
    let bowl_material = materials.add(stone(Srgba::rgb(0.10, 0.09, 0.09)));
    let coal_material = materials.add(StandardMaterial {
        base_color: Srgba::rgb(0.14, 0.10, 0.09).into(),
        emissive: LinearRgba::rgb(3.0, 0.6, 0.10),
        ..default()
    });
    let coal_bed = meshes.add(Cuboid::new(0.66, 0.08, 0.66));

    // The ground: one plane, so the pool's falloff has somewhere to read.
    commands.spawn((
        Name::new("Ground"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(40.0, 40.0))),
        MeshMaterial3d(stone_material.clone()),
    ));

    // Two rows of six pillars, 6 m apart, 6 m either side of the axis: the light falls on their
    // faces and the shadow-casting braziers throw them long.
    for row in 0..2 {
        let z = if row == 0 { -6.0 } else { 6.0 };
        for column in 0..6 {
            let x = -12.5 + column as f32 * 5.0;
            commands.spawn((
                Name::new(format!("Pillar {row}/{column}")),
                Mesh3d(meshes.add(Cuboid::new(0.6, 3.0, 0.6))),
                MeshMaterial3d(stone_material.clone()),
                Transform::from_xyz(x, 1.5, z),
            ));
        }
    }

    // The moon: a dim cool key from behind the camera's left shoulder. A directional light is the
    // host's, not the crate's (see the crate docs' ownership note); the crate never adds one.
    commands.spawn((
        Name::new("Moon"),
        DirectionalLight {
            color: Color::srgb(0.62, 0.72, 1.0),
            illuminance: NIGHT_MOON_LUX,
            // 0.3 lx throws a shadow nobody can see, so it is not worth a shadow map.
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -35f32.to_radians(),
            -40f32.to_radians(),
            0.0,
        )),
    ));

    // The four braziers.
    for (index, at) in BRAZIERS.iter().enumerate() {
        let phase = index as f32 * 1.9;
        commands
            .spawn((
                Name::new(format!("Brazier {index}")),
                Brazier,
                Mesh3d(meshes.add(Cuboid::new(0.8, BRAZIER_TOP, 0.8))),
                MeshMaterial3d(bowl_material.clone()),
                // The coals sit in the bowl and the light is above them: the bowl does not shade its
                // own fire. Cast, it throws a hard dark square across the ground around every brazier.
                NotShadowCaster,
                Transform::from_translation(*at + Vec3::Y * BRAZIER_TOP * 0.5),
            ))
            .with_children(|brazier| {
                // The source the light is motivated by: a bed of glowing coals on the box's top.
                brazier.spawn((
                    Name::new("Coal bed"),
                    Mesh3d(coal_bed.clone()),
                    MeshMaterial3d(coal_material.clone()),
                    Transform::from_xyz(0.0, BRAZIER_TOP * 0.5, 0.0),
                    NotShadowCaster,
                ));
                brazier.spawn((
                    Name::new("Coals"),
                    PointLight {
                        color: BRAZIER_COLOUR,
                        intensity: BRAZIER_LUMENS,
                        range: BRAZIER_RANGE,
                        shadow_maps_enabled: true,
                        ..default()
                    },
                    Flicker { base: BRAZIER_LUMENS, phase },
                    Kindle::default(),
                    Transform::from_xyz(0.0, BRAZIER_LIGHT - BRAZIER_TOP * 0.5, 0.0),
                ));
                brazier.spawn((
                    Name::new("Flame site"),
                    FlameSite { height: BRAZIER_FLAME_HEIGHT, phase },
                    // The flame's foot is on the coals, at the top of the box.
                    Transform::from_xyz(0.0, BRAZIER_TOP * 0.5 + 0.04, 0.0),
                ));
            });
    }

    // The torch-bearers: capsules on circles, each with its own fire.
    let mut draw = Draw::new(seed());
    let body = meshes.add(Capsule3d::new(0.15, 1.4));
    let cloth = materials.add(stone(Srgba::rgb(0.20, 0.19, 0.22)));
    let head = meshes.add(Sphere::new(0.11));
    let skin = materials.add(stone(Srgba::rgb(0.42, 0.33, 0.28)));
    for index in 0..BEARERS {
        let radius = draw.range(3.0, 15.0);
        let centre = Vec3::new(0.0, 0.0, 0.0);
        let angle = draw.range(0.0, TAU);
        let rate = draw.range(0.05, 0.16) * if index.is_multiple_of(2) { 1.0 } else { -1.0 };
        let phase = draw.range(0.0, TAU);
        commands
            .spawn((
                Name::new(format!("Bearer {index}")),
                Bearer { centre, radius, rate, angle },
                Transform::from_translation(centre + Vec3::new(radius, 0.85, 0.0)),
                // The root of a hierarchy of meshes needs a `Visibility`, or Bevy warns (B0004).
                Visibility::default(),
            ))
            .with_children(|bearer| {
                bearer.spawn((
                    Mesh3d(body.clone()),
                    MeshMaterial3d(cloth.clone()),
                    NotShadowCaster,
                    NotShadowReceiver,
                ));
                bearer.spawn((
                    Mesh3d(head.clone()),
                    MeshMaterial3d(skin.clone()),
                    Transform::from_xyz(0.0, 1.05, 0.0),
                    NotShadowCaster,
                    NotShadowReceiver,
                ));
                // The carried fire: a torch's light where a real light is handed out, a flame where
                // it is not. Both sit at the same lift, so the hand-off is invisible.
                bearer.spawn((
                    Name::new("Torch light"),
                    PoolCandidate { offset: Vec3::Y * TORCH_LIFT, intensity: TORCH_LUMENS },
                    Transform::from_xyz(0.0, TORCH_LIFT - 0.85, 0.0),
                ));
                bearer.spawn((
                    Name::new("Torch flame"),
                    FlameSite { height: 0.35, phase },
                    Transform::from_xyz(0.0, TORCH_LIFT - 0.85, 0.0),
                ));
            });
    }

    // The focus the pool measures from.
    commands.spawn((Name::new("Focus"), Focus, Transform::default()));
}

fn walk_bearers(time: Res<Time>, mut bearers: Query<(&mut Bearer, &mut Transform)>) {
    let dt = time.delta_secs();
    for (mut bearer, mut transform) in &mut bearers {
        bearer.angle += bearer.rate * dt;
        let at = bearer.centre
            + Vec3::new(bearer.radius * bearer.angle.cos(), 0.85, bearer.radius * bearer.angle.sin());
        transform.translation = at;
        // Facing along the circle: the capsule's forward is -Z, so the tangent is the heading.
        let heading = Vec3::new(-bearer.angle.sin(), 0.0, bearer.angle.cos()) * bearer.rate.signum();
        transform.rotation = Quat::from_rotation_y(f32::atan2(-heading.x, -heading.z));
    }
}

/// Orbits the focus at 6 m on a 20 s period.
fn orbit_focus(time: Res<Time>, mut focus: Query<&mut Transform, With<Focus>>) {
    let elapsed = time.elapsed_secs();
    let at = Vec3::new(6.0 * (elapsed * TAU / 20.0).cos(), 1.2, 6.0 * (elapsed * TAU / 20.0).sin());
    for mut transform in &mut focus {
        transform.translation = at;
    }
}

/// `K` re-lights every brazier: re-inserting [`Kindle::default()`] restarts the flare, and the
/// message is written again.
fn relight_braziers(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    braziers: Query<&Children, With<Brazier>>,
    lights: Query<(), With<Kindle>>,
) {
    if !keys.just_pressed(KeyCode::KeyK) {
        return;
    }
    for children in &braziers {
        for child in children.iter() {
            if lights.contains(child) {
                commands.entity(child).insert(Kindle::default());
            }
        }
    }
}

/// Prints every [`Kindled`]: four lines when the courtyard catches, four more on every `K`.
fn report_kindled(mut kindled: MessageReader<Kindled>, names: Query<&Name>) {
    for message in kindled.read() {
        match names.get(message.entity) {
            Ok(name) => println!("kindled {} ({})", message.entity, name),
            Err(_) => println!("kindled {}", message.entity),
        }
    }
}
