//! The flicker and the kindle, as a host sees them: intensities written, messages heard, and what a
//! hidden or culled light does.

// Tests are allowed to panic: a failed assertion and a missing fixture are the same thing here, and
// `AGENTS.md`'s no-`unwrap`/`expect`/`panic!` rule is about library code.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::time::Duration;

use bevy::camera::Camera;
use bevy::light::{PointLight, SpotLight};
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy_lights::*;

/// How many [`Kindled`] messages a test's app has heard, by reading them the way a host does.
///
/// `Messages::len()` is not this: a buffered message survives two update cycles, so summing `len()`
/// per frame double-counts. A `MessageReader` sees each message exactly once.
#[derive(Resource, Default)]
struct Heard(Vec<Entity>);

/// The app with no frame run yet: a light spawned into it is present on the very first update, whose
/// delta is zero (a clock has no previous instant to measure from).
fn bare_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, FlickerPlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / 60.0)))
        .init_resource::<Heard>()
        .add_systems(Update, (hear_kindled.after(LightsSystems::Flicker),));
    app
}

/// [`bare_app`] after one frame, so a light spawned into it meets a clock that has a delta.
fn app() -> App {
    let mut app = bare_app();
    app.update();
    app
}

/// Collects every [`Kindled`] the way a host does, so a test counts each message exactly once.
fn hear_kindled(mut heard: ResMut<Heard>, mut kindled: MessageReader<Kindled>) {
    for message in kindled.read() {
        heard.0.push(message.entity);
    }
}

/// A windowless app has no camera, and a light's `ViewVisibility` is then false for everything: the
/// crate only culls a light when there is a camera to cull it from.
fn with_camera(app: &mut App) {
    app.world_mut().spawn(Camera::default());
    app.update();
}

fn heard(app: &App) -> &[Entity] {
    &app.world().resource::<Heard>().0
}

fn intensities(app: &mut App) -> Vec<f32> {
    app.world_mut().query::<&PointLight>().iter(app.world()).map(|light| light.intensity).collect()
}

#[test]
fn a_kindling_light_is_heard_once_and_settles_inside_its_flicker_window() {
    let mut app = app();
    app.world_mut().spawn((
        PointLight { intensity: 0.0, ..default() },
        Flicker { base: 1000.0, phase: 0.0, diameter: None, ..default() },
        Kindle::default(),
    ));
    // Two seconds, more than three times the kindle's length.
    for _ in 0..120 {
        app.update();
    }
    assert_eq!(heard(&app).len(), 1, "one `Kindled` for one kindle");
    let intensity = intensities(&mut app)[0];
    assert!(
        (700.0..=1000.0).contains(&intensity),
        "settled intensity {intensity} is outside the flicker window 700..1000"
    );
}

#[test]
fn a_light_present_on_the_very_first_frame_is_heard_once() {
    // The first update of a fixed-step run has a delta of zero, so a kindle's `elapsed` is still 0
    // after it. Announcing from `elapsed == 0.0` alone announced the same fire again on frame two:
    // the courtyard printed `kindled` twice for each of its four braziers.
    let mut app = bare_app();
    app.world_mut().spawn((
        PointLight { intensity: 0.0, ..default() },
        Flicker { base: 1000.0, phase: 0.0, diameter: None, ..default() },
        Kindle::default(),
    ));
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(heard(&app).len(), 1, "one fire, one `Kindled`: {:?}", heard(&app));
}

#[test]
fn a_spot_light_flickers_too() {
    // The deliberate behaviour change: a wall sconce is a spot light, and it was meant to flicker.
    let mut app = app();
    app.world_mut().spawn((
        SpotLight { intensity: 0.0, ..default() },
        Flicker { base: 5000.0, phase: 1.3, diameter: None, ..default() },
    ));
    let mut seen = Vec::new();
    for _ in 0..12 {
        app.update();
        let intensity = app
            .world_mut()
            .query::<&SpotLight>()
            .iter(app.world())
            .map(|light| light.intensity)
            .next()
            .unwrap_or_default();
        seen.push(intensity);
    }
    let varies = seen.windows(2).any(|pair| pair[0] != pair[1]);
    assert!(varies, "a spot light's intensity never moved: {seen:?}");
    assert!(
        seen.iter().all(|i| (3500.0..=5000.0).contains(i)),
        "every sample is inside 0.70..1.00 of the base: {seen:?}"
    );
}

