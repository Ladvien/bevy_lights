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

/// The settings of a pool light that are the template's: everything but its intensity (its candidate's)
/// and its shadow flag (the shadow budget's).
fn shared_settings(light: &PointLight) -> (Color, f32, bool, bool, f32, f32, f32) {
    (
        light.color,
        light.range,
        light.contact_shadows_enabled,
        light.affects_lightmapped_mesh_diffuse,
        light.shadow_depth_bias,
        light.shadow_normal_bias,
        light.shadow_map_near_z,
    )
}

#[test]
fn every_setting_of_the_template_reaches_every_pool_light_live_and_a_later_light_matches() {
    // An editor turns on contact shadows or moves the shadow bias while the pool burns: every light it
    // owns, the reserve included, takes the change on the next frame — not only the lights spawned
    // after it — and each keeps the shadow flag the budget gave it.
    let mut app =
        app(LightPool { size: 2, reserve: 1, reach: 10.0, gain: 1.0, max_shadowed: Some(1), ..default() });
    app.world_mut().resource_mut::<LightPool>().light.shadow_maps_enabled = true;
    focus_at(&mut app, Vec3::ZERO);
    candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
    candidate(&mut app, Vec3::new(2.0, 0.0, 0.0), 100.0);
    settle(&mut app);
    let flags_before: Vec<bool> = lights(&mut app).iter().map(|light| light.4).collect();
    assert_eq!(flags_before.iter().filter(|shadowed| **shadowed).count(), 1, "one shadow: {flags_before:?}");

    let template = {
        let mut pool = app.world_mut().resource_mut::<LightPool>();
        pool.light.color = Color::srgb(1.0, 0.4, 0.1);
        pool.light.range = 13.0;
        pool.light.contact_shadows_enabled = true;
        pool.light.affects_lightmapped_mesh_diffuse = false;
        pool.light.shadow_depth_bias = 0.3;
        pool.light.shadow_normal_bias = 1.1;
        pool.light.shadow_map_near_z = 0.25;
        shared_settings(&pool.light)
    };
    app.update();
    let mut found: Vec<(usize, PointLight)> = app
        .world_mut()
        .query::<(&PoolLight, &PointLight)>()
        .iter(app.world())
        .map(|(slot, light)| (slot.slot, *light))
        .collect();
    found.sort_by_key(|(slot, _)| *slot);
    assert_eq!(found.len(), 3, "two lights and a reserve");
    for (slot, light) in &found {
        assert_eq!(shared_settings(light), template, "slot {slot} kept an old setting");
    }
    let flags_after: Vec<bool> = found.iter().map(|(_, light)| light.shadow_maps_enabled).collect();
    assert_eq!(flags_after, flags_before, "a template change is not a shadow change");

    // One fixture: a light the pool spawns later is the same as the ones already burning.
    app.world_mut().resource_mut::<LightPool>().size = 3;
    settle(&mut app);
    let spawned: Vec<_> = app
        .world_mut()
        .query::<(&PoolLight, &PointLight)>()
        .iter(app.world())
        .map(|(_, light)| shared_settings(light))
        .collect();
    assert_eq!(spawned.len(), 4);
    assert!(spawned.iter().all(|settings| *settings == template), "{spawned:?}");
}

/// Every pool light, by slot: `(slot, translation, visibility, flicker base, casts a shadow)`.
fn lights(app: &mut App) -> Vec<(usize, Vec3, Visibility, f32, bool)> {
    let mut found: Vec<_> = app
        .world_mut()
        .query::<(&PoolLight, &Transform, &Visibility, &Flicker, &PointLight)>()
        .iter(app.world())
        .map(|(slot, transform, visibility, flicker, light)| {
            (slot.slot, transform.translation, *visibility, flicker.base, light.shadow_maps_enabled)
        })
        .collect();
    found.sort_by_key(|(slot, ..)| *slot);
    found
}

/// The visible pool lights that cast a shadow, by where they stand.
fn shadowed_at(app: &mut App) -> Vec<Vec3> {
    lights(app)
        .into_iter()
        .filter(|(_, _, visibility, _, shadow)| *shadow && *visibility != Visibility::Hidden)
        .map(|(_, at, ..)| at)
        .collect()
}

/// A pool whose lights cast shadows, at most `cap` of them.
fn shadowed_pool(size: usize, cap: usize, reserve: usize) -> LightPool {
    let mut pool = LightPool { size, reach: 100.0, gain: 1.0, reserve, max_shadowed: Some(cap), ..default() };
    pool.light.shadow_maps_enabled = true;
    pool
}

/// The visible lights standing at `at`, summed: what a cross-fade at one place must hold.
fn total_at(lights: &[(usize, Vec3, Visibility, f32, bool)], at: Vec3) -> f32 {
    lights
        .iter()
        .filter(|light| light.2 != Visibility::Hidden && (light.1 - at).length() < 1e-4)
        .map(|light| light.3)
        .sum()
}

#[test]
fn the_shadow_budget_goes_to_the_best_ranked_lights() {
    // Eight real lights, three shadows: the three candidates nearest the focus (all burn alike, so
    // nearest is brightest) cast shadows, the next three light without one, and the budget says so.
    let mut app = app(shadowed_pool(8, 3, 1));
    focus_at(&mut app, Vec3::ZERO);
    // Spawned furthest first, so entity order cannot be what picks the shadows.
    let mut by_distance: Vec<Entity> =
        (1..=6).rev().map(|x| candidate(&mut app, Vec3::new(x as f32, 0.0, 0.0), 100.0)).collect();
    by_distance.reverse();
    settle(&mut app);
    let mut xs: Vec<f32> = shadowed_at(&mut app).iter().map(|at| at.x).collect();
    xs.sort_by(f32::total_cmp);
    assert_eq!(xs, [1.0, 2.0, 3.0], "the three nearest cast the shadows: {:?}", lights(&mut app));
    let budget = app.world().resource::<PoolBudget>().clone();
    assert_eq!(budget.granted(), 6);
    let mut named = budget.shadowed.clone();
    named.sort();
    let mut nearest = by_distance[..3].to_vec();
    nearest.sort();
    assert_eq!(named, nearest, "the budget names exactly the shadowed holders");
    assert!(budget.casts_shadow(by_distance[0]) && !budget.casts_shadow(by_distance[3]));

    // A change of the settings is a settings change, not a hand-off: it applies on the next frame.
    app.world_mut().resource_mut::<LightPool>().max_shadowed = Some(1);
    app.update();
    let xs: Vec<f32> = shadowed_at(&mut app).iter().map(|at| at.x).collect();
    assert_eq!(xs, [1.0], "a lowered cap keeps the nearest: {:?}", lights(&mut app));
    app.world_mut().resource_mut::<LightPool>().light.shadow_maps_enabled = false;
    app.update();
    assert!(
        shadowed_at(&mut app).is_empty(),
        "the template's flag turns them all off: {:?}",
        lights(&mut app)
    );
    assert!(app.world().resource::<PoolBudget>().shadowed.is_empty());
}

