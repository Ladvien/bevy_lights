//! `problems()` on the settings types: what an editor shows beside a field.

use bevy_lights::*;

fn fields(p: Vec<Problem>) -> Vec<&'static str> {
    p.into_iter().map(|p| p.field).collect()
}

#[test]
fn the_defaults_and_a_sane_fire_have_no_problems() {
    assert!(Flicker { base: 800.0, ..Default::default() }.problems().is_empty());
    assert!(Flicker::at_mean(1000.0, WHIPPING_FLICKER_DEPTH).problems().is_empty());
    let site = FlameSite {
        height: 1.0,
        phase: 0.0,
        kindle: 0.0,
        diameter: None,
        depth: FLICKER_DEPTH,
        depth_fade: false,
        fire: None,
    };
    assert!(site.problems().is_empty());
    assert!(FlameLook::default().problems().is_empty());
    assert!(LightPool::default().problems().is_empty());
}

#[test]
fn a_flicker_names_every_bad_field() {
    let bad = Flicker { base: -1.0, phase: f32::NAN, diameter: Some(0.0), depth: 0.9 };
    assert_eq!(fields(bad.problems()), ["base", "phase", "diameter", "depth"]);
    assert_eq!(bad.problems()[3].kind, ProblemKind::OutOfRange { min: 0.0, max: MAX_FLICKER_DEPTH });
}

#[test]
fn a_flame_site_and_look_report_their_own_fields() {
    let site = FlameSite {
        height: 0.0,
        phase: 0.0,
        kindle: -2.0,
        diameter: None,
        depth: f32::INFINITY,
        depth_fade: false,
        fire: None,
    };
    assert_eq!(fields(site.problems()), ["height", "kindle", "depth"]);
    let look = FlameLook { strength: -0.5, max_height: 0.0, ..Default::default() };
    assert_eq!(fields(look.problems()), ["strength", "max_height"]);
}

#[test]
fn a_pool_reports_reach_hysteresis_and_the_template_light() {
    let mut pool = LightPool { reach: 0.0, hysteresis: 0.5, fade_seconds: f32::NAN, ..Default::default() };
    pool.light.range = -3.0;
    assert_eq!(fields(pool.problems()), ["reach", "hysteresis", "fade_seconds", "light.range"]);
    assert!(LightPool { size: 0, ..Default::default() }.problems().is_empty());
}
