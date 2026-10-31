//! The showcase: four braziers, 48 torch-bearers, and a pool of eight real lights between them.
//!
//! A 40 × 40 m courtyard at night, in a metre world. Twelve pillars in two rows give the light
//! something to fall on and the pool something to hide behind; a moon fills the shadows cool and
//! dim; and the fires are the only warm light, which is the composition every night scene lives or
//! dies on.
//!
//! # What to look for
//!
//! - **The four braziers** (`Brazier`): each an iron bowl of coals with a shadow-casting `PointLight` at
//!   +0.9 m and a [`FlameSite`] on its rim. They all start with [`Kindle::default()`], so the first
//!   0.6 s is the whole courtyard catching at once; the `kindled` lines on stdout are the four
//!   [`Kindled`] messages.
//! - **The 48 torch-bearers** (`Bearer`): capsules walking circles of 3–15 m, each with a
//!   [`PoolCandidate`] at +1.8 m and a flame at the same place. Only the eight nearest the focus are
//!   real lights; the other forty are a flame and nothing else. Walk the camera and watch the pool
//!   hand its lights over. The three best-ranked of the eight cast shadows
//!   ([`LightPool::max_shadowed`]): a figure by a near torch throws a shadow away from it, and a shadow
//!   moving to another torch deepens there over a third of a second instead of appearing.
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
//! `LIGHTS_RELIGHT=<seconds>` re-lights the braziers once at that time (what `K` does); `LIGHTS_SNUFF=<seconds>` puts them out (what `O` does). `LIGHTS_FRAMES`, `LIGHTS_SHOT`, `LIGHTS_SHOT_DIR`/`LIGHTS_SHOT_EVERY`/`LIGHTS_SHOT_FROM`,
//! `LIGHTS_EYE`, `LIGHTS_LOOK`, `LIGHTS_SEED` and `LIGHTS_POOL` come from `examples/common/mod.rs`. `LIGHTS_EV100=x` sets the camera's exposure (default 6; 9 is a dark frame whose ground is not saturated by the fires). `LIGHTS_DEPTH_FADE=1`
//! sets `FlameSite::depth_fade` on every flame and gives the camera a `DepthPrepass`; the occlusion
//! case it is for has its own scene in `examples/occlusion.rs`.
//! `LIGHTS_RANK=distance` (rank by distance alone, the old rule), `LIGHTS_REACH`, `LIGHTS_FOCUS_WALK` (the focus's orbit radius; past the pool's reach the torches
//! stream across the boundary, which is what makes a candidate leave the pool) and `LIGHTS_POOL_LOG`
//! (the pool's total every frame, the only way to see a hand-off) are also this example's own.
//!
//! Six more are this example's own: `LIGHTS_RESERVE` (reserve lights for a cross-faded hand-off, default
//! 2, one for hand-offs and one for shadows; `0` dips instead), `LIGHTS_SHADOWED` (how many of the pool's
//! lights cast shadows, the best-ranked first; default 3, `0` for none), `LIGHTS_HYSTERESIS` (the pool's
//! overlap ratio, default 1.15),
//! `LIGHTS_FADE` (seconds a light takes to come back after a hand-off, default 0.35),
//! `LIGHTS_BED` (the braziers' fuel-bed diameter in metres, default 0.34, which sizes their flicker
//! through the puffing law) and `LIGHTS_UNSIZED=1` (ignore the bed and use the crate's inherited
//! rate, to see the difference). The first two set to the old behaviour (`1.0`, `0.0`) show the pop
//! the crate used to have, which is `docs/research/pool-hand-off.md`'s before/after.
//! `LIGHTS_GPU_TIMING=1` (with `LIGHTS_FRAMES=420`) prints the mean GPU time per render pass on stdout.
//! The default camera is the eye at `(0, 9, 16)` looking at the origin.
//!
//! ```sh
//! LIGHTS_FRAMES=600 LIGHTS_SHOT=/tmp/court.png cargo run --release --example courtyard
//! ```

mod common;

use std::f32::consts::TAU;

use bevy::asset::RenderAssetUsages;
use bevy::camera::Exposure;
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_lights::*;

use common::{Draw, ExampleHarness, FlyCamera, camera, camera_placement, gust_env, seed, wind_env};

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

/// The share of [`coal_radiance`] the bed's texture carries. At 1.0 the bed's red channel is ~35 times the
/// display's white at `NIGHT_EV100`, and the tonemapper (TonyMcMapface) rolls a value that far over towards
/// white: measured over the bed at 2.2 m, mean RGB 223/145/113 with 59 % of its pixels at R 255, a pale
/// pink plate whose coals read as terrazzo. At 0.2, with the lumps given a bright middle (`PLATEAU`), the same bed measures 191/79/41
/// with 14 % clipped: orange coals with dark char between them. A design default (`docs/research/fuel-bed.md`, round 2); the lining
/// keeps the full ramp, because its glow is a fraction of an ember and is not in the tonemapper's roll-off.
const BED_GAIN: f32 = 0.2;

