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
        .spawn((
            PoolCandidate { offset: Vec3::Y * 2.0, intensity, ..default() },
            Transform::from_translation(at),
        ))
        .id()
}

/// The lights the pool assigns candidates to. A reserve light (the cross-fade's departing owner) has a
/// slot at or past `LightPool::size` and is never assigned, so it is not a slot here.
fn assigned(app: &mut App, size: usize) -> Vec<(usize, Vec3, Visibility, f32)> {
    slots(app).into_iter().filter(|(slot, ..)| *slot < size).collect()
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
    let lit = assigned(&mut app, 3);
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
    let lit = assigned(&mut app, 2);
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
    assert_eq!(assigned(&mut app, 3).len(), 3);
    app.world_mut().resource_mut::<LightPool>().size = 1;
    app.update();
    app.update();
    let remaining = assigned(&mut app, 1);
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
fn a_change_of_gain_leaves_a_slots_own_colour_alone() {
    // A host that tints its own fire lights (a palette recolouring everything carrying its marker)
    // and writes `gain` every frame must keep its tint: only a change to the template's own look
    // resets the slots.
    let mut app = app(LightPool { size: 1, reach: 10.0, gain: 1.0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
    settle(&mut app);
    let tint = Color::srgb(0.9, 0.3, 0.1);
    for mut light in
        app.world_mut().query_filtered::<&mut PointLight, With<PoolLight>>().iter_mut(app.world_mut())
    {
        light.color = tint;
    }
    for gain in [0.9, 0.7, 0.4, 1.0] {
        app.world_mut().resource_mut::<LightPool>().gain = gain;
        app.update();
        let colour = app
            .world_mut()
            .query_filtered::<&PointLight, With<PoolLight>>()
            .iter(app.world())
            .map(|light| light.color)
            .next()
            .expect("a pool light");
        assert_eq!(colour, tint, "gain {gain} reset the slot's own colour");
    }
}

#[test]
fn two_slots_never_flicker_in_step() {
    let mut app = app(LightPool { size: 4, reach: 100.0, gain: 1.0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    for x in 1..=4 {
        candidate(&mut app, Vec3::new(x as f32, 0.0, 0.0), 100.0);
    }
    settle(&mut app);
    let mut phases: Vec<(usize, f32)> = app
        .world_mut()
        .query_filtered::<(Entity, &PoolLight, &Flicker), ()>()
        .iter(app.world())
        .map(|(_, light, flicker)| (light.slot, flicker.phase))
        .collect();
    phases.retain(|(slot, _)| *slot < 4);
    phases.sort_by_key(|(slot, _)| *slot);
    let phases: Vec<f32> = phases.into_iter().map(|(_, phase)| phase).collect();
    assert_eq!(phases.len(), 4);
    for (index, phase) in phases.iter().enumerate() {
        assert!((phase - 1.3 * index as f32).abs() < 1e-5, "slot {index}'s phase is {phase}");
    }
}

// ---- The hand-off: a steady pool stays steady, and a light that changes owner fades in. ----

/// Moves a candidate: its own `Transform`, which the pool reads the frame the host writes it.
fn move_candidate(app: &mut App, entity: Entity, to: Vec3) {
    app.world_mut().entity_mut(entity).insert(Transform::from_translation(to));
}

#[test]
fn a_steady_pool_keeps_its_candidates_and_its_brightness() {
    // The light a torch holds must not chatter: with nothing moving, two frames later each slot is on
    // the same candidate at the same base.
    let mut app = app(LightPool { size: 2, reach: 100.0, gain: 1.0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    let near = candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
    let far = candidate(&mut app, Vec3::new(2.0, 0.0, 0.0), 100.0);
    settle(&mut app);
    let first = slots(&mut app);
    assert!((first[0].3 - 100.0).abs() < 1e-4, "slot 0 settled at {}", first[0].3);
    for _ in 0..5 {
        app.update();
    }
    let later = slots(&mut app);
    assert_eq!(first, later, "a steady pool changed: {first:?} then {later:?}");
    assert!((later[0].1.x - 1.0).abs() < 1e-5 && (later[1].1.x - 2.0).abs() < 1e-5);
    assert!((later[0].3 - 100.0).abs() < 1e-4 && (later[1].3 - 100.0).abs() < 1e-4);
    let _ = (near, far);
}

#[test]
fn a_slot_that_changes_owner_fades_in_from_nothing() {
    // Slot 0 holds the torch at 1 m. When that torch walks out to 9 m and another sits at 0.5 m, the
    // slot takes the nearer one — and its light comes back over FADE_SECONDS rather than snapping.
    let mut app = app(LightPool { size: 1, reach: 100.0, gain: 1.0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    let walker = candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
    settle(&mut app);
    let settled = slots(&mut app);
    assert!((settled[0].1.x - 1.0).abs() < 1e-5, "slot 0 starts on the walker");
    assert!((settled[0].3 - 100.0).abs() < 1e-4, "and at full brightness");

    // A challenger much nearer, and the held one out of the way: this is a hand-off by any rule.
    candidate(&mut app, Vec3::new(0.2, 0.0, 0.0), 100.0);
    move_candidate(&mut app, walker, Vec3::new(9.0, 0.0, 0.0));
    app.update();
    let (_, at, _, base) = slots(&mut app)[0];
    assert!((at.x - 0.2).abs() < 1e-5, "slot 0 moved to the challenger, at {at}");
    assert!(base < 1.0, "the new light starts dark and fades in, not at 100: {base}");

    // Within the fade it is back to full: 0.35 s is 21 frames at 60 Hz.
    for _ in 0..24 {
        app.update();
    }
    let (_, _, _, base) = slots(&mut app)[0];
    assert!((base - 100.0).abs() < 1e-3, "the fade did not finish: {base}");
}

#[test]
fn a_challenger_that_is_only_slightly_nearer_does_not_take_the_light() {
    // The overlap rule: 1.15 hysteresis means a challenger must be ~13 % nearer. 1.0 m held against
    // 0.95 m free is a challenge of 5 %, so the held torch keeps its light and nothing fades.
    let mut app = app(LightPool { size: 1, reach: 100.0, gain: 1.0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    let held = candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
    settle(&mut app);
    let near_but_not_enough = candidate(&mut app, Vec3::new(0.95, 0.0, 0.0), 100.0);
    app.update();
    app.update();
    let (_, at, _, base) = slots(&mut app)[0];
    assert!((at.x - 1.0).abs() < 1e-5, "the held torch kept the light, slot at {at}");
    assert!((base - 100.0).abs() < 1e-3, "and did not dim: {base}");

    // Move the challenger clearly nearer and the light is handed over.
    move_candidate(&mut app, near_but_not_enough, Vec3::new(0.5, 0.0, 0.0));
    app.update();
    let (_, at, _, base) = slots(&mut app)[0];
    assert!((at.x - 0.5).abs() < 1e-5, "a challenger twice as near took the light, slot at {at}");
    assert!(base < 100.0, "and the new owner faded in: {base}");
    let _ = held;
}

#[test]
fn a_held_candidate_that_goes_hands_the_light_on_at_once() {
    // Despawned: there is no overlap to hold, so the slot takes the nearest survivor the same frame.
    let mut app = app(LightPool { size: 1, reach: 100.0, gain: 1.0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    let held = candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
    settle(&mut app);
    candidate(&mut app, Vec3::new(2.0, 0.0, 0.0), 100.0);
    app.world_mut().entity_mut(held).despawn();
    app.update();
    let (_, at, _, base) = slots(&mut app)[0];
    assert!((at.x - 2.0).abs() < 1e-5, "the survivor got the light, slot at {at}");
    assert!(base < 1.0, "as a hand-off, so it fades in: {base}");
}

#[test]
fn a_hand_off_costs_less_than_a_pop_and_recovers_within_the_fade() {
    // The craft checklist's own line, isolated, **with no reserve**: the slot keeps its intensity and
    // gains a fade factor, so the total dips by that one light's share and is whole again by the end of
    // the fade. A pop would keep the total and move the light. (With a reserve the same hand-off
    // cross-fades instead — that is `a_cross_faded_hand_off_holds_the_subject_and_a_dip_does_not`.)
    let mut app = app(LightPool { size: 2, reach: 100.0, gain: 1.0, reserve: 0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    let walker = candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
    candidate(&mut app, Vec3::new(1.5, 0.0, 0.0), 100.0);
    settle(&mut app);
    let total = |app: &mut App| -> f32 { slots(app).iter().map(|(_, _, _, base)| base).sum() };

    let settled = total(&mut app);
    assert!((settled - 200.0).abs() < 1e-3, "both slots at full brightness: {settled}");

    // The held torch walks away and a newcomer arrives much nearer: one slot changes owner.
    move_candidate(&mut app, walker, Vec3::new(30.0, 0.0, 0.0));
    candidate(&mut app, Vec3::new(0.2, 0.0, 0.0), 100.0);
    app.update();
    let after_pop = total(&mut app);
    assert!(
        after_pop >= settled - 100.0 && after_pop < settled - 1.0,
        "the total dips by the one hand-off light and no more: {settled} -> {after_pop}"
    );

    // And it is whole again inside the fade.
    for _ in 0..24 {
        app.update();
    }
    let recovered = total(&mut app);
    assert!((recovered - settled).abs() < 1e-2, "the total did not recover: {recovered} against {settled}");
}

#[test]
fn a_cross_faded_hand_off_holds_the_subject_and_a_dip_does_not() {
    // The craft's overlap rule, measured: with a reserve light the departing owner fades out while the
    // arriving one fades in, so the pair's total stays where it was. Without it the total dips.
    fn run(reserve: usize) -> Vec<f32> {
        let mut app = app(LightPool { size: 1, reach: 100.0, gain: 1.0, reserve, ..default() });
        focus_at(&mut app, Vec3::ZERO);
        let walker = candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
        settle(&mut app);
        // A challenger twice as near takes the light, and the walker leaves the pool entirely.
        candidate(&mut app, Vec3::new(0.4, 0.0, 0.0), 100.0);
        app.world_mut().entity_mut(walker).despawn();
        let mut totals = Vec::new();
        for _ in 0..6 {
            app.update();
            // Every light the pool owns is on the walker's old spot or the challenger's: their sum is
            // the subject's total light, which is what a cross-fade is meant to keep constant.
            let total: f32 = slots(&mut app)
                .iter()
                .map(|(_, _, visibility, base)| if *visibility == Visibility::Hidden { 0.0 } else { *base })
                .sum();
            totals.push(total);
        }
        totals
    }
    let cross = run(1);
    let dip = run(0);
    // The cross-fade's first frames keep more light on the subject than the dip does.
    assert!(
        cross[0] > dip[0] + 1.0,
        "a cross-fade keeps the departing light on: {:?} against {:?}",
        cross,
        dip
    );
    // And it is never *over* the settled total: the two fades sum to one, they do not add.
    let settled = 100.0;
    for total in &cross {
        assert!(*total <= settled + 1.0, "a cross-fade invented brightness: {total} against {settled}");
    }
    assert!(cross[0] >= settled * 0.9, "the subject holds through the hand-off: {:?}", cross);
}

#[test]
fn the_pool_forgets_a_candidate_that_has_gone() {
    // The remembered positions are pruned to the candidates that still exist, or the map would grow by
    // one entry per candidate ever pooled — a leak in a process that runs for hours.
    let mut app = app(LightPool { size: 2, reach: 100.0, gain: 1.0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    let mut spawned = Vec::new();
    for index in 0..40 {
        spawned.push(candidate(&mut app, Vec3::new(1.0 + index as f32 * 0.1, 0.0, 0.0), 100.0));
        settle(&mut app);
    }
    for entity in spawned.iter().take(35) {
        app.world_mut().entity_mut(*entity).despawn();
    }
    for _ in 0..4 {
        app.update();
    }
    // The pool still works after the churn: two slots on the surviving candidates.
    assert_eq!(assigned(&mut app, 2).len(), 2, "the pool still works after the churn");
    // And it churned without failing: 40 candidates came and went, and the pool's assignment is still
    // the nearest. (The remembered map's size is private; what a test can see is that the pool keeps
    // working after more candidates than its slots ever held, which the prune is what makes safe.)
    let remaining = assigned(&mut app, 2);
    assert!(
        remaining.iter().all(|(_, at, visibility, _)| { *visibility == Visibility::Inherited && at.y > 1.0 }),
        "the survivors are lit: {remaining:?}"
    );
}

#[test]
fn the_two_ends_of_a_cross_fade_keep_the_same_flicker() {
    // A cross-fade's departing light must move exactly as the light that left did. If it keeps the
    // reserve's own phase instead, the two ends beat against each other and their sum wobbles — which a
    // test of the mean cannot see, because the means are equal. So this one compares the phases.
    let mut app = app(LightPool { size: 1, reach: 100.0, gain: 1.0, reserve: 1, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    let walker = candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
    settle(&mut app);

    // Slot 0's own phase: the one the departing light must carry. (The reserve is slot 1, and its phase
    // is its own — that is the bug this test is for.)
    let phase_of = |app: &mut App, slot: usize| -> f32 {
        app.world_mut()
            .query_filtered::<(&PoolLight, &Flicker), ()>()
            .iter(app.world())
            .find(|(light, _)| light.slot == slot)
            .map(|(_, flicker)| flicker.phase)
            .unwrap_or(f32::NAN)
    };
    let held_phase = phase_of(&mut app, 0);
    assert!(held_phase.is_finite(), "slot 0 exists");

    // Displace the holder: the reserve fades it out, and it must be at slot 0's phase.
    candidate(&mut app, Vec3::new(0.4, 0.0, 0.0), 100.0);
    app.world_mut().entity_mut(walker).despawn();
    app.update();

    let reserve = app
        .world_mut()
        .query_filtered::<(&PoolLight, &Flicker), ()>()
        .iter(app.world())
        .find(|(light, _)| light.slot == 1)
        .map(|(_, flicker)| (flicker.phase, flicker.base))
        .unwrap_or((f32::NAN, 0.0));
    assert!(reserve.1 > 0.0, "the reserve light is fading a departure out: {reserve:?}");
    assert!(
        (reserve.0 - held_phase).abs() < 1e-5,
        "the departing light must keep the phase it had ({held_phase}), not the reserve's ({})",
        reserve.0
    );
}

#[test]
fn a_departure_is_detected_at_all() {
    let mut app = app(LightPool { size: 1, reach: 100.0, gain: 1.0, reserve: 1, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    let walker = candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
    settle(&mut app);
    let reserve_before = app
        .world_mut()
        .query_filtered::<(&PoolLight, &Flicker), ()>()
        .iter(app.world())
        .find(|(light, _)| light.slot == 1)
        .map(|(_, flicker)| flicker.base)
        .unwrap_or(f32::NAN);
    assert_eq!(reserve_before, 0.0, "the reserve is dark before the hand-off");

    // The holder leaves, and no new candidate arrives: the slot goes empty.
    app.world_mut().entity_mut(walker).despawn();
    app.update();
    let reserve_after = app
        .world_mut()
        .query_filtered::<(&PoolLight, &Flicker), ()>()
        .iter(app.world())
        .find(|(light, _)| light.slot == 1)
        .map(|(_, flicker)| flicker.base)
        .unwrap_or(f32::NAN);
    assert!(
        reserve_after > 0.0,
        "a departure with no replacement must still fade out through the reserve: {reserve_after}"
    );
}

#[test]
fn a_child_candidate_moves_with_its_parent() {
    // A torch is a child of its bearer: it has its own `Transform` (a constant lift) and a
    // `GlobalTransform` the parent places. Reading its local translation instead of its world position
    // puts every child candidate at the same point, so the pool never re-sorts and nothing ever leaves —
    // which is the bug this test is for.
    let mut app = app(LightPool { size: 1, reach: 100.0, gain: 1.0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    let bearer = app.world_mut().spawn(Transform::from_xyz(1.0, 0.85, 0.0)).id();
    let torch = app
        .world_mut()
        .spawn((
            PoolCandidate { offset: Vec3::Y, intensity: 100.0, ..default() },
            Transform::from_xyz(0.0, 1.0, 0.0),
        ))
        .id();
    app.world_mut().entity_mut(torch).insert(ChildOf(bearer));
    settle(&mut app);
    let first = assigned(&mut app, 1)[0].1;
    assert!(
        (first.x - 1.0).abs() < 1e-4,
        "the child's light is at its parent's place, not the origin: {first}"
    );

    // The bearer walks, and the child's light follows it.
    app.world_mut().entity_mut(bearer).insert(Transform::from_xyz(7.0, 0.85, 0.0));
    app.update();
    app.update();
    let moved = assigned(&mut app, 1)[0].1;
    assert!((moved.x - 7.0).abs() < 1e-4, "the child's light followed its parent to {moved}");
}

#[test]
fn a_brighter_light_outranks_a_nearer_dim_one() {
    // The rule the pool used to have ranked by distance alone, so a torch two metres away took a real
    // light from a brazier three metres away that puts 25 times the light on the subject. The rank is now
    // the illuminance the candidate puts on the subject, `intensity / distance²`.
    let torch_lumens = 8_000.0;
    let brazier_lumens = 60_000.0;
    for (rank, expected) in [(Rank::Illuminance, "brazier"), (Rank::Distance, "torch")] {
        let mut app = app(LightPool { size: 1, reach: 100.0, gain: 1.0, rank, ..default() });
        focus_at(&mut app, Vec3::ZERO);
        let torch = candidate(&mut app, Vec3::new(2.0, 0.0, 0.0), torch_lumens);
        let brazier = candidate(&mut app, Vec3::new(3.0, 0.0, 0.0), brazier_lumens);
        settle(&mut app);
        let (_, at, _, base) = assigned(&mut app, 1)[0];
        let wants = if expected == "brazier" { brazier_lumens } else { torch_lumens };
        assert!(
            (base - wants).abs() < 1e-3,
            "{rank:?} should give the light to the {expected}: base {base} at {at}"
        );
        let _ = (torch, brazier);
    }
}

#[test]
fn four_times_the_light_at_twice_the_distance_ranks_equal_and_the_tie_breaks_on_entity() {
    // The inverse square, exactly: `intensity / distance²` is equal for (1×, 1 m) and (4×, 2 m). The tie
    // then falls to `Entity`, which is what keeps the assignment deterministic.
    let mut app = app(LightPool { size: 1, reach: 100.0, gain: 1.0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    let close_dim = candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 10_000.0);
    let far_bright = candidate(&mut app, Vec3::new(2.0, 0.0, 0.0), 40_000.0);
    settle(&mut app);
    let (_, _, _, base) = assigned(&mut app, 1)[0];
    let winner = if close_dim.to_bits() < far_bright.to_bits() { 10_000.0 } else { 40_000.0 };
    assert!((base - winner).abs() < 1e-3, "the tie broke on Entity: base {base} wants {winner}");

    // And the same scene assigns the same way on every frame.
    let first = assigned(&mut app, 1);
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(first, assigned(&mut app, 1), "a steady pool must not swap equidistant candidates");
}

#[test]
fn gain_never_reshuffles_which_lights_are_real() {
    // The night's dimmer multiplies what a slot writes, not which candidate it takes: a darker night has
    // the same lights in the same places.
    let mut app = app(LightPool { size: 1, reach: 100.0, gain: 1.0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    candidate(&mut app, Vec3::new(2.0, 0.0, 0.0), 8_000.0);
    candidate(&mut app, Vec3::new(3.0, 0.0, 0.0), 60_000.0);
    settle(&mut app);
    let bright = assigned(&mut app, 1)[0];
    app.world_mut().resource_mut::<LightPool>().gain = 0.25;
    app.update();
    let dim = assigned(&mut app, 1)[0];
    assert_eq!(bright.1, dim.1, "the same slot, at the same place");
    assert!(
        (dim.3 - bright.3 * 0.25).abs() < 1e-2,
        "and a quarter as bright: {} against {}",
        dim.3,
        bright.3
    );
}

#[test]
fn the_pools_default_rank_is_illuminance() {
    // The test above pairs each rule with the answer it should give, which validates both arms but not
    // which one a host gets by default. This one pins the default, so changing it is a decision rather
    // than a slip.
    assert_eq!(
        LightPool::default().rank,
        Rank::Illuminance,
        "a released crate's default is a behaviour a host relies on"
    );
}

#[test]
fn the_pools_hand_off_budget_is_its_free_reserves() {
    // The rule iteration 14 added: a hand-off the pool cannot cross-fade is a hand-off that dims its
    // subject, so at most as many lights change owner in a frame as there are free reserves. The rule
    // had no test of its own, and its wrong form is silent — it looks exactly like a pool that is
    // merely slow to respond. Measured: four steady slots, then four challengers nearer the focus in
    // one frame. With one reserve one slot moves this frame; with no reserve all four do.
    fn moved_in_one_frame(reserve: usize) -> usize {
        let mut app =
            app(LightPool { size: 4, reach: 100.0, gain: 1.0, fade_seconds: 0.35, reserve, ..default() });
        focus_at(&mut app, Vec3::ZERO);
        for x in 4..=7 {
            candidate(&mut app, Vec3::new(x as f32, 0.0, 0.0), 10_000.0);
        }
        settle(&mut app);
        let before = assigned(&mut app, 4);
        // Four challengers, all nearer than every held candidate, appearing at once.
        for x in 1..=4 {
            candidate(&mut app, Vec3::new(x as f32 * 0.5, 0.0, 0.0), 10_000.0);
        }
        app.update();
        let after = assigned(&mut app, 4);
        before.iter().zip(after.iter()).filter(|((_, at, _, _), (_, at2, _, _))| at != at2).count()
    }
    assert_eq!(
        moved_in_one_frame(0),
        4,
        "with no reserve there is nothing to hide a move behind, so no throttle"
    );
    assert_eq!(moved_in_one_frame(1), 1, "one reserve carries one cross-faded hand-off, so one move a frame");
    assert_eq!(moved_in_one_frame(2), 2, "two reserves carry two");
}

#[test]
fn one_new_candidate_displaces_one_light_and_no_more() {
    // The artifact this test is for: every slot compared its held candidate against the *same* best
    // free candidate, so one newcomer displaced all of them at once — the whole street's lights
    // teleported, and a slot's fallback then took a candidate another slot was keeping, which left
    // most slots dark for a frame and fading back in. A pool hands one light over, not eight.
    let mut app = app(LightPool { size: 4, reach: 100.0, gain: 1.0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    for x in 1..=4 {
        candidate(&mut app, Vec3::new(x as f32, 0.0, 0.0), 10_000.0);
    }
    settle(&mut app);
    let before = assigned(&mut app, 4);
    assert_eq!(before.iter().filter(|(_, _, v, _)| *v == Visibility::Inherited).count(), 4);

    // One far brighter source, far closer than the worst held candidate: it must take exactly one
    // light, and the three others must not move or dim.
    candidate(&mut app, Vec3::new(0.5, 0.0, 0.0), 90_000.0);
    app.update();
    let after = assigned(&mut app, 4);
    let moved: Vec<_> = before
        .iter()
        .zip(after.iter())
        .filter(|((_, at, _, _), (_, at2, _, _))| at != at2)
        .map(|((slot, at, _, _), (_, at2, _, _))| (*slot, *at, *at2))
        .collect();
    assert_eq!(moved.len(), 1, "one hand-off, not a reshuffle: {moved:?}");
    let full =
        after.iter().filter(|(_, _, visibility, base)| *visibility == Visibility::Inherited && *base > 999.0);
    assert_eq!(full.count(), 3, "the three untouched lights stayed at full brightness: {after:?}");
}

/// A light handed to a prop must flicker on **that prop's** clock, not the slot's: otherwise the real
/// light and the flame drawn on the torch beat against each other, which is the defect
/// `docs/research/flame-flicker.md` measures for the braziers, one level up.
#[test]
fn a_handed_out_light_takes_the_props_own_fire_phase_and_size() {
    let mut app = app(LightPool { size: 2, reach: 100.0, gain: 1.0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    // One candidate near, one far, each with its own fire: different phases and one sized bed.
    app.world_mut().spawn((
        PoolCandidate {
            offset: Vec3::Y * 2.0,
            intensity: 100.0,
            phase: Some(2.5),
            diameter: Some(0.34),
            ..default()
        },
        Transform::from_translation(Vec3::new(1.0, 0.0, 0.0)),
    ));
    app.world_mut().spawn((
        PoolCandidate {
            offset: Vec3::Y * 2.0,
            intensity: 100.0,
            phase: Some(4.1),
            diameter: None,
            ..default()
        },
        Transform::from_translation(Vec3::new(9.0, 0.0, 0.0)),
    ));
    settle(&mut app);
    let mut fires: Vec<(usize, f32, Option<f32>)> = app
        .world_mut()
        .query_filtered::<(&PoolLight, &Flicker), ()>()
        .iter(app.world())
        .map(|(light, flicker)| (light.slot, flicker.phase, flicker.diameter))
        .collect();
    fires.retain(|(slot, ..)| *slot < 2);
    fires.sort_by_key(|(slot, ..)| *slot);
    assert_eq!(fires.len(), 2);
    // Ranked by illuminance: equal intensities, so the nearer one (2.5) takes slot 0.
    assert!((fires[0].1 - 2.5).abs() < 1e-5, "slot 0 should burn at the near prop's phase: {fires:?}");
    assert_eq!(fires[0].2, Some(0.34), "and at its fuel bed's size: {fires:?}");
    assert!((fires[1].1 - 4.1).abs() < 1e-5, "slot 1 should burn at the far prop's phase: {fires:?}");
    assert_eq!(fires[1].2, None, "a fire with no size given keeps the un-sized rate: {fires:?}");
}

/// A prop's own flicker depth travels with its fire onto the pool light, and a candidate that chose
/// none leaves the light at the crate default rather than at whichever prop held the slot before.
#[test]
fn a_handed_out_light_takes_the_props_own_flicker_depth_or_the_default() {
    let mut app = app(LightPool { size: 2, reach: 100.0, gain: 1.0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    app.world_mut().spawn((
        PoolCandidate { offset: Vec3::Y * 2.0, intensity: 100.0, depth: Some(0.3), ..default() },
        Transform::from_translation(Vec3::new(1.0, 0.0, 0.0)),
    ));
    app.world_mut().spawn((
        PoolCandidate { offset: Vec3::Y * 2.0, intensity: 100.0, depth: None, ..default() },
        Transform::from_translation(Vec3::new(9.0, 0.0, 0.0)),
    ));
    settle(&mut app);
    let mut depths: Vec<(usize, f32)> = app
        .world_mut()
        .query_filtered::<(&PoolLight, &Flicker), ()>()
        .iter(app.world())
        .map(|(light, flicker)| (light.slot, flicker.depth))
        .collect();
    depths.retain(|(slot, _)| *slot < 2);
    depths.sort_by_key(|(slot, _)| *slot);
    assert_eq!(depths.len(), 2);
    assert!((depths[0].1 - 0.3).abs() < 1e-6, "the near prop chose 0.3: {depths:?}");
    assert!((depths[1].1 - FLICKER_DEPTH).abs() < 1e-6, "the far prop chose none: {depths:?}");
}

/// A host that models no per-prop phases keeps the crate's spread, so the slots still never beat in
/// step: the new field is an opt-in, not a change of default.
#[test]
fn a_candidate_with_no_fire_phase_leaves_the_slots_own_alone() {
    let mut app = app(LightPool { size: 2, reach: 100.0, gain: 1.0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
    candidate(&mut app, Vec3::new(2.0, 0.0, 0.0), 100.0);
    settle(&mut app);
    let mut phases: Vec<(usize, f32)> = app
        .world_mut()
        .query_filtered::<(&PoolLight, &Flicker), ()>()
        .iter(app.world())
        .map(|(light, flicker)| (light.slot, flicker.phase))
        .collect();
    phases.retain(|(slot, _)| *slot < 2);
    phases.sort_by_key(|(slot, _)| *slot);
    for (index, phase) in &phases {
        assert!(
            (phase - 1.3 * *index as f32).abs() < 1e-5,
            "slot {index}'s own phase is {phase}: {phases:?}"
        );
    }
}

#[test]
fn the_budget_names_who_holds_a_light_and_who_went_without() {
    let mut app = app(LightPool { size: 3, reach: 10.0, reserve: 0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    let near: Vec<Entity> =
        (1..=5).map(|x| candidate(&mut app, Vec3::new(x as f32, 0.0, 0.0), 100.0)).collect();
    let far = candidate(&mut app, Vec3::new(50.0, 0.0, 0.0), 100.0);
    let hidden = candidate(&mut app, Vec3::new(0.5, 0.0, 0.0), 100.0);
    app.world_mut().entity_mut(hidden).insert(Visibility::Hidden);
    settle(&mut app);

    let budget = app.world().resource::<PoolBudget>().clone();
    assert_eq!(budget.granted(), 3, "{budget:?}");
    assert_eq!(budget.wanting, 5, "the far and the hidden candidate do not want one: {budget:?}");
    assert_eq!(budget.starved(), 2);
    for (i, entity) in near.iter().enumerate() {
        assert_eq!(budget.holds(*entity), i < 3, "candidate at x = {}", i + 1);
    }
    assert!(!budget.holds(far) && !budget.holds(hidden));
    assert_eq!(budget.holder_of(0), Some(near[0]));
    assert_eq!(budget.holder_of(9), None);

    // A dark pool wants and holds nothing.
    app.world_mut().resource_mut::<LightPool>().gain = 0.0;
    settle(&mut app);
    let budget = app.world().resource::<PoolBudget>();
    assert_eq!((budget.granted(), budget.wanting, budget.starved()), (0, 0, 0), "{budget:?}");
}