#[test]
fn a_culled_light_keeps_its_intensity_and_still_kindles() {
    let mut app = app();
    with_camera(&mut app);
    let light = app
        .world_mut()
        .spawn((
            PointLight { intensity: 1234.0, ..default() },
            Flicker { base: 1000.0, phase: 0.0, diameter: None, ..default() },
            Kindle::default(),
            // What a camera's visibility pass leaves on a light it culled: false.
            ViewVisibility::default(),
        ))
        .id();
    assert!(
        !app.world().get::<ViewVisibility>(light).is_some_and(|seen| seen.get()),
        "the fixture's light must be culled for this test to mean anything"
    );
    for _ in 0..120 {
        app.update();
    }
    let intensity = app.world().get::<PointLight>(light).map(|light| light.intensity).unwrap_or_default();
    assert_eq!(intensity, 1234.0, "a culled light is not rewritten");
    let elapsed = app.world().get::<Kindle>(light).map(|kindle| kindle.elapsed).unwrap_or_default();
    assert!(elapsed >= KINDLE_SECONDS, "a culled light's kindle still advances: {elapsed}");
}

#[test]
fn a_hidden_light_is_not_written() {
    // A host that puts a light out with `Visibility::Hidden` is not asking for a flicker.
    let mut app = app();
    let light = app
        .world_mut()
        .spawn((
            PointLight { intensity: 0.0, ..default() },
            Flicker { base: 1000.0, phase: 0.0, diameter: None, ..default() },
            Visibility::Hidden,
        ))
        .id();
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(
        app.world().get::<PointLight>(light).map(|light| light.intensity),
        Some(0.0),
        "a hidden light keeps its intensity"
    );
}

#[test]
fn re_inserting_the_kindle_writes_a_second_message() {
    let mut app = app();
    let light = app
        .world_mut()
        .spawn((
            PointLight { intensity: 0.0, ..default() },
            Flicker { base: 1000.0, phase: 0.0, diameter: None, ..default() },
            Kindle::default(),
        ))
        .id();
    for _ in 0..120 {
        app.update();
    }
    assert_eq!(heard(&app), [light], "the first lighting is heard once");
    app.world_mut().entity_mut(light).insert(Kindle::default());
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(heard(&app), [light, light], "re-lighting the fire is heard again");
}

#[test]
fn a_light_without_a_kindle_reaches_its_plain_flicker_at_once() {
    let mut app = app();
    app.world_mut().spawn((
        PointLight { intensity: 0.0, ..default() },
        Flicker { base: 1000.0, phase: 0.0, diameter: None, ..default() },
    ));
    app.update();
    let intensity = intensities(&mut app)[0];
    assert!((700.0..=1000.0).contains(&intensity), "no kindle means no flare: {intensity}");
    assert!(heard(&app).is_empty(), "nothing to hear without a kindle");
}

#[test]
fn a_sized_fire_flickers_at_the_puffing_laws_rate_and_an_unsized_one_at_the_inherited_rate() {
    // The law is `f = 1.5 / sqrt(D)` Hz. A light with a diameter must move at that rate; a light
    // without one must be unchanged from before this feature existed.
    use bevy_lights::model::{flicker_factor, flicker_factor_of};

    let mut app = App::new();
    app.add_plugins((MinimalPlugins, FlickerPlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / 60.0)));
    let sized = app
        .world_mut()
        .spawn((
            PointLight { intensity: 0.0, ..default() },
            Flicker { base: 1000.0, phase: 0.0, diameter: Some(0.34), ..default() },
        ))
        .id();
    let plain = app
        .world_mut()
        .spawn((
            PointLight { intensity: 0.0, ..default() },
            Flicker { base: 1000.0, phase: 0.0, diameter: None, ..default() },
        ))
        .id();
    app.update();
    let t = app.world().resource::<Time>().elapsed_secs();
    let read = |app: &App, entity: Entity| {
        app.world().get::<PointLight>(entity).map(|light| light.intensity).unwrap_or(0.0)
    };
    assert!(
        (read(&app, sized) - 1000.0 * flicker_factor_of(0.34, t, 0.0)).abs() < 1e-2,
        "a sized fire must follow the law's rate: {} against {}",
        read(&app, sized),
        1000.0 * flicker_factor_of(0.34, t, 0.0)
    );
    assert!(
        (read(&app, plain) - 1000.0 * flicker_factor(t, 0.0)).abs() < 1e-2,
        "an unsized fire must be the inherited shape: {} against {}",
        read(&app, plain),
        1000.0 * flicker_factor(t, 0.0)
    );
}

