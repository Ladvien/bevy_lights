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
        Flicker { base: 1000.0, phase: 0.0 },
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
        Flicker { base: 1000.0, phase: 0.0 },
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
    app.world_mut().spawn((SpotLight { intensity: 0.0, ..default() }, Flicker { base: 5000.0, phase: 1.3 }));
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
            Flicker { base: 1000.0, phase: 0.0 },
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
            Flicker { base: 1000.0, phase: 0.0 },
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
            Flicker { base: 1000.0, phase: 0.0 },
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
    app.world_mut().spawn((PointLight { intensity: 0.0, ..default() }, Flicker { base: 1000.0, phase: 0.0 }));
    app.update();
    let intensity = intensities(&mut app)[0];
    assert!((700.0..=1000.0).contains(&intensity), "no kindle means no flare: {intensity}");
    assert!(heard(&app).is_empty(), "nothing to hear without a kindle");
}
