//! Shared example scaffolding: a free-fly camera and the `LIGHTS_*` switches an unattended run uses.
//!
//! - `LIGHTS_FRAMES=N` exits after N frames at a fixed 1/60 s step, so every run simulates the same
//!   frames and a capture is comparable across builds. Without it the examples run on wall-clock
//!   time.
//! - `LIGHTS_SHOT=/tmp/x.png` saves a screenshot of the primary window on the last frame, then
//!   exits.
//! - `LIGHTS_SHOT_DIR=/tmp/frames` saves a frame sequence there (created if missing) as
//!   `frame0000.png` and up, `LIGHTS_SHOT_EVERY=N` capturing every Nth frame (default 1) from
//!   `LIGHTS_SHOT_FROM` (default 1). Flicker, kindling and the pool's hand-offs are temporal, so
//!   this is how they are reviewed; skip the first dozen frames while pipelines compile.
//! - `LIGHTS_EYE=x,y,z` and `LIGHTS_LOOK=x,y,z` place and aim the camera.
//! - `LIGHTS_SEED=N` (default 7) moves the torch-bearers' start angles, so a capture is not always
//!   the same arrangement of figures.
//! - `LIGHTS_POOL=N` overrides the pool's size (the courtyard's default is 8).
//! - `LIGHTS_WIND=x,z` blows a wind of that velocity (metres per second, on the ground plane) through
//!   the flames; unset is still air.
//! - `LIGHTS_GUST=I` makes that wind gust with turbulence intensity `I` (0.25 is a lively courtyard);
//!   `LIGHTS_WIND` is then the mean, and `LIGHTS_SEED` picks the wind.
//! - `LIGHTS_KIND=<name>` (courtyard): the braziers burn as that `FireKind`.
//!
//! Keys: WASD + Q/E move (Shift ×5), hold the right mouse button to look.
//!
//! The camera's own eye/look defaults come from the example, not from here, so a bare run of each
//! example shows the composition its doc comment describes.

#![allow(dead_code)]

use std::time::Duration;

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::render::view::window::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk};
use bevy::time::TimeUpdateStrategy;

/// Adds the harness to an example.
pub struct ExampleHarness;

/// A vector from an `x,y,z` environment variable (`LIGHTS_EYE`, `LIGHTS_LOOK`).
pub fn vec3_env(name: &str) -> Option<Vec3> {
    let v: Vec<f32> = std::env::var(name).ok()?.split(',').filter_map(|p| p.trim().parse().ok()).collect();
    (v.len() == 3).then(|| Vec3::new(v[0], v[1], v[2]))
}

/// The wind from `LIGHTS_WIND=x,z`, in metres per second; `None` is still air.
pub fn wind_env() -> Option<Vec2> {
    let v: Vec<f32> =
        std::env::var("LIGHTS_WIND").ok()?.split(',').filter_map(|p| p.trim().parse().ok()).collect();
    (v.len() == 2).then(|| Vec2::new(v[0], v[1]))
}

/// The turbulence intensity from `LIGHTS_GUST=I`; `None` is a steady wind.
pub fn gust_env() -> Option<f32> {
    std::env::var("LIGHTS_GUST").ok()?.trim().parse().ok()
}

/// `LIGHTS_SEED` (default 7): the seed the examples' own randomness draws from.
///
/// A local xorshift64*, so a seeded example needs no dependency for it. It is not a simulation
/// seed: nothing in the crate is random, and the examples use it only to lay out their figures.
pub fn seed() -> u64 {
    std::env::var("LIGHTS_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(7)
}

/// A seeded draw: xorshift64*.
pub struct Draw(u64);

impl Draw {
    /// A draw from `seed`; a zero seed is stepped off zero, since xorshift cannot leave it.
    pub fn new(seed: u64) -> Self {
        Self(if seed == 0 { 0x9e37_79b9_7f4a_7c15 } else { seed })
    }

    /// The next value.
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// A value in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u32 << 24) as f32
    }

    /// A value in `[low, high)`.
    pub fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }
}

/// The free-fly camera.
#[derive(Component, Default)]
pub struct FlyCamera {
    yaw: f32,
    pitch: f32,
    speed: f32,
}

impl FlyCamera {
    /// Start looking along yaw/pitch, in radians, moving at `speed` world units a second.
    pub fn looking(yaw: f32, pitch: f32, speed: f32) -> Self {
        Self { yaw, pitch, speed }
    }
}

impl FlyCamera {
    /// Aim at `look_at` from `eye`, setting the yaw and pitch the fly system writes back to the rotation every frame: a system
    /// that moves the camera on its own must aim it here, or the fly system restores the old aim.
    pub fn aim(&mut self, transform: &mut Transform, eye: Vec3, look_at: Vec3) {
        let dir = (look_at - eye).normalize_or(Vec3::NEG_Z);
        self.yaw = (-dir.x).atan2(-dir.z);
        self.pitch = dir.y.clamp(-1.0, 1.0).asin();
        *transform = Transform::from_translation(eye).with_rotation(Quat::from_euler(
            EulerRot::YXZ,
            self.yaw,
            self.pitch,
            0.0,
        ));
    }
}

/// The harness camera: a `Camera3d` at `eye` looking at `look_at`, flown with WASD when the window
/// has focus.
pub fn camera(eye: Vec3, look_at: Vec3, speed: f32) -> impl Bundle {
    let dir = (look_at - eye).normalize_or(Vec3::NEG_Z);
    let yaw = (-dir.x).atan2(-dir.z);
    let pitch = dir.y.clamp(-1.0, 1.0).asin();
    (
        Camera3d::default(),
        FlyCamera::looking(yaw, pitch, speed),
        Transform::from_translation(eye).with_rotation(Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0)),
    )
}

