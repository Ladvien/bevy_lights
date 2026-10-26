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

/// How many torch-bearers walk the courtyard.
const BEARERS: usize = 48;

// ---- The look, as a cinematographer sets it: a temperature, a pool, and the ratio between the fire
// ---- and the sky. Every lumen below is derived from these, not hand-picked. The numbers and their
// ---- sources are in `docs/research/lighting-look.md`.

/// The temperature a wood fire burns at, in kelvin: the warm half of the scene, and the colour every
/// fire's light and flame is taken from.
const FIRE_KELVIN: f32 = 1900.0;
/// The temperature the camera is balanced to: tungsten, 2856 K (CIE Illuminant A), the standing
/// convention for a firelit night. Against it a fire at 1900 K reads orange and the sky at 8000 K
/// reads blue, so the scene's warm/cool split comes from two temperatures and one balance rather
/// than from two hand-picked colours.
const BALANCE_KELVIN: f32 = 2856.0;
/// The temperature of the night sky's fill: the cool half. A moon is reflected sunlight, and against a
/// fire balance it is far enough away in colour to read blue.
const SKY_KELVIN: f32 = 8000.0;
/// The camera's exposure for a firelit night: Bevy's exposure is `2^-ev100 / 1.2`, and a Lambertian
/// surface of albedo 0.3 lit to `E` lux has luminance `0.3·E/π`. At `ev100` 6.0, a fire's pool at
/// 1000 lx is linear 1.24 (a little over white) and 250 lx is 0.31: the fire is the frame's key and
/// the sky, at a quarter of it, is two stops under. `ev100` 6 is a night interior/exterior stop.
const NIGHT_EV100: f32 = 6.0;
/// What a fire puts on the ground at the centre of its pool, in lux. The pool is sized so that this
/// reads a little over middle grey at [`NIGHT_EV100`] and its edge, [`POOL_LUX`], is the last of the
/// ground the fire owns.
const POOL_LUX: f32 = 1000.0;
/// The illuminance at the *edge* of a fire's pool: a quarter of the centre, so the edge is two stops
/// down — the shoulder `lighting-look.md` asks for.
const POOL_EDGE_LUX: f32 = 250.0;
/// How far a brazier's pool reaches, in metres: the ground it lights to [`POOL_LUX`].
const BRAZIER_POOL: f32 = 4.0;
/// How far a carried torch's pool reaches. Smaller than a brazier's: a 0.35 m flame lighting a 4 m
/// patch is the "flame too small for the ground it lights" defect `lighting-look.md` names.
const TORCH_POOL: f32 = 2.0;
/// The key-to-fill ratio the courtyard aims at: two to one, one stop. A fire's pool centre is the key
/// and the moon is the fill; at 4:1 the moonlight left figures between pools at 3-9 DN, darker than
/// the ground they stood on, which is worse than flat. One stop keeps a shadow side and keeps the
/// courtyard readable away from the fires — the high-key end of the craft's scale, chosen for the
/// reader rather than for the photograph.
const COURT_RATIO: f32 = HIGH_KEY;

/// The box's height, the flame on its coals, and the light at the flame's centre rather than at the
/// rim: a light a hand's width over a box lights the ground from inside the box's own shadow, which
/// reads as a dark square under every brazier.
const BRAZIER_TOP: f32 = 0.7;
const BRAZIER_FLAME_HEIGHT: f32 = 1.1;
const BRAZIER_LIGHT: f32 = BRAZIER_TOP + BRAZIER_FLAME_HEIGHT * 0.5;
/// How far past its pool a fire's light is still computed: the windowed inverse square has faded it
/// out by here, so the falloff reads as a shoulder and never as a disc.
const POOL_RANGE_MARGIN: f32 = 2.5;
/// A carried torch's light sits this high over the bearer's feet.
const TORCH_LIFT: f32 = 1.8;

/// The lumens that put [`POOL_LUX`] on the ground `pool` metres out, `over` metres below the light.
fn lumens_for(pool: f32, over: f32) -> f32 {
    fire_lumens(POOL_LUX, pool, over)
}