/// The side of the generated char field, in texels: the bed is 0.34 m and the nearest camera in the
/// showcase is 4 m away, so a texel is about 2.7 mm — far under a pixel at every captured angle.
const COAL_FIELD: u32 = 128;
/// The char field's seed: the example's own, so a capture's coals are reproducible like its figures.
const COAL_FIELD_SEED: u32 = 7;

/// How far below the lip the coal bed's top face sits, in metres. See `docs/research/brazier-bowl.md`.
const BED_BELOW_LIP: f32 = 0.05;
/// The char-field value whose radiance the lining's glow is tinted from: a dull ember.
const LINING_EMBER_FIELD: f32 = 0.35;
/// The lining's glow at the bed's edge, as a fraction of that ember's radiance.
const LINING_GLOW: f32 = 0.8;
/// How fast the glow dies up the wall, as a fraction of the profile's `v`.
const LINING_GLOW_FALL: f32 = 0.15;
/// Sides of the bowl's revolved profile.
const BOWL_SEGMENTS: usize = 32;

/// A brazier's bowl, as one of two meshes: the iron shell (`lining == false`: the outside from foot to
/// lip, and the flat lip) or the soot lining (`lining == true`: the inside wall down to the bed). Both
/// are one revolved profile centred vertically on `BRAZIER_TOP / 2`, so each can be the mesh of an
/// entity placed at mid-height as the old box was. Separate strips keep the lip's corners sharp, and
/// separate meshes let a fire basket's inside be black with soot while its lip stays bright iron.
/// `docs/research/brazier-bowl.md`.
fn bowl_mesh(lining: bool) -> Mesh {
    let lift = -BRAZIER_TOP * 0.5;
    // (radius, height above the ground)
    let outside: [(f32, f32); 6] =
        [(0.20, 0.0), (0.17, 0.04), (0.10, 0.20), (0.10, 0.30), (0.34, 0.52), (0.44, BRAZIER_TOP)];
    let lip: [(f32, f32); 2] = [(0.44, BRAZIER_TOP), (0.38, BRAZIER_TOP)];
    let inside: [(f32, f32); 3] =
        [(0.38, BRAZIER_TOP), (0.33, BRAZIER_TOP - 0.05), (0.0, BRAZIER_TOP - 0.10)];
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let strips: &[&[(f32, f32)]] = if lining { &[&inside] } else { &[&outside, &lip] };
    for &strip in strips {
        // The profile's normal at each point: the edge's perpendicular, averaged across the two edges
        // that meet there. Each strip is traversed so that it points out of the solid.
        let edge_normal = |a: (f32, f32), b: (f32, f32)| {
            let (dr, dy) = (b.0 - a.0, b.1 - a.1);
            Vec2::new(dy, -dr).normalize_or_zero()
        };
        let base = positions.len() as u32;
        for (i, &(r, y)) in strip.iter().enumerate() {
            let before = if i > 0 { edge_normal(strip[i - 1], strip[i]) } else { Vec2::ZERO };
            let after = if i + 1 < strip.len() { edge_normal(strip[i], strip[i + 1]) } else { Vec2::ZERO };
            let n = (before + after).normalize_or_zero();
            for s in 0..=BOWL_SEGMENTS {
                let a = s as f32 / BOWL_SEGMENTS as f32 * TAU;
                let (sin, cos) = a.sin_cos();
                positions.push([r * cos, y + lift, r * sin]);
                normals.push([n.x * cos, n.y, n.x * sin]);
                uvs.push([s as f32 / BOWL_SEGMENTS as f32, i as f32 / (strip.len() - 1) as f32]);
            }
        }
        let ring = BOWL_SEGMENTS as u32 + 1;
        for i in 0..strip.len() as u32 - 1 {
            for s in 0..BOWL_SEGMENTS as u32 {
                let (a, b) = (base + i * ring + s, base + (i + 1) * ring + s);
                indices.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
            }
        }
    }
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

/// The heat the soot lining takes from the bed, as an emissive texture over the lining's profile: the
/// mesh's `v` runs from the lip (0) down the wall to the bed (0.5) and on to the centre (1), and the wall
/// glows from the bed's edge upward with a falloff of `LINING_GLOW_FALL` of the profile. The radiance at the
/// bed's edge is `LINING_GLOW` of a dull ember's. Design defaults, `docs/research/brazier-bowl.md`.
fn lining_glow_image() -> Image {
    const ROWS: u32 = 64;
    let ember = coal_radiance(LINING_EMBER_FIELD);
    let mut data: Vec<f32> = Vec::with_capacity((ROWS * 4 * 4) as usize);
    for row in 0..ROWS {
        let v = row as f32 / (ROWS - 1) as f32;
        let glow = if v < 0.5 { LINING_GLOW * (-(0.5 - v) / LINING_GLOW_FALL).exp() } else { LINING_GLOW };
        for _ in 0..4 {
            data.extend_from_slice(&[ember[0] * glow, ember[1] * glow, ember[2] * glow, 1.0]);
        }
    }
    let bytes = data.iter().flat_map(|f| f.to_le_bytes()).collect::<Vec<u8>>();
    Image::new(
        Extent3d { width: 4, height: ROWS, depth_or_array_layers: 1 },
        TextureDimension::D2,
        bytes,
        TextureFormat::Rgba32Float,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

/// The char field as an image a `StandardMaterial` can sample as its **emissive**: [`char_field`] over
/// the bed, mapped by [`coal_radiance`] to the bed's own radiance in candelas per square metre.
///
/// `Rgba32Float`, not `R8Unorm`, and the reason is the dynamic range rather than precision: a seam is
/// about **5 860 cd/m²** against a burnt-through lump's 17, a factor of 350, and an 8-bit texture clamps
/// at 1.0. A bed written into 8 bits is a bed with one brightness in it, which is the flat slab this
/// exists to replace. `StandardMaterial`'s `emissive_texture` is sampled as linear RGB with no
/// colour-space conversion, so the values here are the radiances the shader adds.
fn coal_radiance_image(seed: u32) -> Image {
    let mut data = Vec::with_capacity((COAL_FIELD * COAL_FIELD * 4) as usize);
    for y in 0..COAL_FIELD {
        for x in 0..COAL_FIELD {
            let u = x as f32 / (COAL_FIELD - 1) as f32;
            let v = y as f32 / (COAL_FIELD - 1) as f32;
            let rgb = coal_radiance(char_field(u, v, seed)).map(|c| c * BED_GAIN);
            data.extend_from_slice(&rgb);
            data.push(1.0);
        }
    }
    let bytes = data.iter().flat_map(|f| f.to_le_bytes()).collect::<Vec<u8>>();
    Image::new(
        Extent3d { width: COAL_FIELD, height: COAL_FIELD, depth_or_array_layers: 1 },
        TextureDimension::D2,
        bytes,
        TextureFormat::Rgba32Float,
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
/// `LIGHTS_KIND=<candle|lantern|torch|brazier|campfire|bonfire>`: dress the braziers' flame and flicker as
/// that [`FireKind`], to look at a kind's numbers on the real scene (the bed's box is unchanged).
fn brazier_kind() -> Option<FireKind> {
    let name = std::env::var("LIGHTS_KIND").ok()?;
    name.parse().ok()
}
/// How far a brazier's pool reaches, in metres: the ground it lights to [`POOL_LUX`].
const BRAZIER_POOL: f32 = FireKind::Brazier.reach();
/// How far a carried torch's pool reaches. Smaller than a brazier's: a 0.35 m flame lighting a 4 m
/// patch is the "flame too small for the ground it lights" defect `lighting-look.md` names.
const TORCH_POOL: f32 = FireKind::Torch.reach();
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
/// A torch's stick, and a lantern's body: the held object that ends at the flame's foot.
const TORCH_STICK: f32 = 0.55;
const LANTERN_BODY: f32 = 0.14;

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
/// The braziers' places: the courtyard's four inner corners, 5 m out from the centre.
const BRAZIERS: [Vec3; 4] = [
    Vec3::new(5.0, 0.0, 5.0),
    Vec3::new(-5.0, 0.0, 5.0),
    Vec3::new(-5.0, 0.0, -5.0),
    Vec3::new(5.0, 0.0, -5.0),
];

/// `LIGHTS_SHADOWED=N`: how many of the pool's lights cast shadows, the best-ranked first (default 3).
fn shadowed_env() -> usize {
    std::env::var("LIGHTS_SHADOWED").ok().and_then(|v| v.parse().ok()).unwrap_or(3)
}

/// `LIGHTS_GPU_TIMING=1` records the render graph's GPU timestamps and prints, once, the mean per pass
/// (microseconds, frames 100 to 400) on stdout as `gpu_us <pass> <mean>`. It needs a Vulkan or DX12
/// adapter with timestamp queries; without one the run prints nothing. Run it with `LIGHTS_FRAMES=420`.
fn gpu_timing_on() -> bool {
    std::env::var("LIGHTS_GPU_TIMING").is_ok_and(|v| v != "0")
}

fn report_gpu(
    store: Res<bevy::diagnostic::DiagnosticsStore>,
    mut sums: Local<std::collections::BTreeMap<String, (f64, u32)>>,
    mut frame: Local<u32>,
) {
    *frame += 1;
    if (100..400).contains(&*frame) {
        for d in store.iter().filter(|d| d.path().as_str().ends_with("elapsed_gpu")) {
            if let Some(ms) = d.value() {
                let entry = sums.entry(d.path().to_string()).or_default();
                entry.0 += ms;
                entry.1 += 1;
            }
        }
    }
    if *frame == 400 {
        for (path, (sum, n)) in sums.iter() {
            println!("gpu_us {path} {:.1}", sum / f64::from((*n).max(1)) * 1000.0);
        }
    }
}

/// Adds the render diagnostics and the report when `LIGHTS_GPU_TIMING` asks for them.
struct GpuTiming;

impl Plugin for GpuTiming {
    fn build(&self, app: &mut App) {
        if gpu_timing_on() {
            app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin)
                .add_systems(Update, report_gpu);
        }
    }
}

fn main() {
    let (eye, look) = camera_placement(Vec3::new(0.0, 9.0, 16.0), Vec3::ZERO);
    App::new()
        .add_plugins(
            DefaultPlugins.set(bevy::render::RenderPlugin {
                render_creation: bevy::render::settings::WgpuSettings {
                    features: if gpu_timing_on() {
                        bevy::render::settings::WgpuFeatures::TIMESTAMP_QUERY
                            | bevy::render::settings::WgpuFeatures::TIMESTAMP_QUERY_INSIDE_PASSES
                            | bevy::render::settings::WgpuFeatures::TIMESTAMP_QUERY_INSIDE_ENCODERS
                    } else {
                        bevy::render::settings::WgpuFeatures::empty()
                    },
                    ..Default::default()
                }
                .into(),
                ..Default::default()
            }),
        )
        .add_plugins((ExampleHarness, FlickerPlugin, LightPoolPlugin::<Focus>::default(), FlamePlugin))
        .insert_resource(CameraPlacement(eye, look))
        // The pool is inserted before the plugin's `init_resource`, so these numbers win: eight real
        // lights over a 20 m reach, each an 8 m point light, the best-ranked three of them casting shadows
        // (the braziers cast their own).
        .insert_resource(LightPool {
            size: std::env::var("LIGHTS_POOL").ok().and_then(|v| v.parse().ok()).unwrap_or(8),
            // `LIGHTS_RANK=distance` ranks by distance alone, the rule before the pool weighed a light's
            // output; the default ranks by the illuminance each candidate puts on the subject.
            rank: match std::env::var("LIGHTS_RANK").ok().as_deref() {
                Some("distance") => Rank::Distance,
                _ => Rank::Illuminance,
            },
            // `LIGHTS_RESERVE=0` makes a hand-off dip rather than cross-fade, which is the comparison. Two by
            // default: a pool with shadows shares its reserves between hand-offs and shadow changes, and one
            // left three shadows filled in 40 % of frames on this courtyard where two fill them in 99 %.
            reserve: std::env::var("LIGHTS_RESERVE").ok().and_then(|v| v.parse().ok()).unwrap_or(2),
            // The pool's shadow budget: the best-ranked `LIGHTS_SHADOWED` fires throw shadows and the rest
            // only light (the template's flag below turns them on at all).
            max_shadowed: Some(shadowed_env()),
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
                shadow_maps_enabled: shadowed_env() > 0,
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
        .insert_resource(FlameWind { velocity: wind_env().unwrap_or_default() })
        .insert_resource(GlobalAmbientLight { color: Color::WHITE, brightness: 0.0, ..default() })
        .add_systems(Startup, (spawn_scene, spawn_camera, gust_from_env).chain())
        .add_systems(Update, (walk_bearers, orbit_focus, relight_braziers, snuff_braziers).chain())
        .add_systems(Update, (report_kindled, report_snuffed))
        .add_systems(Update, report_look)
        .add_systems(Update, sweep_camera)
        .add_systems(Update, report_flicker)
        .add_plugins(GpuTiming)
        .run();
}

/// `LIGHTS_GUST=I`: the wind gusts about `LIGHTS_WIND` with turbulence intensity `I`.
fn gust_from_env(mut commands: Commands) {
    if let Some(intensity) = gust_env() {
        commands.insert_resource(FlameGust {
            mean: wind_env().unwrap_or_default(),
            intensity,
            seed: seed() as u32,
            ..default()
        });
    }
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

/// `LIGHTS_DEPTH_FADE=1` turns on `FlameSite::depth_fade` for every flame and gives the camera the
/// `DepthPrepass` it reads; off, the courtyard draws exactly as it always has.
fn depth_fade_on() -> bool {
    std::env::var("LIGHTS_DEPTH_FADE").is_ok_and(|v| v != "0")
}

/// `LIGHTS_EMBERS=r` gives every flame `FlameEmbers { rate: r }` (sparks a second); unset or `0`, the courtyard draws no sparks.
fn embers_on() -> Option<FlameEmbers> {
    let rate: f32 = std::env::var("LIGHTS_EMBERS").ok()?.trim().parse().ok()?;
    (rate > 0.0).then_some(FlameEmbers { rate })
}

/// `LIGHTS_EV100=x` overrides the camera's exposure (default [`NIGHT_EV100`]). A higher value is a darker
/// frame: with the ground not saturated, a flame's own contrast can be judged against it.
fn exposure_ev100() -> f32 {
    std::env::var("LIGHTS_EV100").ok().and_then(|v| v.parse().ok()).unwrap_or(NIGHT_EV100)
}

fn sweep_camera(
    mut frame: Local<u32>,
    placement: Res<CameraPlacement>,
    mut cameras: Query<(&mut FlyCamera, &mut Transform)>,
) {
    *frame += 1;
    let t = (*frame as f32 / 420.0).min(1.0);
    let CameraPlacement(eye, look) = *placement;
    let at = Vec3::new(-16.0 + 32.0 * t, look.y, look.z);
    for (mut fly, mut transform) in &mut cameras {
        fly.aim(&mut transform, eye, at);
    }
}

fn spawn_camera(mut commands: Commands, placement: Res<CameraPlacement>, fill: Res<NightFill>) {
    let CameraPlacement(eye, look) = *placement;
    let mut camera = commands.spawn((
        camera(eye, look, 8.0),
        // A cool fill the moon does not have to provide: the sky dome under it, which lights the faces
        // the moon's direction misses. Without it a figure's shadow thrown by a fire onto a wall the moon
        // does not reach is a black hole (`srgb(1,1,5)` at 2 cd/m²); at 20 (about 63 lx, an eighth of the
        // moon) it is a navy shadow (`lighting-look.md`). Its colour is the sky's temperature, not a
        // hand-picked blue.
        AmbientLight { color: sky_colour(), brightness: 20.0, ..default() },
        Exposure { ev100: exposure_ev100() },
    ));
    // The depth fade reads the depth prepass, so a camera that asks for it carries one.
    if depth_fade_on() {
        camera.insert(DepthPrepass);
    }
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
    // Cast iron: near-black, and glossier than the stone so the lip can carry the one highlight on the prop.
    let bowl_material = materials.add(StandardMaterial {
        base_color: Srgba::rgb(0.03, 0.028, 0.026).into(),
        perceptual_roughness: 0.45,
        ..default()
    });
    // The inside of a fire basket is soot: darker than iron and with no sheen, so the fire's own light,
    // 0.5 m away, does not turn the lining into a bright plate.
    let soot_material = materials.add(StandardMaterial {
        base_color: Srgba::rgb(0.016, 0.0145, 0.013).into(),
        perceptual_roughness: 0.95,
        reflectance: 0.0,
        // The wall nearest the bed is heated by it and glows dull red; the glow dies toward the rim, which
        // is what makes the inside a gradient instead of one tone (`docs/research/brazier-bowl.md`).
        emissive: LinearRgba::new(1.0, 1.0, 1.0, 1.0),
        emissive_texture: Some(images.add(lining_glow_image())),
        emissive_exposure_weight: 1.0,
        ..default()
    });
    // A bed of coals is a *surface with a temperature field on it*, not a uniform emitter: burnt-through
    // char is dark and glowing fuel is bright, interleaved at the scale of a coal. A constant emissive
    // was the brightest thing in the frame at every camera and clipped to a flat white slab from above,
    // where it also hid the flame standing in it — `docs/research/fuel-bed.md` has the measurements.
    //
    // The bed emits, and it does **not** reflect. Two things follow, and both were measured on a probe
    // rather than assumed (`/tmp/l21` in the log's iteration):
    //
    // 1. The glow rides on `emissive`, not on `base_color`. `unlit: true` was the first attempt and it is
    //    wrong here, not for its colour but for who owns exposure: Bevy's unlit branch returns
    //    `base_color` **without** multiplying by `view.exposure` (`bevy_pbr-0.20.0/src/render/pbr.wesl`
    //    lines 83-86, against `pbr_functions.wesl` line 832 for emissive). A bed that ignores exposure
    //    cannot be exposed, and exposure belongs to `bevy_elements`; this crate never meters it.
    // 2. Its albedo is charcoal, not the old 0.14. A glowing coal is an emitter, and the light landing on
    //    it from the fire *above* it — 58 000 lx at point-blank range, the measurement `fuel-bed.md`
    //    records — was what clipped the slab white in the first place, not the emissive at all.
    //
    // The field is generated here rather than shipped as an image: it is a heap of coal lumps built from
    // an integer hash, seeded from the run, so a capture is reproducible and the example stays one
    // binary.
    let bed_field = images.add(coal_radiance_image(COAL_FIELD_SEED));
    let coal_material = materials.add(StandardMaterial {
        // Charcoal, and near-black on purpose. This number is the one that decides whether the bed reads
        // as coals at all, and the reason is geometry: the brazier's light sits 0.575 m above the bed, so
        // `fire_lumens`' own inverse square puts **~58 000 lx** on it — 50× what the same fire delivers to
        // the ground at its pool's edge. At an albedo of 0.03 that is 554 cd/m² of *reflection*, which is
        // more than a coal at 1250 K radiates (385 cd/m²): the fire's own light flattened the bed into a
        // bright plate and buried the field, which is exactly what the capture showed. At 0.004 it is
        // 74 cd/m², under the ember stop and far under the seams, so the bed's own temperature is what a
        // viewer reads. Charcoal's real albedo is 0.02-0.04 and this is darker than that; the shortfall
        // is the point light being unphysically close to the fuel, and `fuel-bed.md` records the trade.
        base_color: Srgba::rgb(0.004, 0.0035, 0.0032).into(),
        perceptual_roughness: 0.95,
        // No specular at all, and this is the number that matters most. With Bevy's default reflectance
        // (`F0` = 4 %) the bed's **specular** image of its own light — 58 000 lx from 0.575 m above —
        // measured **~740 cd/m²**, brighter than any coal and a flat sheen across the whole bed: rendered
        // with black albedo and zero emissive it still read 245/153/88. The field was being buried under
        // the fire's reflection *of itself*. `reflectance: 0.0` removes the term, and `fuel-bed.md`
        // records why a fuel bed is the one surface in a scene for which that is right.
        reflectance: 0.0,
        // The texture *is* the bed's radiance, texel by texel, so the emissive is white and the texture
        // supplies the colour: a fire colour here would multiply a colour with no blue into a crimson
        // bed, because a 1900 K blackbody's blue channel is zero.
        emissive: LinearRgba::new(1.0, 1.0, 1.0, 1.0),
        emissive_texture: Some(bed_field),
        // Emissive is multiplied by the camera's exposure, like every other light contribution, so the
        // bed is exposed with the scene and `bevy_elements` keeps owning exposure.
        emissive_exposure_weight: 1.0,
        ..default()
    });
    // The bed's *drawn* size is the same number that sizes its flicker and its flame. It used to be
    // hard-coded at 0.34 while `LIGHTS_BED` moved only the other two, so a host that sized a fire
    // differently got a bed that did not match the fire standing on it — and `LIGHTS_UNSIZED`, whose
    // whole point is to compare two flicker rates, drew the same box either way.
    let bed_side = brazier_bed().unwrap_or(BRAZIER_BED);
    let coal_bed = meshes.add(Circle::new(bed_side * 0.5));

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
                Mesh3d(meshes.add(bowl_mesh(false))),
                MeshMaterial3d(bowl_material.clone()),
                // The coals sit in the bowl and the light is above them: the bowl does not shade its
                // own fire. Cast, it throws a hard dark square across the ground around every brazier.
                NotShadowCaster,
                Transform::from_translation(*at + Vec3::Y * BRAZIER_TOP * 0.5),
            ))
            .with_children(|brazier| {
                // The bowl's inside, a separate mesh so it can be soot rather than iron.
                brazier.spawn((
                    Name::new("Soot lining"),
                    Mesh3d(meshes.add(bowl_mesh(true))),
                    MeshMaterial3d(soot_material.clone()),
                    NotShadowCaster,
                ));
                // The source the light is motivated by: a bed of glowing coals sunk below the bowl's lip.
                brazier.spawn((
                    Name::new("Coal bed"),
                    Mesh3d(coal_bed.clone()),
                    MeshMaterial3d(coal_material.clone()),
                    Transform::from_xyz(0.0, BRAZIER_TOP * 0.5 - BED_BELOW_LIP, 0.0)
                        .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
                    NotShadowCaster,
                ));
                // The fire's size, from its pool: POOL_LUX at BRAZIER_POOL metres out, from a light
                // BRAZIER_LIGHT over the ground. 60 000 lm was 20x this and blew the pool it lit.
                let lumens = lumens_for(BRAZIER_POOL, BRAZIER_LIGHT);
                let coals = brazier
                    .spawn((
                        Name::new("Coals"),
                        PointLight {
                            color: fire_colour(),
                            intensity: lumens,
                            range: BRAZIER_POOL * POOL_RANGE_MARGIN,
                            shadow_maps_enabled: true,
                            ..default()
                        },
                        Flicker {
                            base: lumens,
                            phase,
                            diameter: brazier_kind().map_or(brazier_bed(), |k| Some(k.diameter())),
                            depth: brazier_kind().map_or(FLICKER_DEPTH, FireKind::depth),
                        },
                        Kindle::default(),
                        Transform::from_xyz(0.0, BRAZIER_LIGHT - BRAZIER_TOP * 0.5, 0.0),
                    ))
                    .id();
                let mut site = brazier.spawn((
                    Name::new("Flame site"),
                    // The flame names the coals as its fire, so it catches with them: at the start, and again
                    // whenever the coals are re-lit (`K`, or `LIGHTS_RELIGHT`), the fire grows rather than
                    // appearing whole.
                    FlameSite {
                        height: brazier_kind().map_or(BRAZIER_FLAME_HEIGHT, FireKind::height),
                        phase,
                        kindle: KINDLE_SECONDS,
                        diameter: brazier_kind().map_or(brazier_bed(), |k| Some(k.diameter())),
                        depth: brazier_kind().map_or(FLICKER_DEPTH, FireKind::depth),
                        depth_fade: depth_fade_on(),
                        fire: Some(coals),
                    },
                    // The flame's foot is on the coals, below the lip with them.
                    Transform::from_xyz(0.0, BRAZIER_TOP * 0.5 + 0.04 - BED_BELOW_LIP, 0.0),
                ));
                if let Some(embers) = embers_on() {
                    site.insert(embers);
                }
            });
    }

    // The torch-bearers: capsules on circles, each with its own fire.
    let mut draw = Draw::new(seed());
    let body = meshes.add(Capsule3d::new(0.15, 1.4));
    let cloth = materials.add(stone(Srgba::rgb(0.20, 0.19, 0.22)));
    let head = meshes.add(Sphere::new(0.11));
    let skin = materials.add(stone(Srgba::rgb(0.42, 0.33, 0.28)));
    // What the flame is held by: a flame with nothing under it is a light with no source
    // (`docs/research/carried-fire.md`, iteration 60). Dark wood and iron, so the fire's own pool is what lights them.
    let wood = materials.add(stone(Srgba::rgb(0.09, 0.06, 0.04)));
    let iron = materials.add(stone(Srgba::rgb(0.05, 0.05, 0.055)));
    let stick = meshes.add(Cylinder::new(0.02, TORCH_STICK));
    let lamp = meshes.add(Cylinder::new(0.07, LANTERN_BODY));
    let fist = meshes.add(Sphere::new(0.05));
    let arm = meshes.add(Cylinder::new(0.05, 1.0));
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
                // The figure throws a shadow, from the braziers and from the pool's shadowed lights (its
                // own torch among them): a figure standing in firelight with no shadow under it reads
                // as pasted on. It receives none: beside its own torch, a capsule would gain only shadow
                // acne on the side the light already misses.
                bearer.spawn((Mesh3d(body.clone()), MeshMaterial3d(cloth.clone()), NotShadowReceiver));
                bearer.spawn((
                    Mesh3d(head.clone()),
                    MeshMaterial3d(skin.clone()),
                    Transform::from_xyz(0.0, 1.05, 0.0),
                    NotShadowReceiver,
                ));
                // The carried fire: one entity, so the flame and the real light it is handed can never
                // drift apart. The light sits at the flame's own centre
                // (`flame_light_offset`), which is the only place a point light approximates a fire of
                // this height without leaving the drawn flame — and the height the light is sized for is
                // read off that same geometry, not typed a second time.
                let flame_height = if lantern { LANTERN_FLAME_HEIGHT } else { TORCH_FLAME_HEIGHT };
                let at = Vec3::new(TORCH_HAND.x, TORCH_HAND.y, TORCH_HAND.z);
                // The held object ends at the flame's foot (`at`): a stick below a torch's flame, a lantern's
                // body below a lantern's. The fist closes on it, and an arm joins the fist to the shoulder.
                let (held, held_material, held_centre, grip) = if lantern {
                    (lamp.clone(), iron.clone(), LANTERN_BODY * 0.5, LANTERN_BODY + 0.03)
                } else {
                    (stick.clone(), wood.clone(), TORCH_STICK * 0.5, TORCH_STICK * 0.5)
                };
                bearer.spawn((
                    Name::new("Held object"),
                    Mesh3d(held),
                    MeshMaterial3d(held_material),
                    Transform::from_translation(at - Vec3::Y * held_centre),
                    NotShadowCaster,
                    NotShadowReceiver,
                ));
                let hand = at - Vec3::Y * grip;
                bearer.spawn((
                    Name::new("Fist"),
                    Mesh3d(fist.clone()),
                    MeshMaterial3d(skin.clone()),
                    Transform::from_translation(hand),
                    NotShadowCaster,
                    NotShadowReceiver,
                ));
                let shoulder = Vec3::new(0.13, 0.45, 0.04);
                let reach = hand - shoulder;
                bearer.spawn((
                    Name::new("Arm"),
                    Mesh3d(arm.clone()),
                    MeshMaterial3d(cloth.clone()),
                    Transform {
                        translation: shoulder + reach * 0.5,
                        rotation: Quat::from_rotation_arc(Vec3::Y, reach.normalize()),
                        scale: Vec3::new(1.0, reach.length(), 1.0),
                    },
                    NotShadowCaster,
                    NotShadowReceiver,
                ));
                let mut torch = bearer.spawn_empty();
                let fire = torch.id();
                torch.insert((
                    Name::new("Torch fire"),
                    PoolCandidate {
                        offset: Vec3::from(flame_light_offset(flame_height)),
                        intensity: torch_lumens(TORCH_POOL, carried_light_height(flame_height)) * burn,
                        // The handed-out light breathes on this torch's own clock, so it matches the
                        // flame drawn on it instead of the pool slot's phase.
                        phase: Some(phase),
                        diameter: None,
                        ..default()
                    },
                    // The pool's lights flicker at the un-sized rate (`Flicker { diameter: None }` in
                    // `pool.rs`), so the torch's flame must too — the same value on both is what keeps the
                    // flame and its light one fire.
                    FlameSite {
                        height: flame_height,
                        phase,
                        kindle: 0.0,
                        diameter: None,
                        depth: FLICKER_DEPTH,
                        depth_fade: depth_fade_on(),
                        fire: Some(fire),
                    },
                    Transform::from_translation(at),
                ));
                if let Some(embers) = embers_on() {
                    torch.insert(embers);
                }
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

/// `K` re-lights every brazier and every torch: re-inserting [`Kindle::default()`] restarts the flare, and the
/// message is written again. `LIGHTS_RELIGHT=<seconds>` does it once at that elapsed time, so an
/// unattended capture can show the flame catching with its light again.
fn relight_braziers(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    braziers: Query<&Children, With<Brazier>>,
    lights: Query<(), With<Kindle>>,
    torches: Query<Entity, With<PoolCandidate>>,
    mut timed: Local<Option<Option<f32>>>,
) {
    let at = *timed.get_or_insert_with(|| std::env::var("LIGHTS_RELIGHT").ok().and_then(|v| v.parse().ok()));
    let elapsed = time.elapsed_secs();
    let due = at.is_some_and(|at| elapsed - time.delta_secs() < at && elapsed >= at);
    if !keys.just_pressed(KeyCode::KeyK) && !due {
        return;
    }
    for children in &braziers {
        for child in children.iter() {
            if lights.contains(child) {
                commands.entity(child).remove::<Snuff>().insert(Kindle::default());
            }
        }
    }
    for torch in &torches {
        commands.entity(torch).remove::<Snuff>().insert(Kindle::default());
    }
}

/// `O` puts every brazier and every torch out: inserting [`Snuff::default()`] dims each light to nothing and
/// shrinks the flame linked to it (a torch's light is the pool's: it dims on its candidate), and [`Snuffed`] is
/// written when the last of it is gone. `LIGHTS_SNUFF=<seconds>`
/// does it once at that elapsed time, so an unattended capture can show a fire going out. `K` lights them
/// again.
fn snuff_braziers(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    braziers: Query<&Children, With<Brazier>>,
    lights: Query<(), With<Kindle>>,
    torches: Query<Entity, With<PoolCandidate>>,
    mut timed: Local<Option<Option<f32>>>,
) {
    let at = *timed.get_or_insert_with(|| std::env::var("LIGHTS_SNUFF").ok().and_then(|v| v.parse().ok()));
    let elapsed = time.elapsed_secs();
    let due = at.is_some_and(|at| elapsed - time.delta_secs() < at && elapsed >= at);
    if !keys.just_pressed(KeyCode::KeyO) && !due {
        return;
    }
    for children in &braziers {
        for child in children.iter() {
            if lights.contains(child) {
                commands.entity(child).insert(Snuff::default());
            }
        }
    }
    for torch in &torches {
        commands.entity(torch).insert(Snuff::default());
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
    // a 0.35 s fade is invisible in a 30-frame sample and far below a frame's mean luminance. A `*` marks
    // a light casting a shadow (the braziers, and the pool's `LIGHTS_SHADOWED`).
    if std::env::var_os("LIGHTS_POOL_LOG").is_some() {
        let by_slot: Vec<String> = lights
            .iter()
            .map(|light| {
                format!("{:.0}{}", light.intensity, if light.shadow_maps_enabled { "*" } else { "" })
            })
            .collect();
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

/// Prints every [`Snuffed`]: four lines when the braziers have gone out.
fn report_snuffed(mut snuffed: MessageReader<Snuffed>, names: Query<&Name>) {
    for message in snuffed.read() {
        match names.get(message.entity) {
            Ok(name) => println!("snuffed {} ({})", message.entity, name),
            Err(_) => println!("snuffed {}", message.entity),
        }
    }
}