/// A deeper fire dips further and never rises above its base; a depth of zero holds the base still.
/// A stray depth cannot drive a light below nothing.
#[test]
fn a_deeper_fire_dips_further_but_never_exceeds_its_base() {
    let mut app = app();
    let spawn = |app: &mut App, depth: f32| {
        app.world_mut()
            .spawn((
                PointLight { intensity: 0.0, ..default() },
                Flicker { base: 1000.0, phase: 0.0, diameter: None, depth },
            ))
            .id()
    };
    let still = spawn(&mut app, 0.0);
    let calm = spawn(&mut app, 0.03);
    let deep = spawn(&mut app, 0.4);
    let wild = spawn(&mut app, 9.0);
    let mut low = [f32::MAX; 4];
    let mut high = [f32::MIN; 4];
    for _ in 0..300 {
        app.update();
        for (i, id) in [still, calm, deep, wild].into_iter().enumerate() {
            let v = app.world().get::<PointLight>(id).unwrap().intensity;
            low[i] = low[i].min(v);
            high[i] = high[i].max(v);
        }
    }
    assert!(
        (low[0] - 1000.0).abs() < 1e-2 && (high[0] - 1000.0).abs() < 1e-2,
        "depth 0 is still: {low:?} {high:?}"
    );
    assert!(low[1] > low[2] && low[2] > 0.0, "a deeper fire dips further: {low:?}");
    for (i, h) in high.iter().enumerate() {
        assert!(*h <= 1000.0 + 1e-2, "fire {i} rose above its base: {h}");
    }
    assert!(low[3] >= 0.0, "a stray depth must not go negative: {}", low[3]);
}

/// A lumen budget written against `flicker_mean` holds at any depth: the light a fire actually puts out
/// averages `base · flicker_mean(depth)`, so a host solves for `base` instead of dividing by `0.85`.
#[test]
fn a_budget_solved_through_flicker_mean_is_the_light_the_fire_averages() {
    for depth in [CALM_FLICKER_DEPTH, FLICKER_DEPTH, WHIPPING_FLICKER_DEPTH] {
        assert!((0.0..=MAX_FLICKER_DEPTH).contains(&depth));
        let mut app = app();
        let budget = 7900.0;
        let light = app
            .world_mut()
            .spawn((
                PointLight { intensity: 0.0, ..default() },
                Flicker { base: budget / flicker_mean(depth), phase: 0.4, diameter: Some(0.3), depth },
            ))
            .id();
        let (mut sum, frames) = (0.0, 3000);
        for _ in 0..frames {
            app.update();
            sum += app.world().get::<PointLight>(light).unwrap().intensity;
        }
        let mean = sum / frames as f32;
        assert!((mean / budget - 1.0).abs() < 0.03, "depth {depth}: mean {mean} against budget {budget}");
    }
}

/// The time-average of what a fire's light does, from the crate's own swing, over 300 s at 50 Hz.
fn averaged(fire: &Flicker) -> f32 {
    let n = 15_000;
    (0..n)
        .map(|i| fire.base * flicker_swing(fire.diameter, fire.depth, i as f32 / 50.0, fire.phase))
        .sum::<f32>()
        / n as f32
}

/// A fire built at its mean averages that mean at every named depth, and changing its depth keeps it.
#[test]
fn a_fire_built_at_its_mean_averages_it_and_keeps_it_through_a_depth_change() {
    for depth in [CALM_FLICKER_DEPTH, FLICKER_DEPTH, WHIPPING_FLICKER_DEPTH] {
        let mut fire = Flicker { phase: 0.9, diameter: Some(0.34), ..Flicker::at_mean(1000.0, depth) };
        assert!((fire.base * flicker_mean(depth) / 1000.0 - 1.0).abs() < 1e-4);
        assert!((averaged(&fire) / 1000.0 - 1.0).abs() < 0.01, "depth {depth}");
        for next in [CALM_FLICKER_DEPTH, WHIPPING_FLICKER_DEPTH, MAX_FLICKER_DEPTH] {
            fire.set_depth(next);
            assert!((averaged(&fire) / 1000.0 - 1.0).abs() < 0.01, "{depth} -> {next}");
        }
    }
}