/// The lumens a **hand-held** fire needs: a torch's flame is a tenth of a brazier's coal bed, so its
/// pool is sized from its edge ([`POOL_EDGE_LUX`]) rather than its centre. Its centre is then four
/// times that, which at this exposure is bright but not a searchlight — asking for a brazier's centre
/// brightness from a 0.35 m flame needed 61 607 lm and blew the stone white.
fn torch_lumens(pool: f32, over: f32) -> f32 {
    fire_lumens(POOL_EDGE_LUX, pool, over)
}

/// The fire's colour as linear RGB, from the temperature it burns at.
fn fire_temperature() -> LinearRgba {
    let [r, g, b] = blackbody_colour(FIRE_KELVIN, 1.0);
    LinearRgba::rgb(r, g, b)
}

/// The fire's colour, from the temperature it burns at.
fn fire_colour() -> Color {
    let [r, g, b] = blackbody_colour(FIRE_KELVIN, 1.0);
    LinearRgba::rgb(r, g, b).into()
}

/// The night fill's colour: the sky at its temperature, as the camera balanced to the fire records it
/// (so it is blue).
fn sky_colour() -> Color {
    let [r, g, b] = white_balanced(SKY_KELVIN, BALANCE_KELVIN, 1.0);
    LinearRgba::rgb(r, g, b).into()
}
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
            light: PointLight {
                color: fire_colour(),
                // The same windowed falloff the fires have: the pool light is a torch's fire, one
                // metre of it is a real light rather than a flame.
                range: TORCH_POOL * POOL_RANGE_MARGIN,
                intensity: 0.0,
                ..default()
            },
            ..default()
        })
        // A cool near-black behind the courtyard's far edge, instead of the default flat grey.
        .insert_resource(ClearColor(Color::srgb(0.010, 0.014, 0.034)))
        // The night's fill, as a cinematographer sets it: the sky's colour, at the illuminance that
        // makes COURT_RATIO against the fire. It is the host's light (this crate never adds one), and
        // the ratio is the crate's number.
        .insert_resource(NightFill { illuminance: fill_for(POOL_LUX, COURT_RATIO) })
        // The flames' colour comes from the same temperature as their light, and `max_height` is set
        // once here, before any site spawns (the height is packed as a fraction of it).
        .insert_resource(FlameLook { color: fire_temperature(), strength: 0.82, max_height: 4.0 })
        .insert_resource(GlobalAmbientLight { color: Color::WHITE, brightness: 0.0, ..default() })
        .add_systems(Startup, (spawn_scene, spawn_camera).chain())
        .add_systems(Update, (walk_bearers, orbit_focus, relight_braziers).chain())
        .add_systems(Update, report_kindled)
        .add_systems(Update, report_look)
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

fn spawn_camera(mut commands: Commands, placement: Res<CameraPlacement>, fill: Res<NightFill>) {
    let CameraPlacement(eye, look) = *placement;
    commands.spawn((
        camera(eye, look, 8.0),
        // A cool fill the moon does not have to provide: without it the courtyard's pillars are
        // black silhouettes and the fire has nothing to read against. Its colour is the sky's
        // temperature, not a hand-picked blue.
        AmbientLight { color: sky_colour(), brightness: 2.0, ..default() },
        Exposure { ev100: NIGHT_EV100 },
    ));
    // The moon: the host's directional light (the crate never adds one), a sky-coloured fill at the
    // illuminance that makes the target key-to-fill against the braziers' pools.
    commands.spawn((
        Name::new("Moon fill"),
        DirectionalLight {
            color: sky_colour(),
            illuminance: fill.illuminance,
            // A fill two stops under the fire throws no shadow worth a shadow map.
            shadow_maps_enabled: false,
            ..default()
        },
        // From behind the camera's shoulder and high: a fill's job is to keep the shadow side
        // readable, and a fill that rakes from the side turns the whole frame into a warm wash (the
        // baseline's fault). `lighting-look.md`'s separation comes from the two colours, not from the
        // fill's direction.
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -55f32.to_radians(),
            -40f32.to_radians(),
            0.0,
        )),
    ));
}

/// Where the camera goes, read once at startup so the systems do not read the environment again.
#[derive(Resource, Clone, Copy)]
struct CameraPlacement(Vec3, Vec3);

