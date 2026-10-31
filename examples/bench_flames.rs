//! What a scene full of fires costs a frame when almost none of them is on screen: the flames' own bench.
//!
//! `bench_lights` has no renderer, so it cannot see a flame. A game that puts a flame on every torch of a
//! town has thousands of `FlameSite`s in the world and a few dozen in front of the camera, and the cost it
//! must be able to read is the cost of the rest. This example builds that scene: `BENCH_FLAMES` torches
//! (default 5000) on a 3 m grid, one camera looking at a corner of it, so about twenty are in view; renders
//! it for `BENCH_FRAMES` frames (default 400, the first 100 not counted) to an off-screen image, with no
//! window and no display; and prints one line:
//!
//! ```text
//! bench_flames flames=5000 visible=21 embers=0 embers_visible=0 smoke=0 smoke_visible=0 tongues=0 beds=0 beds_visible=0 wind=0,0 frames=300 mean_ms=… p50_ms=… min_ms=…
//! ```
//!
//! `visible` is the number of flames the renderer counted as in view at the last frame (a fire's tongues are flames too), which is what
//! proves the others were culled. Every `*_ms` is one `App::update()`, main world and the render world's CPU
//! work together (the app is pipelined, so the slower of the two shows), and it needs a GPU adapter to run,
//! because the renderer does. Compare a run against a run of the same binary's predecessor on the same machine
//! and the same load, never against another host. `BENCH_WIND=x,z` blows a wind of that velocity (metres per
//! second) through every flame, which makes the flames lean and so rewrites each one's scale and its culling
//! box every frame. `BENCH_EMBERS=1` gives every torch `FlameEmbers` (a second quad a flame), and `embers_visible` is
//! how many of those quads the renderer counted as in view. `BENCH_SMOKE=1` gives every torch `FlameSmoke` (one more quad,
//! the smoke's), and `smoke_visible` counts those the same way. `BENCH_TONGUES=n` gives every torch `FlameTongues` of `n` tongues
//! (`n - 1` more flames a fire, each its own draw). `BENCH_BEDS=1` gives every torch a `FuelBed` (one more disc a fire, an opaque
//! cut-out), and `beds_visible` counts the discs the renderer counted as in view.

use std::time::Instant;

use bevy::camera::RenderTarget;
use bevy::camera::visibility::ViewVisibility;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::window::ExitCondition;
use bevy::winit::WinitPlugin;
use bevy_lights::*;

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// The grid's spacing, in metres: a street of torches, not a bonfire's worth in one spot.
const SPACING: f32 = 3.0;

/// How many frames are run and thrown away first: the flame material's pipeline compiles in them.
const WARM_UP: usize = 100;

fn main() {
    let flames = env_usize("BENCH_FLAMES", 5_000);
    let embers = env_usize("BENCH_EMBERS", 0) != 0;
    let smoke = env_usize("BENCH_SMOKE", 0) != 0;
    let tongues = u8::try_from(env_usize("BENCH_TONGUES", 0)).unwrap_or(u8::MAX);
    let beds = env_usize("BENCH_BEDS", 0) != 0;
    let frames = env_usize("BENCH_FRAMES", 400);
    let wind = std::env::var("BENCH_WIND").ok().and_then(|v| {
        let v: Vec<f32> = v.split(',').filter_map(|p| p.trim().parse().ok()).collect();
        (v.len() == 2).then(|| Vec2::new(v[0], v[1]))
    });

    let mut app = App::new();
    app.add_plugins((
        // No window and no event loop: the app is driven by hand below, and the camera draws to an image.
        DefaultPlugins
            .set(WindowPlugin { primary_window: None, exit_condition: ExitCondition::DontExit, ..default() })
            .disable::<WinitPlugin>(),
        FlamePlugin,
    ))
    .insert_resource(FlameWind { velocity: wind.unwrap_or_default() })
    .add_systems(Startup, move |commands: Commands, images: ResMut<Assets<Image>>| {
        scene(commands, images, flames, embers, smoke, tongues, beds)
    });
    app.finish();
    app.cleanup();

    let mut samples = Vec::with_capacity(frames);
    for frame in 0..frames {
        let start = Instant::now();
        app.update();
        if frame >= WARM_UP {
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    let visible = |world: &mut World, name: Option<&str>| {
        world
            .query::<(&ViewVisibility, Option<&Name>, Has<Flame>)>()
            .iter(world)
            .filter(|(view, quad, is_flame)| match name {
                None => *is_flame && view.get(),
                Some(name) => view.get() && quad.is_some_and(|quad| quad.as_str() == name),
            })
            .count()
    };
    let flames_visible = visible(app.world_mut(), None);
    let embers_visible = visible(app.world_mut(), Some("Embers"));
    let smoke_visible = visible(app.world_mut(), Some("Smoke"));
    let beds_visible = visible(app.world_mut(), Some("Fuel bed"));

    samples.sort_by(f64::total_cmp);
    let mean = samples.iter().sum::<f64>() / samples.len().max(1) as f64;
    let at = |i: usize| samples.get(i).copied().unwrap_or(0.0);
    let wind = wind.unwrap_or_default();
    println!(
        "bench_flames flames={flames} visible={flames_visible} embers={} embers_visible={embers_visible} smoke={} smoke_visible={smoke_visible} tongues={tongues} beds={} beds_visible={beds_visible} wind={},{} frames={} mean_ms={mean:.3} p50_ms={:.3} min_ms={:.3}",
        u8::from(embers),
        u8::from(smoke),
        u8::from(beds),
        wind.x,
        wind.y,
        samples.len(),
        at(samples.len() / 2),
        at(0),
    );
}

/// The torches, and one camera at the grid's corner, drawing to an image no one reads.
fn scene(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    flames: usize,
    embers: bool,
    smoke: bool,
    tongues: u8,
    beds: bool,
) {
    let mut target = Image::new_fill(
        Extent3d { width: 1280, height: 720, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        default(),
    );
    target.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
    let target = images.add(target);

    let side = (flames as f32).sqrt().ceil() as usize;
    for index in 0..flames {
        let at = Vec3::new((index % side) as f32 * SPACING, 0.0, (index / side) as f32 * SPACING);
        let mut torch = commands.spawn((FireKind::Torch.site(), Transform::from_translation(at)));
        if embers {
            torch.insert(FlameEmbers::default());
        }
        if smoke {
            torch.insert(FlameSmoke::default());
        }
        if tongues > 1 {
            torch.insert(FlameTongues { count: tongues, spread: 0.55 });
        }
        if beds {
            torch.insert(FuelBed::default());
        }
    }
    commands.spawn((
        Camera3d::default(),
        RenderTarget::Image(target.into()),
        Transform::from_xyz(4.0, 1.5, 12.0).looking_at(Vec3::new(4.0, 1.0, 0.0), Vec3::Y),
    ));
}
