//! `Fire`, as a host sees it: one component that becomes a flame, a pooled light and the kind's own sparks, smoke, tongues and bed,
//! what the host spawned beside it, and what a change to it does.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy_lights::*;

#[derive(Component)]
struct TestFocus;

/// The crate's three non-drawing plugins and the expansion. `FlamePlugin` needs a render app, and nothing here draws.
fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        FlickerPlugin,
        LightPoolPlugin::<TestFocus>::default(),
        FirePlugin,
    ))
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / 60.0)))
    .insert_resource(LightPool { size: 4, reach: 100.0, gain: 1.0, ..default() });
    app.update();
    app
}

fn has<C: Component>(app: &App, entity: Entity) -> bool {
    app.world().entity(entity).contains::<C>()
}

fn get<C: Component + Clone>(app: &App, entity: Entity) -> C {
    app.world().entity(entity).get::<C>().expect("the component").clone()
}

/// What each kind has besides its flame and its light, by the kind's own methods.
fn extras(kind: FireKind) -> [bool; 4] {
    [kind.embers().is_some(), kind.smoke().is_some(), kind.tongues().is_some(), kind.bed().is_some()]
}

#[test]
fn a_fire_of_each_kind_becomes_the_parts_its_kind_names_on_the_frame_it_arrives() {
    for kind in FireKind::ALL {
        let mut app = app();
        let fire = app.world_mut().spawn((Fire::lit(kind, 250.0, 0.4), Transform::default())).id();
        app.update();
        let site = get::<FlameSite>(&app, fire);
        assert_eq!(
            (site.height, site.diameter, site.depth, site.fire, site.color),
            (kind.height(), Some(kind.diameter()), kind.depth(), Some(fire), None),
            "{kind}"
        );
        let candidate = get::<PoolCandidate>(&app, fire);
        let want = kind.candidate(250.0, 0.4);
        assert_eq!(
            (candidate.offset, candidate.diameter, candidate.depth, candidate.intensity),
            (want.offset, want.diameter, want.depth, want.intensity),
            "{kind}: the light stands in the flame, at the kind's own rate, depth and brightness"
        );
        assert_eq!(
            candidate.phase,
            Some(site.phase),
            "{kind}: the flame and its light breathe from one clock"
        );
        assert_eq!(
            [
                has::<FlameEmbers>(&app, fire),
                has::<FlameSmoke>(&app, fire),
                has::<FlameTongues>(&app, fire),
                has::<FuelBed>(&app, fire)
            ],
            extras(kind),
            "{kind}"
        );
    }
}

#[test]
fn a_fire_with_no_brightness_of_its_own_is_sized_from_its_kind() {
    let mut app = app();
    let fire = app.world_mut().spawn((Fire::new(FireKind::Campfire), Transform::default())).id();
    app.update();
    let kind = FireKind::Campfire;
    let want = kind.candidate(FIRE_EDGE_LUX, kind.height() * 0.5);
    assert_eq!(get::<PoolCandidate>(&app, fire).intensity, want.intensity);
    assert!(want.intensity > 0.0);
}

#[test]
fn what_the_host_spawned_beside_a_fire_is_the_hosts() {
    let mut app = app();
    let fire = app
        .world_mut()
        .spawn((
            Fire::new(FireKind::Campfire),
            FlameSmoke { density: 0.2 },
            FlameSite { height: 3.0, ..FireKind::Campfire.site() },
            Transform::default(),
        ))
        .id();
    app.update();
    assert_eq!(get::<FlameSmoke>(&app, fire).density, 0.2, "the host's smoke, not the campfire's");
    assert_eq!(get::<FlameSite>(&app, fire).height, 3.0, "the host's flame");
    assert!(has::<FlameTongues>(&app, fire) && has::<FuelBed>(&app, fire), "the rest is the kind's");
    assert!(has::<PoolCandidate>(&app, fire));
}

#[test]
fn a_changed_fire_writes_every_part_again_and_removes_what_it_no_longer_names() {
    let mut app = app();
    let fire = app.world_mut().spawn((Fire::new(FireKind::Campfire), Transform::default())).id();
    app.update();
    assert!(has::<FlameTongues>(&app, fire) && has::<FuelBed>(&app, fire) && has::<FlameEmbers>(&app, fire));

    // A panel flips the kind through `get_mut`, as reflection and BRP do.
    app.world_mut().get_mut::<Fire>(fire).unwrap().kind = FireKind::Lantern;
    app.update();
    let site = get::<FlameSite>(&app, fire);
    assert_eq!(
        (site.height, site.diameter),
        (FireKind::Lantern.height(), Some(FireKind::Lantern.diameter()))
    );
    assert_eq!(get::<PoolCandidate>(&app, fire).diameter, Some(FireKind::Lantern.diameter()));
    for left_behind in
        [has::<FlameTongues>(&app, fire), has::<FuelBed>(&app, fire), has::<FlameEmbers>(&app, fire)]
    {
        assert!(!left_behind, "a lantern has no tongues, bed or sparks");
    }
    assert!(!has::<FlameSmoke>(&app, fire), "an enclosed wick gives no smoke");

    // A flag turns an extra off on its own; replacing the fire (what an editor's undo does) writes again.
    app.world_mut().entity_mut(fire).insert(Fire { smoke: false, ..Fire::new(FireKind::Campfire) });
    app.update();
    assert!(has::<FlameTongues>(&app, fire) && has::<FlameEmbers>(&app, fire));
    assert!(!has::<FlameSmoke>(&app, fire), "the fire says no smoke");
}

