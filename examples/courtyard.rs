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
//! `LIGHTS_RANK=distance` (rank by distance alone, the old rule), `LIGHTS_REACH`, `LIGHTS_FOCUS_WALK` (the focus's orbit radius; past the pool's reach the torches
//! stream across the boundary, which is what makes a candidate leave the pool) and `LIGHTS_POOL_LOG`
//! (the pool's total every frame, the only way to see a hand-off) are also this example's own.
//!
//! Six more are this example's own: `LIGHTS_RESERVE` (reserve lights for a cross-faded hand-off, default
//! 1; `0` dips instead), `LIGHTS_HYSTERESIS` (the pool's overlap ratio, default 1.15),
//! `LIGHTS_FADE` (seconds a light takes to come back after a hand-off, default 0.35),
//! `LIGHTS_BED` (the braziers' fuel-bed diameter in metres, default 0.34, which sizes their flicker
//! through the puffing law) and `LIGHTS_UNSIZED=1` (ignore the bed and use the crate's inherited
//! rate, to see the difference). The first two set to the old behaviour (`1.0`, `0.0`) show the pop
//! the crate used to have, which is `docs/research/pool-hand-off.md`'s before/after.
//! The default camera is the eye at `(0, 9, 16)` looking at the origin.
//!
//! ```sh
//! LIGHTS_FRAMES=600 LIGHTS_SHOT=/tmp/court.png cargo run --release --example courtyard
//! ```

mod common;

use std::f32::consts::TAU;

use bevy::asset::RenderAssetUsages;
use bevy::camera::Exposure;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
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
/// The diameter of a brazier's coal bed, in metres: the fuel bed the puffing law sizes the fire's
/// flicker by. A bigger bed breathes more slowly. `LIGHTS_BED` overrides it; `LIGHTS_UNSIZED=1` gives
/// the fires no size at all (the crate's inherited rate), which is the comparison this feature is.
const BRAZIER_BED: f32 = 0.34;

// ---- The fuel bed's char field: a bed of coals is a surface with a temperature field on it, not a
// ---- uniform emitter. `docs/research/fuel-bed.md` has the defect, the sources and the measurements.

/// The side of the generated char field, in texels: the bed is 0.34 m and the nearest camera in the
/// showcase is 4 m away, so a texel is about 2.7 mm — far under a pixel at every captured angle.
const COAL_FIELD: u32 = 128;
/// The char field's seed: the example's own, so a capture's coals are reproducible like its figures.
const COAL_FIELD_SEED: u32 = 7;
/// The share of the bed's full emissive its **hottest** fuel carries. Above the 1.4 the flat bed used,
/// so giving the bed structure does not dim the source: a fire is supposed to be the brightest thing in
/// frame, and the fix is what is *under* the brightness, not less of it.
const COAL_FIELD_HIGH: f32 = 1.45;
/// The share of the bed's full emissive its **charred** part carries: charcoal still glows, but at a
/// third of the fuel, which is well under the point it clips.
const COAL_FIELD_LOW: f32 = 0.30;

/// The bed's full emissive, as a multiple of a blackbody at [`FIRE_KELVIN`] normalised to 1: the
/// brightest thing in the frame that is not the flame, times [`COAL_FIELD_HIGH`] on its hottest coals.
const COAL_EMISSIVE: f32 = 1.45;

