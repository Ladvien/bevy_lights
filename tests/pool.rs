//! The pool, as a host sees it: which slots are lit, where they sit, how bright, and every way the
//! assignment can collapse (no focus, two foci, a hidden candidate, a gain of zero).

// Tests are allowed to panic: a failed assertion and a missing fixture are the same thing here, and
// `AGENTS.md`'s no-`unwrap`/`expect`/`panic!` rule is about library code.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy_lights::*;

/// The focus marker the tests measure from. A host would use its camera focus; here it is a bare
/// component, which is the point of the plugin's generic.
#[derive(Component)]
struct TestFocus;

fn app(pool: LightPool) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        FlickerPlugin,
        LightPoolPlugin::<TestFocus>::default(),
    ))
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / 60.0)))
    // Written over the plugin's own `init_resource` default, which is what a host does too.
    .insert_resource(pool);
    app.update();
    app
}

fn focus_at(app: &mut App, at: Vec3) {
    app.world_mut().spawn((TestFocus, Transform::from_translation(at)));
    settle(app);
}

/// Every pool light, by slot: `(slot, translation, visibility, flicker base)`.
fn slots(app: &mut App) -> Vec<(usize, Vec3, Visibility, f32)> {
    let mut found: Vec<(usize, Vec3, Visibility, f32)> = app
        .world_mut()
        .query::<(&PoolLight, &Transform, &Visibility, &Flicker)>()
        .iter(app.world())
        .map(|(light, transform, visibility, flicker)| {
            (light.slot, transform.translation, *visibility, flicker.base)
        })
        .collect();
    found.sort_by_key(|(slot, ..)| *slot);
    found
}

/// A candidate at `at` (on the ground), its light 2 m up, burning at `intensity`.
///
/// The pool reads a candidate's world position from its `GlobalTransform`, which Bevy propagates in
/// `PostUpdate`: a candidate spawned in a frame is placed by the pool on the *next* one. Every test
/// that spawns a candidate therefore updates once before asserting, which is also what a host sees.
fn candidate(app: &mut App, at: Vec3, intensity: f32) -> Entity {
    app.world_mut()
        .spawn((PoolCandidate { offset: Vec3::Y * 2.0, intensity }, Transform::from_translation(at)))
        .id()
}

/// Lets the transforms propagate, then runs one pool frame.
fn settle(app: &mut App) {
    app.update();
    app.update();
}