#[test]
fn what_a_host_edits_on_a_part_stands_until_the_fire_changes() {
    let mut app = app();
    let fire = app.world_mut().spawn((Fire::new(FireKind::Torch), Transform::default())).id();
    app.update();
    app.world_mut().get_mut::<FlameSite>(fire).unwrap().depth = 0.4;
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(get::<FlameSite>(&app, fire).depth, 0.4, "nothing rewrote it");
    app.world_mut().get_mut::<Fire>(fire).unwrap().lumens = Some(9_000.0);
    app.update();
    assert_eq!(
        get::<FlameSite>(&app, fire).depth,
        FireKind::Torch.depth(),
        "the fire changed, so the preset is written again"
    );
    assert!(
        (get::<PoolCandidate>(&app, fire).intensity * flicker_mean(FireKind::Torch.depth()) - 9_000.0).abs()
            < 1.0
    );
}

#[test]
fn fires_placed_alike_do_not_breathe_together_and_a_phase_given_is_kept() {
    let mut app = app();
    let fires: Vec<Entity> = (0..20)
        .map(|_| app.world_mut().spawn((Fire::new(FireKind::Torch), Transform::default())).id())
        .collect();
    let pinned = app
        .world_mut()
        .spawn((Fire { phase: Some(1.25), ..Fire::new(FireKind::Torch) }, Transform::default()))
        .id();
    app.update();
    let mut phases: Vec<f32> = fires.iter().map(|f| get::<FlameSite>(&app, *f).phase).collect();
    phases.sort_by(f32::total_cmp);
    assert!(phases.windows(2).all(|w| w[0] != w[1]), "twenty torches, twenty phases: {phases:?}");
    assert!(phases.iter().all(|p| (0.0..std::f32::consts::TAU).contains(p)));
    assert_eq!(get::<FlameSite>(&app, pinned).phase, 1.25);
    assert_eq!(get::<PoolCandidate>(&app, pinned).phase, Some(1.25));
}

#[test]
fn one_colour_is_the_light_and_the_flames_middle_at_the_shipped_brightness() {
    let mut app = app();
    let blue = Color::from(LinearRgba::rgb(0.1, 0.3, 1.0));
    let fire = app
        .world_mut()
        .spawn((Fire { color: Some(blue), ..Fire::new(FireKind::Torch) }, Transform::default()))
        .id();
    app.update();
    assert_eq!(get::<PoolCandidate>(&app, fire).color, Some(blue), "the light is the colour");
    let [r, g, b] = flame_mid([0.1, 0.3, 1.0]);
    let flame = LinearRgba::from(get::<FlameSite>(&app, fire).color.expect("the flame has a colour"));
    assert_eq!([flame.red, flame.green, flame.blue], [r, g, b]);
    assert!(b > 1.5, "a blue weighs half an orange, so its flame is brighter than its light");
}

#[test]
fn the_pool_hands_a_fire_a_light_of_its_colour_and_brightness() {
    let mut app = app();
    let green = Color::from(LinearRgba::rgb(0.2, 1.0, 0.3));
    let fire = app
        .world_mut()
        .spawn((
            Fire { color: Some(green), lumens: Some(12_000.0), ..Fire::new(FireKind::Brazier) },
            Transform::from_xyz(3.0, 0.0, 0.0),
        ))
        .id();
    app.world_mut().spawn((TestFocus, Transform::default()));
    for _ in 0..6 {
        app.update();
    }
    let lights: Vec<(Vec3, f32, Color)> = app
        .world_mut()
        .query::<(&PoolLight, &Transform, &Flicker, &PointLight)>()
        .iter(app.world())
        .filter(|(_, _, flicker, _)| flicker.base > 0.0)
        .map(|(_, at, flicker, light)| (at.translation, flicker.base, light.color))
        .collect();
    assert_eq!(lights.len(), 1, "one fire, one lit slot: {lights:?}");
    let (at, base, colour) = lights[0];
    let middle = Vec3::new(3.0, FireKind::Brazier.height() * 0.5, 0.0);
    assert!((at - middle).length() < 1e-3, "the light stands in the flame: {at:?}");
    assert!((base * flicker_mean(FireKind::Brazier.depth()) - 12_000.0).abs() < 1.0, "{base}");
    assert_eq!(colour, green);
    assert!(has::<FlameSite>(&app, fire));
}

#[test]
fn a_fire_with_unusable_settings_names_them() {
    assert!(Fire::default().problems().is_empty());
    assert!(Fire::lit(FireKind::Bonfire, 250.0, 1.0).problems().is_empty());
    let named = |fire: Fire| fire.problems().into_iter().map(|p| p.field).collect::<Vec<_>>();
    assert_eq!(named(Fire { lumens: Some(0.0), ..default() }), ["lumens"]);
    assert_eq!(named(Fire { lumens: Some(f32::NAN), ..default() }), ["lumens"]);
    assert_eq!(named(Fire { phase: Some(f32::INFINITY), ..default() }), ["phase"]);
    assert_eq!(
        named(Fire { color: Some(Color::from(LinearRgba::rgb(-1.0, 0.5, 0.5))), ..default() }),
        ["color.red"]
    );
}

#[cfg(feature = "serde")]
mod wire {
    use super::*;

    #[test]
    fn a_fire_round_trips_and_a_name_is_a_fire() {
        let fire = Fire {
            lumens: Some(8_000.0),
            phase: Some(0.5),
            color: Some(Color::srgb(0.1, 0.3, 1.0)),
            embers: false,
            ..Fire::new(FireKind::Bonfire)
        };
        let back: Fire = serde_json::from_str(&serde_json::to_string(&fire).unwrap()).unwrap();
        assert_eq!(back, fire);
        let bare: Fire = serde_json::from_str(r#"{"kind": "Campfire"}"#).unwrap();
        assert_eq!(bare, Fire::new(FireKind::Campfire));
        assert_eq!(serde_json::from_str::<Fire>("{}").unwrap(), Fire::default());
    }
}
