//! The CPU cost of the crate's two hot systems, with no window and no GPU.
//!
//! A game that puts a light on every torch needs to know what the pool and the flicker cost per
//! frame at its own scale. This example builds the same app the courtyard does — a focus, a pool of
//! 16, `BENCH_CANDIDATES` candidates and `BENCH_LIGHTS` flickering point lights — runs 600 frames at
//! a fixed 1/60 s step, and prints one line:
//!
//! ```text
//! bench_lights candidates=5000 lights=2000 pool=16 frames=600 mean_us=… p50_us=… min_us=… max_us=…
//! ```
//!
//! `BENCH_CANDIDATES` (default 5000) and `BENCH_LIGHTS` (default 2000) set the scale. Every number is
//! one frame's `App::update()`, so it includes the ECS's own scheduling: compare a run with a change
//! against a run without, on the same machine and the same load, never against another host.
//!
//! **It also measures each system alone.** A total tells you a change cost something; it does not tell you
//! *what*. So after the timed run the two hot systems are timed in isolation — `flicker_lights` over the
//! lights and `assign_pool` over the candidates — each as `p50_us` on the same app, plus the pool's own
//! count of candidates it considered. A change to the pool that moves the total but not the pool's line is
//! not a change to the pool.

use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy_lights::*;

/// The one entity the pool measures from.
#[derive(Component)]
struct Focus;

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// The moving focus: it walks the candidate grid, so a frame's distances are not the previous frame's.
#[derive(Component)]
struct Drifting(Vec3);

fn main() {
    let candidates = env_usize("BENCH_CANDIDATES", 5_000);
    let lights = env_usize("BENCH_LIGHTS", 2_000);
    let frames: usize = env_usize("BENCH_FRAMES", 600);
    let pool_size: usize = env_usize("BENCH_POOL", 16);

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins((TransformPlugin, FlickerPlugin, LightPoolPlugin::<Focus>::default()))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / 60.0)))
        .insert_resource(LightPool { size: pool_size, gain: 1.0, ..default() })
        .add_systems(Update, drift_focus)
        // Each of the crate's sets is switched by `Only` while a per-system timing run is on, and runs
        // normally otherwise. A `Res` in a run condition panics on a missing resource, so it is an
        // `Option`.
        .configure_sets(
            Update,
            LightsSystems::Flicker.run_if(|only: Option<Res<Only>>| only.is_none_or(|o| *o == Only::Flicker)),
        )
        .configure_sets(
            Update,
            LightsSystems::Pool.run_if(|only: Option<Res<Only>>| only.is_none_or(|o| *o == Only::Pool)),
        );

    // The candidates: one per square metre of a square grid, so their distance ordering is not a
    // degenerate sort.
    let side = (candidates as f32).sqrt().ceil() as i32;
    for index in 0..candidates {
        let x = (index as i32 % side) as f32;
        let z = (index as i32 / side) as f32;
        app.world_mut().spawn((
            PoolCandidate { offset: Vec3::Y * 1.8, intensity: 8_000.0, ..default(), ..default() },
            Transform::from_xyz(x, 0.0, z),
        ));
    }
    // The lights the flicker drives: not candidates, so the two systems' costs are separable.
    for index in 0..lights {
        let x = (index as i32 % 64) as f32 * 2.0;
        let z = (index as i32 / 64) as f32 * 2.0;
        app.world_mut().spawn((
            PointLight { intensity: 0.0, range: 8.0, ..default() },
            Flicker { base: 8_000.0, phase: index as f32 * 0.7, diameter: None, ..default(), ..default() },
            Transform::from_xyz(x, 1.8, z),
        ));
    }
    app.world_mut().spawn((Focus, Drifting(Vec3::ZERO), Transform::default()));

    // One frame to build the pool's lights and propagate the transforms; then the timed run.
    app.update();

    let mut samples: Vec<u128> = Vec::with_capacity(frames);
    for _ in 0..frames {
        let start = Instant::now();
        app.update();
        samples.push(start.elapsed().as_micros());
    }

    let mut sorted = samples.clone();
    sorted.sort_unstable();
    let mean = samples.iter().sum::<u128>() / samples.len().max(1) as u128;
    let p50 = sorted[sorted.len() / 2];
    let min = sorted.first().copied().unwrap_or(0);
    let max = sorted.last().copied().unwrap_or(0);
    let pool = app.world().resource::<LightPool>().size;
    println!(
        "bench_lights candidates={candidates} lights={lights} pool={pool} frames={} mean_us={mean} p50_us={p50} min_us={min} max_us={max}",
        samples.len()
    );
    per_system(&mut app, frames.min(300));
}