#[test]
fn the_nearest_candidates_get_the_lights_at_their_offsets() {
    let mut app = app(LightPool { size: 3, reach: 10.0, gain: 0.5, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    for x in 1..=5 {
        candidate(&mut app, Vec3::new(x as f32, 0.0, 0.0), 100.0);
    }
    settle(&mut app);
    let lit = slots(&mut app);
    assert_eq!(lit.len(), 3, "three slots exist: {lit:?}");
    for (slot, at, visibility, base) in &lit {
        let x = *slot as f32 + 1.0;
        assert!(
            (at.x - x).abs() < 1e-5 && (at.y - 2.0).abs() < 1e-5,
            "slot {slot} sits at {at}, wanted ({x}, 2, 0)"
        );
        assert_eq!(*visibility, Visibility::Inherited, "slot {slot} is shown");
        assert!((base - 50.0).abs() < 1e-4, "slot {slot} burns at {base}, wanted 100 · 0.5");
    }
}

#[test]
fn a_hidden_or_distant_candidate_is_skipped() {
    let mut app = app(LightPool { size: 2, reach: 10.0, gain: 1.0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
    let hidden = candidate(&mut app, Vec3::new(2.0, 0.0, 0.0), 100.0);
    // Beyond the reach, so it is never in the list however near it is in the other axis.
    candidate(&mut app, Vec3::new(40.0, 0.0, 0.0), 100.0);
    app.world_mut().entity_mut(hidden).insert(Visibility::Hidden);
    settle(&mut app);
    let lit = slots(&mut app);
    assert_eq!(lit[0].2, Visibility::Inherited, "the near candidate is lit");
    assert_eq!(
        lit[1].2,
        Visibility::Hidden,
        "the hidden candidate gets no light, and the far one is out of reach"
    );
    assert!((lit[0].1.x - 1.0).abs() < 1e-5, "the lit slot is on the visible candidate: {:?}", lit[0]);
}

#[test]
fn a_zero_gain_hides_the_whole_pool() {
    let mut app = app(LightPool { size: 2, reach: 10.0, gain: 0.0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
    settle(&mut app);
    for (slot, _, visibility, _) in slots(&mut app) {
        assert_eq!(visibility, Visibility::Hidden, "slot {slot} is out at gain 0");
    }
}

#[test]
fn no_focus_means_no_lights() {
    let mut app = app(LightPool { size: 2, reach: 10.0, gain: 1.0, ..default() });
    candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
    settle(&mut app);
    for (slot, _, visibility, _) in slots(&mut app) {
        assert_eq!(visibility, Visibility::Hidden, "slot {slot} has no focus to be near");
    }
}

#[test]
fn two_foci_mean_no_lights() {
    let mut app = app(LightPool { size: 2, reach: 10.0, gain: 1.0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    app.world_mut().spawn((TestFocus, Transform::from_xyz(5.0, 0.0, 0.0)));
    candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
    settle(&mut app);
    for (slot, _, visibility, _) in slots(&mut app) {
        assert_eq!(visibility, Visibility::Hidden, "slot {slot} has two foci to be near, which is no focus");
    }
}

#[test]
fn shrinking_the_pool_despawns_the_extra_lights() {
    let mut app = app(LightPool { size: 3, reach: 10.0, gain: 1.0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    for x in 1..=5 {
        candidate(&mut app, Vec3::new(x as f32, 0.0, 0.0), 100.0);
    }
    settle(&mut app);
    assert_eq!(slots(&mut app).len(), 3);
    app.world_mut().resource_mut::<LightPool>().size = 1;
    app.update();
    app.update();
    let remaining = slots(&mut app);
    assert_eq!(remaining.len(), 1, "one light left: {remaining:?}");
    assert_eq!(remaining[0].0, 0, "the survivor is slot 0");
}

#[test]
fn two_candidates_at_one_distance_are_assigned_in_entity_order() {
    let mut app = app(LightPool { size: 1, reach: 10.0, gain: 1.0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    // Equal distance, opposite sides: only `Entity` breaks the tie, and it must do so the same way
    // every frame.
    let first = candidate(&mut app, Vec3::new(2.0, 0.0, 0.0), 100.0);
    candidate(&mut app, Vec3::new(-2.0, 0.0, 0.0), 100.0);
    settle(&mut app);
    let wanted =
        app.world().get::<Transform>(first).map(|transform| transform.translation).unwrap_or_default();
    let lit = slots(&mut app);
    let first_at = lit[0].1;
    // The pool's slot sits at the chosen candidate's own position plus the 2 m offset.
    assert!(
        (first_at - (wanted + Vec3::Y * 2.0)).abs().length() < 1e-5
            || (first_at - (-wanted + Vec3::Y * 2.0)).abs().length() < 1e-5,
        "the light is on one of the tied candidates: {first_at}"
    );
    // And the same one, frame after frame.
    for _ in 0..5 {
        app.update();
        assert_eq!(slots(&mut app)[0].1, first_at, "the tie-break must not move between frames");
    }
}

#[test]
fn the_pools_light_settings_follow_the_resource() {
    let mut app = app(LightPool { size: 1, reach: 10.0, gain: 1.0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
    settle(&mut app);
    {
        let mut pool = app.world_mut().resource_mut::<LightPool>();
        pool.light.color = Color::srgb(1.0, 0.2, 0.1);
        pool.light.range = 21.0;
        pool.light.shadow_maps_enabled = true;
    }
    app.update();
    let (colour, range, shadows) = app
        .world_mut()
        .query_filtered::<&PointLight, With<PoolLight>>()
        .iter(app.world())
        .map(|light| (light.color, light.range, light.shadow_maps_enabled))
        .next()
        .expect("a pool light");
    assert_eq!(colour, Color::srgb(1.0, 0.2, 0.1), "the colour followed");
    assert!((range - 21.0).abs() < 1e-5, "the range followed: {range}");
    assert!(shadows, "the shadow setting followed");
}

#[test]
fn two_slots_never_flicker_in_step() {
    let mut app = app(LightPool { size: 4, reach: 100.0, gain: 1.0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    for x in 1..=4 {
        candidate(&mut app, Vec3::new(x as f32, 0.0, 0.0), 100.0);
    }
    settle(&mut app);
    let phases: Vec<f32> = app
        .world_mut()
        .query_filtered::<&Flicker, With<PoolLight>>()
        .iter(app.world())
        .map(|flicker| flicker.phase)
        .collect();
    assert_eq!(phases.len(), 4);
    for (index, phase) in phases.iter().enumerate() {
        assert!((phase - 1.3 * index as f32).abs() < 1e-5, "slot {index}'s phase is {phase}");
    }
}