/// The char field as an image a `StandardMaterial` can sample: [`char_field`] over the bed, mapped to a
/// share by [`char_share`].
///
/// One channel, because both of the bed's textures are the same field — `base_color_texture` and
/// `emissive_texture` sample the same image, so a charred texel is both darker *and* dimmer, which is
/// what char is. An `R8Unorm` image is a share in `0..=1` with no colour space in between, so the value
/// a caller passed in is the value the shader multiplies by.
fn char_field_image(seed: u32) -> Image {
    let mut data = Vec::with_capacity((COAL_FIELD * COAL_FIELD) as usize);
    for y in 0..COAL_FIELD {
        for x in 0..COAL_FIELD {
            let u = x as f32 / (COAL_FIELD - 1) as f32;
            let v = y as f32 / (COAL_FIELD - 1) as f32;
            let share = char_share(char_field(u, v, seed), COAL_FIELD_LOW, COAL_FIELD_HIGH);
            data.push((share * 255.0).round() as u8);
        }
    }
    Image::new(
        Extent3d { width: COAL_FIELD, height: COAL_FIELD, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::R8Unorm,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

/// The fuel bed the braziers are given, or nothing for the un-sized rate.
fn brazier_bed() -> Option<f32> {
    if std::env::var_os("LIGHTS_UNSIZED").is_some() {
        return None;
    }
    Some(std::env::var("LIGHTS_BED").ok().and_then(|v| v.parse().ok()).unwrap_or(BRAZIER_BED))
}
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
/// The bearer's own root: a 1.7 m capsule stands on the ground, so its centre is this high.
const BEARER_LIFT: f32 = 0.85;
/// Where the hand is, local to the bearer: out to its side and a little forward, so the fire clears the
/// figure's own capsule. A bearer is a 0.30 m capsule, so a flame held at its axis would be drawn inside
/// the body — the flame is a billboard whose widest point is a tenth of its height, and this clears it.
const TORCH_HAND: Vec3 = Vec3::new(0.42, 0.35, 0.16);
/// A carried torch's flame, and the bigger flame of a lantern.
const TORCH_FLAME_HEIGHT: f32 = 0.35;
const LANTERN_FLAME_HEIGHT: f32 = 0.5;

/// The world height of a carried fire's light over the ground: the hand's height plus the flame's own
/// centre.
///
/// One number, used twice — the light is *placed* by it and the fire is *sized* by it — because those
/// two drifting apart is a bug this example had: a torch sized for a light 0.95 m up and placed 1.8 m
/// higher delivers 72 lx where it was sized for 250, since `E = Φ/4πr²` is measured from where the light
/// actually is. `docs/research/carried-fire.md`.
fn carried_light_height(flame_height: f32) -> f32 {
    BEARER_LIFT + TORCH_HAND.y + flame_light_offset(flame_height)[1]
}

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
            // `LIGHTS_RANK=distance` ranks by distance alone, the rule before the pool weighed a light's
            // output; the default ranks by the illuminance each candidate puts on the subject.
            rank: match std::env::var("LIGHTS_RANK").ok().as_deref() {
                Some("distance") => Rank::Distance,
                _ => Rank::Illuminance,
            },
            // `LIGHTS_RESERVE=0` makes a hand-off dip rather than cross-fade, which is the comparison.
            reserve: std::env::var("LIGHTS_RESERVE").ok().and_then(|v| v.parse().ok()).unwrap_or(1),
            hysteresis: std::env::var("LIGHTS_HYSTERESIS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(HYSTERESIS),
            fade_seconds: std::env::var("LIGHTS_FADE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(FADE_SECONDS),
            reach: std::env::var("LIGHTS_REACH").ok().and_then(|v| v.parse().ok()).unwrap_or(20.0),
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
        .add_systems(Update, report_flicker)
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

/// The courtyard's floor: flagstones, each its own material, so a fire has something to put a
/// **placed** highlight on. A single 40 m plane at one roughness spreads a fire's mirror lobe until it
/// is indistinguishable from its diffuse falloff — measured on the frame before this change, the floor's
/// luminance had no local maximum at all (p99 136 against a max of 217). `docs/research/lighting-look.md`
/// has the craft: Doyle's "specular lighting … concentrate and highlight the specific area".
///
/// The stones' roughnesses are drawn from the example's seeded stream and span `STONE_ROUGHNESS`, so some
/// answer a fire sharply and their neighbours stay matte; a floor of identical mirrors reads as a wet car
/// park.
const FLAG_SIZE: f32 = 2.5;
/// The gap between flagstones, in metres: the mortar, darker and rougher, showing through.
const FLAG_GAP: f32 = 0.06;
/// The range a flagstone's roughness is drawn from.
const STONE_ROUGHNESS: (f32, f32) = (0.18, 0.32);

fn spawn_flagstones(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let stone = meshes.add(Plane3d::default().mesh().size(FLAG_SIZE - FLAG_GAP, FLAG_SIZE - FLAG_GAP));
    let mortar = materials.add(StandardMaterial {
        base_color: Srgba::rgb(0.14, 0.13, 0.12).into(),
        perceptual_roughness: 0.85,
        ..default()
    });
    // The mortar: one plane under the stones, so the gaps read as joints rather than as holes.
    commands.spawn((
        Name::new("Ground"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(40.0, 40.0))),
        MeshMaterial3d(mortar),
    ));

    const COUNT: i32 = 40 / FLAG_SIZE as i32;
    let mut draw = Draw::new(seed() ^ 0x5eed);
    for row in 0..COUNT {
        for column in 0..COUNT {
            let x = -20.0 + FLAG_SIZE * (column as f32 + 0.5);
            let z = -20.0 + FLAG_SIZE * (row as f32 + 0.5);
            let roughness = draw.range(STONE_ROUGHNESS.0, STONE_ROUGHNESS.1);
            let colour = materials.add(StandardMaterial {
                base_color: Srgba::rgb(
                    draw.range(0.27, 0.33),
                    draw.range(0.25, 0.31),
                    draw.range(0.24, 0.29),
                )
                .into(),
                perceptual_roughness: roughness,
                reflectance: 0.06,
                ..default()
            });
            commands.spawn((
                Name::new(format!("Flagstone {row}/{column}")),
                Mesh3d(stone.clone()),
                MeshMaterial3d(colour),
                Transform::from_xyz(x, 0.004, z),
                NotShadowCaster,
            ));
        }
    }
}

fn spawn_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let stone_material = materials.add(stone(Srgba::rgb(0.32, 0.30, 0.28)));
    let bowl_material = materials.add(stone(Srgba::rgb(0.10, 0.09, 0.09)));
    // The coals glow at the fire's own temperature, brighter than the stone can reflect: the source
    // the light is motivated by, and the same colour as the flame on it.
    let [cr, cg, cb] = blackbody_colour(FIRE_KELVIN, 1.0);
    // A bed of coals is a *surface with a temperature field on it*, not a uniform emitter: burnt-through
    // char is dark and glowing fuel is bright, interleaved at the scale of a coal. A constant emissive
    // was the brightest thing in the frame at every camera and clipped to a flat white slab from above,
    // where it also hid the flame standing in it — `docs/research/fuel-bed.md` has the measurements.
    //
    // The field is generated here rather than shipped as an image: it is three octaves of the same
    // value noise the flame shader uses, seeded from the run, so a capture is reproducible and the
    // example stays one binary. It drives the bed's *albedo* as well as its emissive, because both of
    // the bed's brightnesses are wrong when it is flat: char absorbs the fire's own light instead of
    // reflecting it, and the diffuse term is most of what clips from overhead.
    let bed_field = images.add(char_field_image(COAL_FIELD_SEED));
    let coal_material = materials.add(StandardMaterial {
        // Char, not coal: the darkest part of the field, and the part that reflects least.
        base_color: Srgba::rgb(0.14, 0.10, 0.09).into(),
        base_color_texture: Some(bed_field.clone()),
        // The hottest fuel, at this exposure the brightest thing in frame that is not the flame. The
        // field scales it per texel, so the bed's mean sits under the clip point and its hottest coals
        // do not.
        emissive: LinearRgba::rgb(cr * COAL_EMISSIVE, cg * COAL_EMISSIVE, cb * COAL_EMISSIVE),
        emissive_texture: Some(bed_field),
        ..default()
    });
    let coal_bed = meshes.add(Cuboid::new(0.34, 0.05, 0.34));

    // The ground: one plane, so the pool's falloff has somewhere to read, and polished enough that a
    // fire puts a highlight on it rather than a flat wash.
    spawn_flagstones(&mut commands, &mut meshes, &mut materials);

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
                    Flicker { base: lumens, phase, diameter: brazier_bed() },
                    Kindle::default(),
                    Transform::from_xyz(0.0, BRAZIER_LIGHT - BRAZIER_TOP * 0.5, 0.0),
                ));
                brazier.spawn((
                    Name::new("Flame site"),
                    // The brazier's light is spawned with `Kindle::default()`, so its flame catches with it
                    // over the same envelope: the fire grows rather than appearing whole.
                    FlameSite {
                        height: BRAZIER_FLAME_HEIGHT,
                        phase,
                        kindle: KINDLE_SECONDS,
                        diameter: brazier_bed(),
                    },
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
        // One bearer in six carries a **lantern** rather than a torch: a bigger flame and four times the
        // light. That is what makes the pool's ranking visible — the same distance, very different
        // output — and it is the case the old distance-only rule got wrong.
        let lantern = index % 6 == 0;
        let burn = if lantern { 4.0 } else { 1.0 };
        commands
            .spawn((
                Name::new(format!("Bearer {index}")),
                Bearer { centre, radius, rate, angle },
                Transform::from_translation(centre + Vec3::new(radius, BEARER_LIFT, 0.0)),
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
                // The carried fire: one entity, so the flame and the real light it is handed can never
                // drift apart. The light sits at the flame's own centre
                // (`flame_light_offset`), which is the only place a point light approximates a fire of
                // this height without leaving the drawn flame — and the height the light is sized for is
                // read off that same geometry, not typed a second time.
                let flame_height = if lantern { LANTERN_FLAME_HEIGHT } else { TORCH_FLAME_HEIGHT };
                let at = Vec3::new(TORCH_HAND.x, TORCH_HAND.y, TORCH_HAND.z);
                bearer.spawn((
                    Name::new("Torch fire"),
                    PoolCandidate {
                        offset: Vec3::from(flame_light_offset(flame_height)),
                        intensity: torch_lumens(TORCH_POOL, carried_light_height(flame_height)) * burn,
                        // The handed-out light breathes on this torch's own clock, so it matches the
                        // flame drawn on it instead of the pool slot's phase.
                        phase: Some(phase),
                        diameter: None,
                    },
                    // The pool's lights flicker at the un-sized rate (`Flicker { diameter: None }` in
                    // `pool.rs`), so the torch's flame must too — the same value on both is what keeps the
                    // flame and its light one fire.
                    FlameSite { height: flame_height, phase, kindle: 0.0, diameter: None },
                    Transform::from_translation(at),
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
            + Vec3::new(bearer.radius * bearer.angle.cos(), BEARER_LIFT, bearer.radius * bearer.angle.sin());
        transform.translation = at;
        // Facing along the circle: the capsule's forward is -Z, so the tangent is the heading.
        let heading = Vec3::new(-bearer.angle.sin(), 0.0, bearer.angle.cos()) * bearer.rate.signum();
        transform.rotation = Quat::from_rotation_y(f32::atan2(-heading.x, -heading.z));
    }
}

/// Orbits the focus at 6 m on a 20 s period./// Orbits the focus at 6 m on a 20 s period.
fn orbit_focus(time: Res<Time>, mut focus: Query<&mut Transform, With<Focus>>) {
    let elapsed = time.elapsed_secs();
    // `LIGHTS_FOCUS_WALK=<radius>` sends the focus out to that radius instead of its 6 m orbit. With a
    // radius past the pool's reach the torches stream across the reach boundary as it goes round, which
    // is the one thing that makes a candidate leave the pool — and so the one thing that makes a
    // cross-fade fire outside a test.
    let radius = std::env::var("LIGHTS_FOCUS_WALK")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .filter(|r| *r > 0.0)
        .unwrap_or(6.0);
    let at = Vec3::new(radius * (elapsed * TAU / 20.0).cos(), 1.2, radius * (elapsed * TAU / 20.0).sin());
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
    let torch_fire = torch_lumens(TORCH_POOL, carried_light_height(TORCH_FLAME_HEIGHT));
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

/// Prints one brazier light's intensity every 30 frames, so a capture's flicker can be read off the
/// log rather than guessed at from a frame's mean.
fn report_flicker(
    lights: Query<&PointLight, With<Flicker>>,
    candidates: Query<(), With<PoolCandidate>>,
    mut frame: Local<u32>,
) {
    *frame += 1;
    // The pool's total, which is the number a cross-fade is meant to hold: the sum of every light the
    // pool owns (its assigned slots and its reserve), across a hand-off.
    let total: f32 = lights.iter().map(|light| light.intensity).sum();
    let count = lights.iter().count();
    // `LIGHTS_POOL_LOG=1` prints the pool's total **every frame** — the only way to see a hand-off, since
    // a 0.35 s fade is invisible in a 30-frame sample and far below a frame's mean luminance.
    if std::env::var_os("LIGHTS_POOL_LOG").is_some() {
        let by_slot: Vec<String> = lights.iter().map(|light| format!("{:.0}", light.intensity)).collect();
        println!("pool {} {} {:.1} [{}]", *frame, count, total, by_slot.join(","));
    }
    if !frame.is_multiple_of(30) {
        return;
    }
    if let Some(light) = lights.iter().next() {
        println!(
            "flicker frame {}: {:.2} (pool total {:.0} over {count}, candidates {})",
            *frame,
            light.intensity,
            total,
            candidates.iter().count()
        );
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