/// Times each of the crate's two hot **system sets** in the real app, so a line can say which of them a
/// change moved.
///
/// The systems are private (they are internals) and the crate's public boundary is [`LightsSystems`]. So
/// this runs the *same* app twice more with a resource switch that disables one set and leaves the other,
/// and reports each set's `App::update()` median. What is left in a line is that set's work plus the ECS's
/// own scheduling, which is a fixed overhead shared by both lines.
fn per_system(base: &mut App, frames: usize) {
    let median = |mut v: Vec<u128>| {
        v.sort_unstable();
        v.get(v.len() / 2).copied().unwrap_or(0)
    };
    let mut measure = |only: Only| {
        base.insert_resource(only);
        let mut samples: Vec<u128> = Vec::with_capacity(frames);
        for _ in 0..frames {
            let start = Instant::now();
            base.update();
            samples.push(start.elapsed().as_micros());
        }
        median(samples)
    };
    let flicker = measure(Only::Flicker);
    let pool = measure(Only::Pool);
    base.world_mut().remove_resource::<Only>();
    println!("bench_lights_systems flicker_p50_us={flicker} pool_p50_us={pool}");
}

/// Which of the crate's system sets a timing run leaves enabled.
#[derive(Resource, Clone, Copy, PartialEq, Eq)]
enum Only {
    /// The flicker and the kindle.
    Flicker,
    /// The light pool.
    Pool,
}

/// Walks the focus across the grid at 12 m/s, so every frame's assignment differs.
fn drift_focus(time: Res<Time>, mut focus: Query<(&mut Transform, &Drifting)>) {
    let elapsed = time.elapsed_secs();
    for (mut transform, drift) in &mut focus {
        transform.translation = drift.0 + Vec3::new(elapsed * 12.0 % 100.0, 1.2, (elapsed * 4.0) % 100.0);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn app_with(candidates: usize, lights: usize) -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            FlickerPlugin,
            LightPoolPlugin::<Focus>::default(),
        ))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / 60.0)))
        .insert_resource(LightPool { size: 4, gain: 1.0, ..default() });
        for index in 0..candidates {
            app.world_mut().spawn((
                PoolCandidate { offset: Vec3::Y, intensity: 8_000.0, ..default(), ..default() },
                Transform::from_xyz(index as f32, 0.0, 0.0),
            ));
        }
        for index in 0..lights {
            app.world_mut().spawn((
                PointLight { intensity: 0.0, ..default() },
                Flicker { base: 8_000.0, phase: index as f32, diameter: None, ..default(), ..default() },
                Transform::from_xyz(index as f32, 1.8, 0.0),
            ));
        }
        app.world_mut().spawn((Focus, Transform::default()));
        app
    }

    #[test]
    fn the_switch_really_isolates_the_two_sets() {
        // The per-system lines are only meaningful if the switch works: with `Only::Flicker` the pool must
        // not run — its slots stay hidden — and with `Only::Pool` the flicker must not write an intensity.
        let mut app = app_with(8, 8);
        app.configure_sets(
            Update,
            LightsSystems::Flicker.run_if(|only: Option<Res<Only>>| only.is_none_or(|o| *o == Only::Flicker)),
        )
        .configure_sets(
            Update,
            LightsSystems::Pool.run_if(|only: Option<Res<Only>>| only.is_none_or(|o| *o == Only::Pool)),
        );

        app.insert_resource(Only::Flicker);
        app.update();
        app.update();
        let lit_by_flicker: usize = app
            .world_mut()
            .query::<&PointLight>()
            .iter(app.world())
            .filter(|light| light.intensity > 0.0)
            .count();
        assert!(lit_by_flicker > 0, "the flicker wrote intensities");
        let slots = app.world_mut().query::<&PoolLight>().iter(app.world()).count();
        assert_eq!(slots, 0, "and the pool did not run: no slots were spawned");

        let mut app = app_with(8, 8);
        app.configure_sets(
            Update,
            LightsSystems::Flicker.run_if(|only: Option<Res<Only>>| only.is_none_or(|o| *o == Only::Flicker)),
        )
        .configure_sets(
            Update,
            LightsSystems::Pool.run_if(|only: Option<Res<Only>>| only.is_none_or(|o| *o == Only::Pool)),
        );
        app.insert_resource(Only::Pool);
        app.update();
        app.update();
        let slots = app.world_mut().query::<&PoolLight>().iter(app.world()).count();
        // Four assigned slots plus the cross-fade's one reserve light.
        assert_eq!(slots, 5, "the pool ran: its slots and its reserve exist");
        let lit_by_flicker = app
            .world_mut()
            .query_filtered::<&PointLight, Without<PoolLight>>()
            .iter(app.world())
            .filter(|light| light.intensity > 0.0)
            .count();
        assert_eq!(lit_by_flicker, 0, "and the flicker did not write");
    }
}