/// The night sky's fill: an illuminance the host's moon provides, derived from the courtyard's target
/// ratio against its fires rather than picked by eye.
#[derive(Resource, Clone, Copy)]
struct NightFill {
    illuminance: f32,
}

/// A grey stone the fire reads warm against.
fn stone(colour: Srgba) -> StandardMaterial {
    StandardMaterial { base_color: colour.into(), perceptual_roughness: 0.9, ..default() }
}

/// The same stone, polished: a floor with a specular answer to a fire, so the frame says where the
/// fire is even in the ground's highlight. `lighting-look.md`'s "specular answer" line.
fn polished_stone(colour: Srgba) -> StandardMaterial {
    StandardMaterial { base_color: colour.into(), perceptual_roughness: 0.55, reflectance: 0.06, ..default() }
}

fn spawn_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let stone_material = materials.add(stone(Srgba::rgb(0.32, 0.30, 0.28)));
    let bowl_material = materials.add(stone(Srgba::rgb(0.10, 0.09, 0.09)));
    // The coals glow at the fire's own temperature, brighter than the stone can reflect: the source
    // the light is motivated by, and the same colour as the flame on it.
    let [cr, cg, cb] = blackbody_colour(FIRE_KELVIN, 1.0);
    // The coals are hot, not white-hot: at this exposure 1.4 is the brightest thing in the frame that
    // is not the flame, and 3.0 clipped to a white slab with no shape in it.
    let coal_material = materials.add(StandardMaterial {
        base_color: Srgba::rgb(0.14, 0.10, 0.09).into(),
        emissive: LinearRgba::rgb(cr * 1.4, cg * 1.4, cb * 1.4),
        ..default()
    });
    let coal_bed = meshes.add(Cuboid::new(0.34, 0.05, 0.34));

    // The ground: one plane, so the pool's falloff has somewhere to read, and polished enough that a
    // fire puts a highlight on it rather than a flat wash.
    let ground_material = materials.add(polished_stone(Srgba::rgb(0.30, 0.28, 0.26)));
    commands.spawn((
        Name::new("Ground"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(40.0, 40.0))),
        MeshMaterial3d(ground_material),
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
                // The fire's size, from its pool: POOL_LUX at BRAZIER_POOL metres out, from a light
                // BRAZIER_LIGHT over the ground. 60 000 lm was 20x this and blew the pool it lit.
                let lumens = lumens_for(BRAZIER_POOL, BRAZIER_LIGHT);
                brazier.spawn((
                    Name::new("Coals"),
                    PointLight {
                        color: fire_colour(),
                        intensity: lumens,
                        range: BRAZIER_POOL * POOL_RANGE_MARGIN,
                        shadow_maps_enabled: true,
                        ..default()
                    },
                    Flicker { base: lumens, phase },
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
                    PoolCandidate {
                        offset: Vec3::Y * TORCH_LIFT,
                        intensity: torch_lumens(TORCH_POOL, TORCH_LIFT - 0.85),
                    },
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

/// Prints the look's numbers once, a second in: what the fires were sized to, and the key-to-fill the
/// frame is built on. A capture can be checked against this line.
fn report_look(fill: Res<NightFill>, mut said: Local<bool>) {
    if *said {
        return;
    }
    *said = true;
    let brazier_lumens = lumens_for(BRAZIER_POOL, BRAZIER_LIGHT);
    let torch_fire = torch_lumens(TORCH_POOL, TORCH_LIFT - 0.85);
    println!(
        "look: fire {FIRE_KELVIN:.0} K, sky {SKY_KELVIN:.0} K, balanced {BALANCE_KELVIN:.0} K, \
pool {POOL_LUX:.0} lx (edge {POOL_EDGE_LUX:.0}) at {brazier_lumens:.0} lm (brazier, {BRAZIER_POOL:.1} m) \
and {torch_fire:.0} lm (torch, {TORCH_POOL:.1} m), moon {:.1} lx, key-to-fill {:.2}:1 ({:.2} stops), \
at the pool's edge {:.2}:1",
        fill.illuminance,
        key_to_fill(POOL_LUX, fill.illuminance),
        stops(key_to_fill(POOL_LUX, fill.illuminance)),
        key_to_fill(POOL_EDGE_LUX, fill.illuminance),
    );
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