#[derive(Resource)]
struct RunConfig {
    frames: Option<u32>,
    shot: Option<String>,
    shot_dir: Option<String>,
    shot_every: u32,
    shot_from: u32,
    frame: u32,
    shot_taken: bool,
    /// Screenshots spawned but not yet written; the run does not end over one.
    pending: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl Plugin for ExampleHarness {
    fn build(&self, app: &mut App) {
        let env_u32 = |name: &str, default: u32| {
            std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
        };
        let frames: Option<u32> = std::env::var("LIGHTS_FRAMES").ok().and_then(|v| v.parse().ok());
        let shot = std::env::var("LIGHTS_SHOT").ok();
        let shot_dir = std::env::var("LIGHTS_SHOT_DIR").ok();
        // A frame sequence is only worth reviewing when the frames are the same every run, so
        // either capture switch runs the app on a fixed 1/60 s step.
        if frames.is_some() || shot_dir.is_some() {
            app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / 60.0)));
        }
        app.insert_resource(RunConfig {
            frames,
            shot,
            shot_dir,
            shot_every: env_u32("LIGHTS_SHOT_EVERY", 1).max(1),
            shot_from: env_u32("LIGHTS_SHOT_FROM", 1).max(1),
            frame: 0,
            shot_taken: false,
            pending: std::sync::Arc::default(),
        })
        .add_systems(Update, (fly, capture_frames, run_limit).chain());
    }
}

/// Where the run places its camera, if the example asked for one: `LIGHTS_EYE`/`LIGHTS_LOOK` with the
/// example's own defaults.
pub fn camera_placement(default_eye: Vec3, default_look: Vec3) -> (Vec3, Vec3) {
    (vec3_env("LIGHTS_EYE").unwrap_or(default_eye), vec3_env("LIGHTS_LOOK").unwrap_or(default_look))
}

fn fly(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    time: Res<Time<Real>>,
    mut cameras: Query<(&mut Transform, &mut FlyCamera)>,
) {
    let dt = time.delta_secs();
    for (mut transform, mut fly) in &mut cameras {
        if mouse.pressed(MouseButton::Right) {
            fly.yaw -= motion.delta.x * 0.003;
            fly.pitch = (fly.pitch - motion.delta.y * 0.003).clamp(-1.54, 1.54);
            transform.rotation = Quat::from_euler(EulerRot::YXZ, fly.yaw, fly.pitch, 0.0);
        }
        let mut wish = Vec3::ZERO;
        let forward = transform.forward().as_vec3();
        let right = transform.right().as_vec3();
        for (key, dir) in [
            (KeyCode::KeyW, forward),
            (KeyCode::KeyS, -forward),
            (KeyCode::KeyD, right),
            (KeyCode::KeyA, -right),
            (KeyCode::KeyE, Vec3::Y),
            (KeyCode::KeyQ, -Vec3::Y),
        ] {
            if keys.pressed(key) {
                wish += dir;
            }
        }
        let speed = if keys.pressed(KeyCode::ShiftLeft) { fly.speed * 5.0 } else { fly.speed };
        transform.translation += wish.normalize_or_zero() * speed * dt;
    }
}

fn capture_frames(
    mut commands: Commands,
    config: Res<RunConfig>,
    mut seen: Local<u32>,
    mut index: Local<u32>,
) {
    let Some(dir) = config.shot_dir.clone() else {
        return;
    };
    *seen += 1;
    if *seen < config.shot_from || !(*seen - config.shot_from).is_multiple_of(config.shot_every) {
        return;
    }
    if config.frames.is_some_and(|limit| *seen > limit) {
        return;
    }
    if *index == 0
        && let Err(error) = std::fs::create_dir_all(&dir)
    {
        error!("cannot create frame directory {dir}: {error}");
        return;
    }
    let path = format!("{dir}/frame{:04}.png", *index);
    *index += 1;
    let pending = config.pending.clone();
    pending.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    commands.spawn(Screenshot::primary_window()).observe(move |captured: On<ScreenshotCaptured>| {
        save_to_disk(path.clone())(captured);
        pending.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    });
}

fn run_limit(
    mut config: ResMut<RunConfig>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
    mut waited: Local<u32>,
) {
    config.frame += 1;
    let Some(limit) = config.frames else {
        return;
    };
    if config.frame < limit {
        return;
    }
    // Every capture is written before the app exits; a screenshot takes a frame or two to arrive.
    // Give up after two seconds' worth of frames.
    let pending = config.pending.load(std::sync::atomic::Ordering::Relaxed);
    if pending > 0 {
        *waited += 1;
        if *waited < 120 {
            return;
        }
    }
    match config.shot.clone() {
        Some(path) if !config.shot_taken => {
            config.shot_taken = true;
            let mut save = save_to_disk(path);
            commands.spawn(Screenshot::primary_window()).observe(
                move |captured: On<ScreenshotCaptured>, mut exit: MessageWriter<AppExit>| {
                    exit.write(AppExit::Success);
                    save(captured);
                },
            );
            *waited = 0;
        }
        Some(_) if *waited < 120 => *waited += 1,
        _ => {
            exit.write(AppExit::Success);
        }
    }
}