/// The visible reserve lights (slot at or past the pool's size), by `(casts a shadow, flicker base)`.
fn reserves_at(app: &mut App, size: usize) -> Vec<(bool, f32)> {
    lights(app)
        .into_iter()
        .filter(|(slot, _, visibility, ..)| *slot >= size && *visibility != Visibility::Hidden)
        .map(|(_, _, _, base, shadow)| (shadow, base))
        .collect()
}

#[test]
fn turning_the_pools_shadows_off_reaches_the_reserves_too() {
    // Pool of one, cap one, one reserve. The torch leaves and the reserve fades its shadowed light out;
    // while it does, the player turns shadows off, by the template's flag or by a cap of nothing. The
    // settings say no pool light casts, so none may, the reserve's included, though its light fades on.
    for turn_off in [
        (|pool: &mut LightPool| pool.light.shadow_maps_enabled = false) as fn(&mut LightPool),
        |pool: &mut LightPool| pool.max_shadowed = Some(0),
    ] {
        let mut app = app(shadowed_pool(1, 1, 1));
        focus_at(&mut app, Vec3::ZERO);
        let near = candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
        settle(&mut app);
        assert_eq!(shadowed_at(&mut app).len(), 1);
        app.world_mut().entity_mut(near).insert(Visibility::Hidden);
        app.update();
        assert_eq!(
            reserves_at(&mut app, 1).iter().filter(|(shadow, _)| *shadow).count(),
            1,
            "fixture: a reserve carries the shadow: {:?}",
            lights(&mut app)
        );
        turn_off(&mut app.world_mut().resource_mut::<LightPool>());
        app.update();
        assert!(shadowed_at(&mut app).is_empty(), "shadows off, yet a light casts: {:?}", lights(&mut app));
        let carrying = reserves_at(&mut app, 1);
        assert!(
            carrying.len() == 1 && carrying[0].1 > 0.0,
            "the reserve still fades its light out, unshadowed: {:?}",
            lights(&mut app)
        );
        // And it stays off for the whole of the fade.
        for frame in 0..30 {
            app.update();
            assert!(shadowed_at(&mut app).is_empty(), "frame {frame}: {:?}", lights(&mut app));
        }
    }
}

#[test]
fn a_departure_on_the_frame_shadows_go_off_starts_unshadowed() {
    // The reserve's light is copied from last frame's look, which was shadowed. The frame the shadows go
    // off it must not start casting from that copy.
    let mut app = app(shadowed_pool(1, 1, 1));
    focus_at(&mut app, Vec3::ZERO);
    let near = candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
    settle(&mut app);
    assert_eq!(shadowed_at(&mut app).len(), 1);
    app.world_mut().entity_mut(near).insert(Visibility::Hidden);
    app.world_mut().resource_mut::<LightPool>().light.shadow_maps_enabled = false;
    app.update();
    let carrying = reserves_at(&mut app, 1);
    assert!(
        carrying.len() == 1 && carrying[0].1 > 0.0,
        "fixture: the departing light fades out: {:?}",
        lights(&mut app)
    );
    assert!(shadowed_at(&mut app).is_empty(), "a reserve started a shadow: {:?}", lights(&mut app));
}

#[test]
fn a_light_that_earns_a_shadow_gains_it_by_a_cross_fade() {
    // One shadow, on the nearer of two torches. The nearer goes out, and the other earns the shadow
    // without moving: a reserve light takes it as it stood (no shadow) and fades out while its own light
    // fades back in with the shadow, so the ground it lights holds and the shadow deepens.
    let mut app = app(shadowed_pool(2, 1, 2));
    focus_at(&mut app, Vec3::ZERO);
    let near = candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
    let far = candidate(&mut app, Vec3::new(3.0, 0.0, 0.0), 100.0);
    settle(&mut app);
    assert_eq!(app.world().resource::<PoolBudget>().shadowed, [near]);
    let at = Vec3::new(3.0, 2.0, 0.0);
    app.world_mut().entity_mut(near).insert(Visibility::Hidden);
    let mut frames = Vec::new();
    for _ in 0..26 {
        app.update();
        frames.push(lights(&mut app));
    }
    let first = &frames[0];
    let here: Vec<_> = first
        .iter()
        .filter(|light| light.2 != Visibility::Hidden && (light.1 - at).length() < 1e-4)
        .collect();
    assert_eq!(here.len(), 2, "the torch's light and a reserve carrying it as it was: {first:?}");
    assert!(
        here.iter().any(|light| light.0 < 2 && light.4 && light.3 < 1e-3),
        "its own light fades in, shadowed: {here:?}"
    );
    assert!(here.iter().any(|light| light.0 >= 2 && !light.4), "the reserve carries it unshadowed: {here:?}");
    // The two ends run on one clock, so the sum is the light itself on every frame, not a dip: with the
    // departing light's fade read one frame ahead of the arriving one's, it sagged to 93 mid-fade.
    for (frame, lights) in frames.iter().enumerate() {
        let total = total_at(lights, at);
        assert!((total - 100.0).abs() < 1e-3, "frame {frame}: the lit ground moved, {total} ({lights:?})");
    }
    let last = &frames[25];
    assert_eq!(shadowed_at(&mut app), [at], "settled with the shadow: {last:?}");
    assert!((total_at(last, at) - 100.0).abs() < 1e-3, "at full brightness: {last:?}");
    assert_eq!(app.world().resource::<PoolBudget>().shadowed, [far]);
}