/// A put-out fire stays out through `set_depth`; a stray depth is clamped and leaves a finite base; a
/// per-frame `set_mean` follows the fire's current depth.
#[test]
fn set_depth_and_set_mean_hold_at_the_edges() {
    let mut out = Flicker::at_mean(0.0, FLICKER_DEPTH);
    out.set_depth(WHIPPING_FLICKER_DEPTH);
    assert_eq!(out.base, 0.0);
    let mut fire = Flicker::at_mean(500.0, FLICKER_DEPTH);
    for stray in [9.0, -3.0, f32::NAN] {
        fire.set_depth(stray);
        assert!(fire.base.is_finite() && (0.0..=MAX_FLICKER_DEPTH).contains(&fire.depth));
        assert!((fire.mean() - 500.0).abs() < 1e-2, "{stray}: {}", fire.mean());
    }
    fire.set_depth(WHIPPING_FLICKER_DEPTH);
    fire.set_mean(1200.0);
    assert!((averaged(&fire) / 1200.0 - 1.0).abs() < 0.01);
}

/// A host writes `Flicker { base, phase, ..default() }` and nothing more: the spelling the crate
/// documents as stable. If `Flicker` ever became `#[non_exhaustive]` this stops compiling, and the
/// fields a host did not name keep their defaults.
#[test]
fn a_host_literal_naming_only_base_and_phase_takes_the_defaults() {
    let f = Flicker { base: 1000.0, phase: 1.3, ..default() };
    assert_eq!(f.diameter, None);
    assert_eq!(f.depth, FLICKER_DEPTH);
}

/// Every [`Snuffed`] a test's app has heard.
#[derive(Resource, Default)]
struct Out(Vec<Entity>);

fn hear_snuffed(mut out: ResMut<Out>, mut snuffed: MessageReader<Snuffed>) {
    out.0.extend(snuffed.read().map(|message| message.entity));
}

#[test]
fn a_snuffed_light_falls_to_exactly_nothing_is_heard_once_and_can_be_lit_again() {
    let mut app = app();
    app.init_resource::<Out>().add_systems(Update, hear_snuffed.after(LightsSystems::Flicker));
    let light = app
        .world_mut()
        .spawn((
            PointLight { intensity: 0.0, ..default() },
            Flicker { base: 1000.0, phase: 0.0, diameter: None, depth: 0.0, ..default() },
        ))
        .id();
    app.update();
    assert!((intensities(&mut app)[0] - 1000.0).abs() < 1e-2, "a plain fire burns at its base");

    app.world_mut().entity_mut(light).insert(Snuff::default());
    let mut last = f32::MAX;
    for _ in 0..(SNUFF_SECONDS * 60.0) as usize + 5 {
        app.update();
        let now = intensities(&mut app)[0];
        assert!(now <= last + 1e-3, "a dying fire never brightens: {last} -> {now}");
        last = now;
    }
    assert_eq!(last, 0.0, "out is exactly out");
    assert_eq!(app.world().resource::<Out>().0, [light], "announced once, for this light");

    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<Out>().0.len(), 1, "an extinguished fire says nothing more");

    app.world_mut().entity_mut(light).remove::<Snuff>().insert(Kindle::default());
    for _ in 0..((KINDLE_SECONDS * 60.0) as usize + 3) {
        app.update();
    }
    assert!((intensities(&mut app)[0] - 1000.0).abs() < 1e-2, "removing Snuff and kindling lights it again");
}

#[test]
fn a_snuff_beats_a_kindle_and_a_loaded_out_fire_says_nothing() {
    let mut app = app();
    app.init_resource::<Out>().add_systems(Update, hear_snuffed.after(LightsSystems::Flicker));
    app.world_mut().spawn((
        PointLight { intensity: 500.0, ..default() },
        Flicker { base: 1000.0, phase: 0.0, diameter: None, depth: 0.0, ..default() },
        Kindle::default(),
        Snuff { elapsed: SNUFF_SECONDS },
    ));
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(intensities(&mut app)[0], 0.0, "put out wins over kindling");
    assert!(app.world().resource::<Out>().0.is_empty(), "a fire that loaded as out is not announced");
}