#[test]
fn without_a_reserve_a_change_of_shadow_dips_the_light() {
    // With nothing to cross-fade behind, a shadow change does what a hand-off does: the light dips and
    // comes back with the new shadow.
    let mut app = app(shadowed_pool(2, 1, 0));
    focus_at(&mut app, Vec3::ZERO);
    let near = candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
    candidate(&mut app, Vec3::new(3.0, 0.0, 0.0), 100.0);
    settle(&mut app);
    app.world_mut().entity_mut(near).insert(Visibility::Hidden);
    app.update();
    let at = Vec3::new(3.0, 2.0, 0.0);
    let now = lights(&mut app);
    assert_eq!(shadowed_at(&mut app), [at], "the shadow moved at once: {now:?}");
    assert!(total_at(&now, at) < 1.0, "and the light dipped: {now:?}");
    for _ in 0..24 {
        app.update();
    }
    let later = lights(&mut app);
    assert!((total_at(&later, at) - 100.0).abs() < 1e-3, "and came back: {later:?}");
}

#[test]
fn a_nearer_light_takes_the_shadow_without_ever_exceeding_the_budget() {
    // Two held lights and one shadow, on the nearer. Then the other walks in to a quarter of the
    // distance and earns the shadow. The swap is a loss and a gain, each cross-faded through the one
    // reserve, and at no frame do two slots cast shadows at once.
    let mut app = app(shadowed_pool(2, 1, 1));
    focus_at(&mut app, Vec3::ZERO);
    let first = candidate(&mut app, Vec3::new(2.0, 0.0, 0.0), 100.0);
    let second = candidate(&mut app, Vec3::new(4.0, 0.0, 0.0), 100.0);
    settle(&mut app);
    assert!(app.world().resource::<PoolBudget>().casts_shadow(first));
    move_candidate(&mut app, second, Vec3::new(1.0, 0.0, 0.0));
    app.update();
    // The loss goes first: the first torch's shadow lifts through a reserve standing in its light.
    let lifted = lights(&mut app);
    let reserve = lifted.iter().find(|light| light.0 >= 2).copied().expect("a reserve light");
    assert!(
        reserve.2 != Visibility::Hidden && reserve.4,
        "the reserve fades the shadowed light out: {lifted:?}"
    );
    assert!((reserve.1 - Vec3::new(2.0, 2.0, 0.0)).length() < 1e-4, "where it stood: {lifted:?}");
    for frame in 0..80 {
        app.update();
        let slots_casting = lights(&mut app)
            .iter()
            .filter(|light| light.0 < 2 && light.4 && light.2 != Visibility::Hidden)
            .count();
        assert!(
            slots_casting <= 1,
            "frame {frame}: {slots_casting} slots cast shadows, {:?}",
            lights(&mut app)
        );
    }
    let budget = app.world().resource::<PoolBudget>().clone();
    assert_eq!(budget.shadowed, [second], "the nearer light took the shadow: {budget:?}");
    assert!(budget.holds(first) && budget.holds(second), "and nobody lost a light: {budget:?}");
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

#[derive(Resource, Default)]
struct BudgetWrites(usize);

#[test]
fn the_budget_is_flagged_changed_only_when_the_schedule_changes() {
    let mut app = app(LightPool { size: 2, reach: 10.0, reserve: 0, ..default() });
    app.init_resource::<BudgetWrites>().add_systems(
        Update,
        |budget: Res<PoolBudget>, mut writes: ResMut<BudgetWrites>| {
            if budget.is_changed() {
                writes.0 += 1;
            }
        },
    );
    focus_at(&mut app, Vec3::ZERO);
    candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
    candidate(&mut app, Vec3::new(2.0, 0.0, 0.0), 100.0);
    settle(&mut app);
    app.world_mut().resource_mut::<BudgetWrites>().0 = 0;

    for _ in 0..30 {
        app.update();
    }
    assert_eq!(
        app.world().resource::<BudgetWrites>().0,
        0,
        "a steady pool must not mark its schedule changed"
    );

    // A closer third candidate takes a slot: the schedule changes, so the flag must fire.
    candidate(&mut app, Vec3::new(0.5, 0.0, 0.0), 100.0);
    settle(&mut app);
    assert!(app.world().resource::<BudgetWrites>().0 >= 1);
}

#[cfg(feature = "serde")]
mod saved_pool {
    use bevy::light::PointLight;
    use bevy::prelude::*;
    use bevy_lights::{FocusRule, LightPool, Rank};

    #[test]
    fn a_pool_round_trips_with_its_slot_template() {
        let pool = LightPool {
            size: 6,
            reach: 31.5,
            gain: 0.4,
            light: PointLight {
                color: Color::srgb(1.0, 0.5, 0.2),
                range: 7.0,
                radius: 0.25,
                shadow_maps_enabled: true,
                intensity: 9999.0,
                ..default()
            },
            rank: Rank::Distance,
            focus: FocusRule::Shared,
            hysteresis: 1.3,
            fade_seconds: 0.0,
            reserve: 3,
            max_shadowed: Some(3),
        };
        let json = serde_json::to_string(&pool).unwrap();
        let back: LightPool = serde_json::from_str(&json).unwrap();
        assert_eq!((back.size, back.reach, back.gain, back.rank), (6, 31.5, 0.4, Rank::Distance));
        assert_eq!(back.focus, FocusRule::Shared, "the focus rule is saved");
        assert_eq!((back.hysteresis, back.fade_seconds, back.reserve), (1.3, 0.0, 3));
        assert_eq!(back.max_shadowed, Some(3), "the shadow budget is saved");
        assert_eq!(back.light.color, pool.light.color);
        assert_eq!((back.light.range, back.light.radius, back.light.shadow_maps_enabled), (7.0, 0.25, true));
        assert_eq!(
            back.light.intensity, 0.0,
            "the template's intensity is ignored by the pool and not saved"
        );
    }

    /// Bevy's `soft_shadows_enabled` exists only with its `experimental_pbr_pcss` feature; `None` without it.
    fn soft(light: &PointLight) -> Option<bool> {
        light.field("soft_shadows_enabled").and_then(|f| f.try_downcast_ref::<bool>()).copied()
    }

    #[test]
    fn a_pool_saves_every_setting_its_lights_take_from_the_template() {
        let mut light = PointLight {
            contact_shadows_enabled: true,
            affects_lightmapped_mesh_diffuse: false,
            shadow_depth_bias: 0.3,
            shadow_normal_bias: 1.1,
            shadow_map_near_z: 0.25,
            ..LightPool::default().light
        };
        // Set where the build has it, so a build with Bevy's PCSS checks that it is saved too.
        if let Some(soft) = light.field_mut("soft_shadows_enabled").and_then(|f| f.try_downcast_mut::<bool>())
        {
            *soft = true;
        }
        let pool = LightPool { light, ..default() };
        let back: LightPool = serde_json::from_str(&serde_json::to_string(&pool).unwrap()).unwrap();
        let saved = |l: &PointLight| {
            (
                l.contact_shadows_enabled,
                l.affects_lightmapped_mesh_diffuse,
                l.shadow_depth_bias,
                l.shadow_normal_bias,
                l.shadow_map_near_z,
                soft(l),
            )
        };
        assert_eq!(saved(&back.light), saved(&pool.light));

        // A file that names none of them keeps Bevy's own defaults, not zeros.
        let partial: LightPool = serde_json::from_str(r#"{"light": {"range": 12.0}}"#).unwrap();
        assert_eq!(saved(&partial.light), saved(&PointLight::default()));
    }

    #[test]
    fn a_project_file_naming_only_some_fields_takes_the_defaults_for_the_rest() {
        let pool: LightPool = serde_json::from_str(r#"{"size": 4, "light": {"range": 12.0}}"#).unwrap();
        let defaults = LightPool::default();
        assert_eq!(pool.size, 4);
        assert_eq!(pool.light.range, 12.0);
        assert_eq!(pool.light.color, defaults.light.color);
        assert_eq!((pool.reach, pool.reserve, pool.rank), (defaults.reach, defaults.reserve, defaults.rank));
        assert_eq!(
            pool.focus,
            FocusRule::One,
            "a file saved before the focus rule existed serves one viewer"
        );
    }
}

#[derive(Resource, Default)]
struct Heard {
    lit: usize,
    out: usize,
}

fn listen(app: &mut App) {
    app.init_resource::<Heard>().add_systems(
        Update,
        |mut lit: MessageReader<Kindled>, mut out: MessageReader<Snuffed>, mut heard: ResMut<Heard>| {
            heard.lit += lit.read().count();
            heard.out += out.read().count();
        },
    );
}

/// The one real light's flicker base and visibility.
fn only_slot(app: &mut App) -> (f32, Visibility) {
    let found = assigned(app, 1);
    (found[0].3, found[0].2)
}

#[test]
fn a_pooled_fire_that_is_snuffed_dims_its_light_to_nothing_and_frees_the_slot() {
    let mut app = app(LightPool { size: 1, reserve: 0, reach: 30.0, gain: 1.0, ..default() });
    listen(&mut app);
    focus_at(&mut app, Vec3::ZERO);
    let near = candidate(&mut app, Vec3::new(2.0, 0.0, 0.0), 1000.0);
    let far = candidate(&mut app, Vec3::new(9.0, 0.0, 0.0), 400.0);
    settle(&mut app);
    assert_eq!(only_slot(&mut app).0, 1000.0, "the nearer fire holds the only light");

    app.world_mut().entity_mut(near).insert(Snuff::default());
    let mut previous = 1000.0;
    for frame in 0..(SNUFF_SECONDS * 60.0) as usize - 2 {
        app.update();
        let (base, _) = only_slot(&mut app);
        assert!(base <= previous + 1e-3, "frame {frame}: rose from {previous} to {base}");
        assert!(base < 1000.0 || frame == 0, "frame {frame}: still at full");
        previous = base;
    }
    assert!(previous < 60.0, "nearly out by the end: {previous}");
    assert_eq!(app.world().resource::<Heard>().out, 0, "not announced before it is out");

    for _ in 0..4 {
        app.update();
    }
    assert_eq!(app.world().resource::<Heard>().out, 1, "announced once, on the frame it went out");
    // The freed slot is the next fire's, which fades in over `fade_seconds`.
    for _ in 0..60 {
        app.update();
    }
    assert_eq!(only_slot(&mut app).0, 400.0);
    assert_eq!(app.world().resource::<PoolBudget>().holder_of(0), Some(far));
    assert_eq!(app.world().resource::<PoolBudget>().wanting, 1, "a fire that is out wants nothing");
    assert_eq!(app.world().resource::<Heard>().out, 1);
}

/// A scene saved with its torches out loads each with a finished `Snuff`: a town at dawn is thousands of them,
/// and the pool meets every one every frame. A finished `Snuff` is a fire that is out, so it holds no light and
/// is not announced again; one still in progress holds a dimmed light; and a fire that was never snuffed holds a
/// full one.
#[test]
fn fires_loaded_already_out_hold_no_light_and_one_loaded_mid_snuff_holds_a_dim_one() {
    let mut app = app(LightPool { size: 3, reserve: 0, reach: 30.0, gain: 1.0, ..default() });
    listen(&mut app);
    focus_at(&mut app, Vec3::ZERO);
    let out: Vec<Entity> = (1..=4)
        .map(|near| {
            let fire = candidate(&mut app, Vec3::new(near as f32, 0.0, 0.0), 1000.0);
            app.world_mut().entity_mut(fire).insert(Snuff { elapsed: SNUFF_SECONDS });
            fire
        })
        .collect();
    let dying = candidate(&mut app, Vec3::new(6.0, 0.0, 0.0), 1000.0);
    app.world_mut().entity_mut(dying).insert(Snuff { elapsed: SNUFF_SECONDS / 2.0 });
    let burning = candidate(&mut app, Vec3::new(9.0, 0.0, 0.0), 1000.0);
    settle(&mut app);

    let budget = app.world().resource::<PoolBudget>();
    let holders: Vec<Option<Entity>> = (0..3).map(|slot| budget.holder_of(slot)).collect();
    assert!(
        holders.iter().flatten().all(|held| !out.contains(held)),
        "a fire that is out holds a light: {holders:?}"
    );
    assert!(holders.contains(&Some(dying)) && holders.contains(&Some(burning)), "{holders:?}");
    assert_eq!(holders.iter().flatten().count(), 2, "the third light has nobody to light: {holders:?}");
    assert_eq!(budget.wanting, 2, "four fires that are out want nothing");

    let base_of = |app: &mut App, fire: Entity| {
        let slot = (0..3).find(|slot| app.world().resource::<PoolBudget>().holder_of(*slot) == Some(fire));
        slot.map(|slot| assigned(app, 3)[slot].3)
    };
    let burning_base = base_of(&mut app, burning);
    let dying_base = base_of(&mut app, dying);
    assert_eq!(burning_base, Some(1000.0), "a fire that was never snuffed is at full");
    assert!(
        dying_base.is_some_and(|base| 0.0 < base && base < 1000.0),
        "a fire loaded half way out is dim, not out and not full: {dying_base:?}"
    );
    assert_eq!(app.world().resource::<Heard>().out, 0, "a fire loaded out is not announced");
}

#[test]
fn a_pooled_fire_that_is_kindled_lends_a_light_that_flares_up_and_is_heard_once() {
    let mut app = app(LightPool { size: 1, reserve: 0, reach: 30.0, gain: 1.0, ..default() });
    listen(&mut app);
    focus_at(&mut app, Vec3::ZERO);
    let torch = candidate(&mut app, Vec3::new(2.0, 0.0, 0.0), 1000.0);
    settle(&mut app);
    app.world_mut().entity_mut(torch).insert(Kindle::default());
    app.update();
    let first = only_slot(&mut app).0;
    assert!(first < 100.0, "a fire just lit is small: {first}");
    let mut previous = first;
    for _ in 0..(KINDLE_SECONDS * 60.0) as usize + 2 {
        app.update();
        let (base, _) = only_slot(&mut app);
        // `kindle_gain` flares a little past 1 on its way to settling, so only the climb is monotone.
        assert!(base >= previous - 1e-3 || previous > 1000.0, "kindle climbs: {previous} then {base}");
        previous = base;
    }
    assert!((previous - 1000.0).abs() < 0.01, "settled at the candidate's own intensity: {previous}");
    assert_eq!(app.world().resource::<Heard>().lit, 1);
}

/// A torch flame's light follows the flame when the wind lays it over: downwind of the upright axis and a
/// little lower, by the same tilt the flame is drawn at. Still air, a candidate with no flame, and a
/// candidate off the `FlameSite` entity are untouched.
#[test]
fn a_pooled_light_follows_its_flames_lean() {
    let mut app = app(LightPool { size: 3, ..default() });
    app.insert_resource(FlameWind { velocity: Vec2::ZERO });
    let leaning = app
        .world_mut()
        .spawn((
            FlameSite {
                color: None,
                height: 1.0,
                phase: 0.0,
                kindle: 0.0,
                diameter: None,
                depth: 0.15,
                depth_fade: false,
                fire: None,
            },
            PoolCandidate { offset: Vec3::Y * 0.5, intensity: 1000.0, ..default() },
            Transform::from_xyz(0.0, 0.0, 1.0),
        ))
        .id();
    let bare = candidate(&mut app, Vec3::new(0.0, 0.0, -1.0), 1000.0);
    focus_at(&mut app, Vec3::ZERO);
    settle(&mut app);
    let by = |app: &mut App, entity: Entity| -> Vec3 {
        let at = app.world().get::<Transform>(entity).expect("candidate").translation;
        slots(app)
            .into_iter()
            .map(|(_, t, ..)| t)
            .find(|t| (t.z - at.z).abs() < 0.01)
            .expect("a light by the candidate")
    };
    let still = by(&mut app, leaning);
    assert!((still - Vec3::new(0.0, 0.5, 1.0)).length() < 1e-4, "still air: {still:?}");

    app.insert_resource(FlameWind { velocity: Vec2::new(3.0, 0.0) });
    settle(&mut app);
    let moved = by(&mut app, leaning);
    let want = Vec3::from(lean_light_shift(1.0, 3.0, DEFAULT_FIRE_DIAMETER, [1.0, 0.0]));
    assert!(moved.x > 0.2 && moved.y < 0.5, "downwind and lower: {moved:?}");
    assert!((moved - (still + want)).length() < 1e-4, "{moved:?} vs {still:?} + {want:?}");
    assert!((by(&mut app, bare) - Vec3::new(0.0, 2.0, -1.0)).length() < 1e-4, "no flame, no lean");
}

/// A torch whose light is a sibling of its flame (joined by `FlameSite::fire`) leans its light exactly as a
/// light on the site entity does; a light no site names stays upright.
#[test]
fn a_pooled_light_linked_by_a_site_follows_the_lean() {
    let mut app = app(LightPool { size: 3, ..default() });
    app.insert_resource(FlameWind { velocity: Vec2::new(3.0, 0.0) });
    let light = app
        .world_mut()
        .spawn((
            PoolCandidate { offset: Vec3::Y * 0.5, intensity: 1000.0, ..default() },
            Transform::from_xyz(0.0, 0.0, 1.0),
        ))
        .id();
    app.world_mut().spawn((
        FlameSite {
            color: None,
            height: 1.0,
            phase: 0.0,
            kindle: 0.0,
            diameter: None,
            depth: 0.15,
            depth_fade: false,
            fire: Some(light),
        },
        Transform::from_xyz(0.0, 0.0, 1.0),
    ));
    let bare = candidate(&mut app, Vec3::new(0.0, 0.0, -1.0), 1000.0);
    focus_at(&mut app, Vec3::ZERO);
    settle(&mut app);
    let at = |app: &mut App, z: f32| -> Vec3 {
        slots(app).into_iter().map(|(_, t, ..)| t).find(|t| (t.z - z).abs() < 0.01).expect("a light")
    };
    let moved = at(&mut app, 1.0);
    let want =
        Vec3::new(0.0, 0.5, 1.0) + Vec3::from(lean_light_shift(1.0, 3.0, DEFAULT_FIRE_DIAMETER, [1.0, 0.0]));
    assert!(moved.x > 0.2, "downwind: {moved:?}");
    assert!((moved - want).length() < 1e-4, "{moved:?} vs {want:?}");
    let _ = bare;
    assert!((at(&mut app, -1.0) - Vec3::new(0.0, 2.0, -1.0)).length() < 1e-4, "no site, no lean");
}

/// A flame site on a torch one metre tall: the light is carried at the flame's middle, half a metre up.
fn torch_site() -> FlameSite {
    FlameSite {
        color: None,
        height: 1.0,
        phase: 0.0,
        kindle: 0.0,
        diameter: None,
        depth: 0.15,
        depth_fade: false,
        fire: None,
    }
}

/// A shadow cross-fade of a light that keeps its owner holds the lit ground still — and under wind the
/// ground the light lights is the leaned flame's, so both ends of the fade stand in the leaned flame. The
/// departing reserve took last frame's un-leaned position and kept it, half a metre upwind of the flame.
#[test]
fn a_shadow_cross_fade_under_wind_keeps_both_ends_in_the_leaned_flame() {
    let mut app = app(shadowed_pool(2, 1, 2));
    app.insert_resource(FlameWind { velocity: Vec2::new(3.0, 0.0) });
    focus_at(&mut app, Vec3::ZERO);
    let torch = |x: f32| {
        (
            torch_site(),
            PoolCandidate { offset: Vec3::Y * 0.5, intensity: 100.0, ..default() },
            Transform::from_xyz(x, 0.0, 0.0),
        )
    };
    let near = app.world_mut().spawn(torch(1.0)).id();
    app.world_mut().spawn(torch(3.0));
    settle(&mut app);
    app.world_mut().entity_mut(near).insert(Visibility::Hidden);
    let lean = Vec3::from(lean_light_shift(1.0, 3.0, DEFAULT_FIRE_DIAMETER, [1.0, 0.0]));
    let at = Vec3::new(3.0, 0.5, 0.0) + lean;
    for frame in 0..26 {
        app.update();
        let now = lights(&mut app);
        let total = total_at(&now, at);
        assert!((total - 100.0).abs() < 1e-3, "frame {frame}: {total} at the leaned flame: {now:?}");
        if frame == 0 {
            // The ordinary hand-off under wind: the torch that went out fades out in its own leaned flame,
            // where its light stood, and does not jump back to the upright axis.
            let gone = total_at(&now, Vec3::new(1.0, 0.5, 0.0) + lean);
            assert!(gone > 99.0, "the departing torch left its leaned flame: {gone} ({now:?})");
        }
    }
}

/// The same cross-fade on a torch-bearer who keeps walking: the slot follows the bearer, and so must the
/// reserve carrying the other end, or a light 2.4 m/s fast is split in two for the whole fade.
#[test]
fn a_shadow_cross_fade_of_a_walking_light_keeps_both_ends_on_it() {
    let mut app = app(shadowed_pool(2, 1, 2));
    focus_at(&mut app, Vec3::ZERO);
    let near = candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
    let walker = candidate(&mut app, Vec3::new(3.0, 0.0, 0.0), 100.0);
    settle(&mut app);
    app.world_mut().entity_mut(near).insert(Visibility::Hidden);
    for frame in 0..26 {
        let at = Vec3::new(3.0 + 0.05 * frame as f32, 2.0, 0.0);
        move_candidate(&mut app, walker, at - Vec3::Y * 2.0);
        app.update();
        let now = lights(&mut app);
        let total = total_at(&now, at);
        assert!((total - 100.0).abs() < 1e-3, "frame {frame}: {total} at the walker's light {at:?}: {now:?}");
    }
}

/// The light a displaced owner leaves fades out where the owner is, not where it was when the hand-off was
/// made: the pool's own doc says it "keeps moving ... exactly as the one that left did". Once the owner is
/// gone from the pool (hidden here), the light holds the last place it followed it to.
#[test]
fn a_departing_light_follows_its_displaced_owner_and_holds_where_it_left_the_pool() {
    let mut app = app(LightPool { size: 1, reserve: 1, reach: 100.0, gain: 1.0, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    let walker = candidate(&mut app, Vec3::new(2.0, 0.0, 0.0), 100.0);
    settle(&mut app);
    // A nearer torch takes the light; the walker is displaced and keeps walking away.
    candidate(&mut app, Vec3::new(0.5, 0.0, 0.0), 100.0);
    let departing = |app: &mut App| -> Vec3 {
        let found = lights(app);
        let reserve = found.iter().find(|light| light.0 >= 1).copied().expect("a reserve light");
        assert!(reserve.2 != Visibility::Hidden, "the reserve fades the walker out: {found:?}");
        reserve.1
    };
    for frame in 1..=12 {
        let at = Vec3::new(2.0 + 0.1 * frame as f32, 0.0, 0.0);
        move_candidate(&mut app, walker, at);
        app.update();
        let at = departing(&mut app);
        assert!(
            (at - Vec3::new(2.0 + 0.1 * frame as f32, 2.0, 0.0)).length() < 1e-4,
            "frame {frame}: {at:?}"
        );
    }
    let last = departing(&mut app);
    app.world_mut().entity_mut(walker).insert(Visibility::Hidden);
    for frame in 0..4 {
        app.update();
        let at = departing(&mut app);
        assert!((at - last).length() < 1e-4, "frame {frame} after it left: {at:?} vs {last:?}");
    }
}

/// A `FireKind`'s bundle is a flame and the light that motivates it, spawned on one entity at the flame's
/// foot. The light belongs in the flame, at `flame_light_offset(height)`: upright in still air, and where
/// the leaned flame's middle is in a wind. With `offset` at zero the pool put it at the foot, and the lean
/// (the move of the flame's *middle*) then sank it `height / 2` straight below the flame, under the coal bed.
#[test]
fn a_fire_kinds_pooled_light_stands_in_its_flame_and_leans_with_it() {
    for kind in FireKind::ALL {
        let mut app = app(LightPool { size: 1, reach: 100.0, gain: 1.0, ..default() });
        app.insert_resource(FlameWind::default());
        let foot = Vec3::new(2.0, 0.0, -3.0);
        app.world_mut().spawn((kind.fire(40.0, 1.0), Transform::from_translation(foot)));
        focus_at(&mut app, Vec3::ZERO);
        settle(&mut app);
        let half = kind.height() * 0.5;
        let still = assigned(&mut app, 1)[0].1;
        assert!(
            (still - (foot + Vec3::Y * half)).length() < 1e-4,
            "{kind}: still air, the light is in the flame's middle: {still:?}"
        );

        app.insert_resource(FlameWind { velocity: Vec2::new(3.0, 0.0) });
        settle(&mut app);
        let tilt = flame_tilt(3.0, kind.diameter());
        let middle = foot + Vec3::new(half * tilt.sin(), half * tilt.cos(), 0.0);
        let light = assigned(&mut app, 1)[0].1;
        assert!(light.y > foot.y, "{kind}: the light sank under the flame's foot: {light:?}");
        assert!(
            (light - middle).length() < 1e-4,
            "{kind}: {light:?} vs the leaned flame's middle {middle:?}"
        );
    }
}

// ---- more than one viewer (`FocusRule::Shared`, `docs/research/pool-foci.md`) --------------------------------

/// How many of the pool's `size` assigned slots are lit within `radius` metres (on the ground plane) of `centre`.
fn lit_near(app: &mut App, size: usize, centre: Vec3, radius: f32) -> usize {
    assigned(app, size)
        .into_iter()
        .filter(|(_, at, visibility, _)| {
            *visibility == Visibility::Inherited
                && Vec2::new(at.x - centre.x, at.z - centre.z).length() <= radius
        })
        .count()
}

/// Two viewers 200 m apart: one among twenty fires 1-4.8 m from it, the other among three, 6-8 m from it.
/// On a plain nearest-focus distance the twenty outrank the three and the second viewer gets nothing.
fn two_viewers(order: impl Fn(usize) -> usize, many: usize) -> (App, Vec3, Vec3) {
    let (a, b) = (Vec3::ZERO, Vec3::new(200.0, 0.0, 0.0));
    let mut app = app(LightPool { size: 8, reach: 30.0, focus: FocusRule::Shared, ..default() });
    focus_at(&mut app, a);
    focus_at(&mut app, b);
    let mut fires: Vec<Vec3> = (0..many).map(|i| a + Vec3::new(1.0 + 0.2 * i as f32, 0.0, 0.0)).collect();
    fires.extend([b + Vec3::new(6.0, 0.0, 0.0), b + Vec3::new(0.0, 0.0, 7.0), b + Vec3::new(0.0, 0.0, -8.0)]);
    for i in 0..fires.len() {
        candidate(&mut app, fires[order(i)], 100.0);
    }
    for _ in 0..60 {
        app.update();
    }
    (app, a, b)
}

#[test]
fn a_viewer_among_few_fires_is_not_starved_by_one_among_many() {
    let (mut app, a, b) = two_viewers(|i| i, 20);
    assert_eq!(lit_near(&mut app, 8, b, 20.0), 3, "all three of the sparse viewer's fires hold a light");
    assert_eq!(lit_near(&mut app, 8, a, 20.0), 5, "the dense viewer keeps the rest of the pool");
    let budget = app.world().resource::<PoolBudget>();
    assert_eq!((budget.wanting, budget.granted(), budget.starved()), (23, 8, 15));
}

#[test]
fn a_viewer_with_fewer_fires_than_its_share_leaves_the_rest_to_the_other() {
    // Size 8, two viewers: four each. The second has three, the first takes five; with the second down to
    // two, six.
    let (mut app, a, b) = two_viewers(|i| i, 20);
    // Despawn one of the second viewer's three fires: the one 6 m east of it.
    let doomed: Entity = app
        .world_mut()
        .query_filtered::<(Entity, &Transform), With<PoolCandidate>>()
        .iter(app.world())
        .find(|(_, transform)| (transform.translation.x - 206.0).abs() < 1e-3)
        .map(|(entity, _)| entity)
        .expect("the fire exists");
    app.world_mut().despawn(doomed);
    for _ in 0..90 {
        app.update();
    }
    assert_eq!(lit_near(&mut app, 8, b, 20.0), 2, "the second viewer has two fires left and holds both");
    assert_eq!(lit_near(&mut app, 8, a, 20.0), 6, "what it does not use goes to the first");
}

#[test]
fn the_assignment_does_not_depend_on_the_order_things_were_spawned_in() {
    let lit = |order: &dyn Fn(usize) -> usize| {
        let (mut app, ..) = two_viewers(order, 20);
        let mut at: Vec<[i32; 3]> = assigned(&mut app, 8)
            .into_iter()
            .filter(|(_, _, visibility, _)| *visibility == Visibility::Inherited)
            .map(|(_, at, ..)| {
                [
                    (at.x * 1000.0).round() as i32,
                    (at.y * 1000.0).round() as i32,
                    (at.z * 1000.0).round() as i32,
                ]
            })
            .collect();
        at.sort();
        at
    };
    let forward = lit(&|i| i);
    let backward = lit(&|i| 22 - i);
    assert_eq!(forward.len(), 8);
    assert_eq!(forward, backward, "the same eight fires are real in either spawn order");
}

#[test]
fn a_fire_between_two_viewers_holds_one_light() {
    let mut app = app(LightPool { size: 4, reach: 20.0, focus: FocusRule::Shared, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    focus_at(&mut app, Vec3::new(10.0, 0.0, 0.0));
    candidate(&mut app, Vec3::new(5.0, 0.0, 0.0), 100.0);
    candidate(&mut app, Vec3::new(-3.0, 0.0, 0.0), 100.0);
    for _ in 0..60 {
        app.update();
    }
    let lit: Vec<Vec3> = assigned(&mut app, 4)
        .into_iter()
        .filter(|(_, _, visibility, _)| *visibility == Visibility::Inherited)
        .map(|(_, at, ..)| at)
        .collect();
    assert_eq!(lit.len(), 2, "two fires, two lights, though one stands in both viewers' reach: {lit:?}");
    assert_eq!(lit.iter().filter(|at| (at.x - 5.0).abs() < 1e-4).count(), 1, "the shared fire is lit once");
}

#[test]
fn the_shared_rule_with_one_viewer_is_the_one_rule_frame_for_frame() {
    // Forty fires placed by a fixed pseudo-random sequence, a focus that walks across them, and both rules:
    // every slot's position, visibility and brightness, and the budget, agree on every frame.
    let build = |focus: FocusRule| {
        let mut app = app(LightPool { size: 6, reach: 14.0, focus, ..default() });
        let eye = app.world_mut().spawn((TestFocus, Transform::default())).id();
        let mut seed = 12345_u32;
        let mut next = move || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 8) as f32 / (1u32 << 24) as f32
        };
        for _ in 0..40 {
            let at = Vec3::new(next() * 40.0 - 20.0, 0.0, next() * 40.0 - 20.0);
            candidate(&mut app, at, 50.0 + next() * 150.0);
        }
        (app, eye)
    };
    let (mut one, eye_one) = build(FocusRule::One);
    let (mut shared, eye_shared) = build(FocusRule::Shared);
    for frame in 0..150 {
        let at = Vec3::new(-18.0 + frame as f32 * 0.24, 0.0, (frame as f32 * 0.05).sin() * 8.0);
        one.world_mut().entity_mut(eye_one).insert(Transform::from_translation(at));
        shared.world_mut().entity_mut(eye_shared).insert(Transform::from_translation(at));
        one.update();
        shared.update();
        assert_eq!(slots(&mut one), slots(&mut shared), "frame {frame}");
        assert_eq!(
            one.world().resource::<PoolBudget>().holders,
            shared.world().resource::<PoolBudget>().holders,
            "frame {frame}"
        );
    }
}

#[test]
fn a_viewer_who_leaves_hands_the_lights_to_the_one_who_stays() {
    let mut app = app(LightPool { size: 4, reach: 30.0, focus: FocusRule::Shared, ..default() });
    let (a, b) = (Vec3::ZERO, Vec3::new(200.0, 0.0, 0.0));
    focus_at(&mut app, a);
    let away = app.world_mut().spawn((TestFocus, Transform::from_translation(b))).id();
    for i in 0..6 {
        candidate(&mut app, a + Vec3::new(1.0 + i as f32, 0.0, 0.0), 100.0);
        candidate(&mut app, b + Vec3::new(1.0 + i as f32, 0.0, 0.0), 100.0);
    }
    for _ in 0..60 {
        app.update();
    }
    assert_eq!(
        (lit_near(&mut app, 4, a, 20.0), lit_near(&mut app, 4, b, 20.0)),
        (2, 2),
        "an equal share each"
    );

    app.world_mut().despawn(away);
    for _ in 0..90 {
        app.update();
    }
    assert_eq!(lit_near(&mut app, 4, a, 20.0), 4, "the viewer who stays has the whole pool");
    assert_eq!(lit_near(&mut app, 4, b, 20.0), 0, "and nobody is near the one who left");

    // A viewer joining takes its share back.
    app.world_mut().spawn((TestFocus, Transform::from_translation(b)));
    for _ in 0..90 {
        app.update();
    }
    assert_eq!(
        (lit_near(&mut app, 4, a, 20.0), lit_near(&mut app, 4, b, 20.0)),
        (2, 2),
        "an equal share again"
    );
}

#[test]
fn the_one_rule_still_leaves_two_viewers_dark() {
    // The default is unchanged: two foci are a misconfiguration under `FocusRule::One`, and `Shared` is how
    // a host asks for them.
    let mut app = app(LightPool { size: 4, reach: 30.0, ..default() });
    assert_eq!(LightPool::default().focus, FocusRule::One);
    focus_at(&mut app, Vec3::ZERO);
    focus_at(&mut app, Vec3::new(100.0, 0.0, 0.0));
    candidate(&mut app, Vec3::new(1.0, 0.0, 0.0), 100.0);
    candidate(&mut app, Vec3::new(101.0, 0.0, 0.0), 100.0);
    for _ in 0..30 {
        app.update();
    }
    assert_eq!(app.world().resource::<PoolBudget>().granted(), 0);
}

/// The sum of every shown pool light's base brightness, reserves included: what the lit scene is given.
fn shown_total(app: &mut App) -> f32 {
    app.world_mut()
        .query::<(&PoolLight, &Visibility, &Flicker)>()
        .iter(app.world())
        .filter(|(_, visibility, _)| **visibility != Visibility::Hidden)
        .map(|(_, _, flicker)| flicker.base)
        .sum()
}

#[test]
fn a_viewer_leaving_does_not_dip_the_pool_when_the_reserves_cover_the_lights_that_move() {
    // Four lights, two viewers, so two lights change owner when one leaves: with two reserves each is
    // cross-faded and the pool's total holds at every frame.
    let mut app = app(LightPool { size: 4, reach: 30.0, focus: FocusRule::Shared, reserve: 2, ..default() });
    focus_at(&mut app, Vec3::ZERO);
    let away = app.world_mut().spawn((TestFocus, Transform::from_xyz(200.0, 0.0, 0.0))).id();
    for i in 0..6 {
        candidate(&mut app, Vec3::new(1.0 + i as f32, 0.0, 0.0), 100.0);
        candidate(&mut app, Vec3::new(201.0 + i as f32, 0.0, 0.0), 100.0);
    }
    for _ in 0..80 {
        app.update();
    }
    let whole = shown_total(&mut app);
    assert!((whole - 400.0).abs() < 1e-3, "four lights at 100: {whole}");
    app.world_mut().despawn(away);
    for frame in 0..40 {
        app.update();
        let total = shown_total(&mut app);
        assert!((total - 400.0).abs() < 1e-3, "frame {frame} after the viewer left: {total}");
    }
}
