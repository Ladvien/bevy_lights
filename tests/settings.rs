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
fn a_wind_names_the_component_that_is_not_a_number() {
    assert!(FlameWind::default().problems().is_empty());
    let wind = FlameWind { velocity: bevy::math::Vec2::new(f32::NAN, 2.0) };
    assert_eq!(fields(wind.problems()), ["velocity.x"]);
}

#[test]
fn a_pool_reports_reach_hysteresis_and_the_template_light() {
    let mut pool = LightPool { reach: 0.0, hysteresis: 0.5, fade_seconds: f32::NAN, ..Default::default() };
    pool.light.range = -3.0;
    assert_eq!(fields(pool.problems()), ["reach", "hysteresis", "fade_seconds", "light.range"]);
    assert!(LightPool { size: 0, ..Default::default() }.problems().is_empty());
}

#[test]
fn a_pool_candidate_and_a_kindle_report_their_bad_fields() {
    assert!(PoolCandidate::default().problems().is_empty());
    assert!(PoolCandidate::at_mean(900.0, 0.3).problems().is_empty());
    let bad = PoolCandidate {
        offset: bevy::math::Vec3::new(0.0, f32::NAN, f32::INFINITY),
        intensity: -1.0,
        phase: Some(f32::NAN),
        diameter: Some(0.0),
        depth: Some(MAX_FLICKER_DEPTH + 0.1),
    };
    assert_eq!(fields(bad.problems()), ["offset.y", "offset.z", "intensity", "phase", "diameter", "depth"]);
    assert!(Kindle::default().problems().is_empty());
    assert_eq!(fields(Kindle { elapsed: -0.5 }.problems()), ["elapsed"]);
    assert_eq!(fields(Kindle { elapsed: f32::NAN }.problems()), ["elapsed"]);
}

/// The preview curve is the same function the system writes: a fire flares up while kindling and
/// matches an already-burning one once the kindle has run its course.
#[test]
fn intensity_at_follows_the_kindle_and_then_the_flicker() {
    let fire = Flicker::at_mean(1000.0, 0.5);
    for t in [0.0, 0.37, 2.9] {
        let burning = fire.intensity_at(t, None);
        assert!(burning > 0.0);
        assert!((fire.intensity_at(t, Some(KINDLE_SECONDS)) - burning).abs() < burning * 1e-5);
        assert_ne!(fire.intensity_at(t, Some(0.0)), burning);
    }
    assert_eq!(Flicker { base: 0.0, ..fire }.intensity_at(1.0, None), 0.0);
}

#[cfg(feature = "serde")]
#[allow(clippy::unwrap_used)]
mod saved_settings {
    use bevy_lights::{FlameLook, FlameWind, Flicker, Kindle};

    #[test]
    fn a_file_naming_only_some_fields_takes_the_defaults_for_the_rest() {
        let f: Flicker = serde_json::from_str(r#"{"base": 1234.0}"#).unwrap();
        assert_eq!(f.base, 1234.0);
        assert_eq!(Flicker { base: 0.0, ..f }, Flicker { base: 0.0, ..Flicker::default() });

        let look: FlameLook = serde_json::from_str("{}").unwrap();
        assert_eq!(look, FlameLook::default());

        let wind: FlameWind = serde_json::from_str("{}").unwrap();
        assert_eq!(wind, FlameWind::default());

        let k: Kindle = serde_json::from_str("{}").unwrap();
        assert_eq!(k, Kindle::default());
    }
}

/// A picker's six entries are all usable fires, and the light and the flame drawn on it agree.
#[test]
fn every_fire_kind_is_a_sound_fire_whose_light_and_flame_agree() {
    for kind in FireKind::ALL {
        let light = kind.flicker(800.0);
        let site = kind.site();
        assert!(light.problems().is_empty(), "{}", kind.name());
        assert!(site.problems().is_empty(), "{}", kind.name());
        assert!((light.mean() - 800.0).abs() < 800.0 * 1e-5, "{}", kind.name());
        assert_eq!((light.diameter, light.depth), (site.diameter, site.depth), "{}", kind.name());
        assert!(site.height <= FlameLook::default().max_height, "{}", kind.name());
    }
}

/// Bigger fires flicker slower, and the list runs smallest first.
#[test]
fn fire_kinds_slow_down_as_they_grow() {
    let rates: Vec<f32> = FireKind::ALL.iter().map(|k| k.rate()).collect();
    assert!(rates.windows(2).all(|w| w[0] > w[1]), "{rates:?}");
}

/// A bigger fire lights more ground, and its light budget follows the inverse square of that reach.
#[test]
fn fire_kinds_light_more_ground_as_they_grow() {
    let reaches: Vec<f32> = FireKind::ALL.iter().map(|k| k.reach()).collect();
    assert!(reaches.windows(2).all(|w| w[0] < w[1]), "{reaches:?}");
    assert!(reaches[0] >= 1.0, "below fire_lumens' floor a kind's reach would be silently raised");
    for kind in FireKind::ALL {
        let lumens = kind.lumens(12.0, 0.9);
        assert_eq!(lumens, fire_lumens(12.0, kind.reach(), 0.9), "{}", kind.name());
        assert!((kind.light(12.0, 0.9).mean() - lumens).abs() < lumens * 1e-5, "{}", kind.name());
    }
    let ratio = FireKind::Bonfire.lumens(12.0, 0.0) / FireKind::Candle.lumens(12.0, 0.0);
    assert!((ratio - 81.0).abs() < 1e-2, "9 m against 1 m at equal lux is 81x: {ratio}");
}

/// An editor draws the lit circle round a placed fire and paints the ground's lux from the same
/// budget the kind's lumens came from: the reach it shows is the reach the kind was sized for.
#[test]
fn a_fires_lit_circle_is_the_reach_it_was_sized_for() {
    for kind in FireKind::ALL {
        let lumens = kind.lumens(12.0, 0.9);
        let lux = fire_lux(lumens, kind.reach(), 0.9);
        assert!((lux - 12.0).abs() < 12.0 * 1e-4, "{}: {lux}", kind.name());
        let reach = fire_reach(lumens, 12.0, 0.9);
        assert!((reach - kind.reach()).abs() < kind.reach() * 1e-3, "{}: {reach}", kind.name());
    }
    // A threshold brighter than the ground under the light has no circle.
    assert_eq!(fire_reach(fire_lumens(12.0, 4.0, 0.9), 1.0e6, 0.9), 0.0);
    assert_eq!(fire_reach(1000.0, 0.0, 0.0), 0.0, "no lux threshold, no finite circle");
}

#[cfg(feature = "serde")]
#[test]
#[allow(clippy::unwrap_used)]
fn a_fire_kind_round_trips_as_its_name() {
    for kind in FireKind::ALL {
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, format!("\"{}\"", kind.name()));
        assert_eq!(serde_json::from_str::<FireKind>(&json).unwrap(), kind);
    }
}

/// A picker's text field and a command line name a kind the way `Display` writes it, in any case;
/// an unknown word is an error that lists the valid ones.
#[test]
#[allow(clippy::unwrap_used)]
fn a_fire_kind_parses_the_name_it_displays() {
    for kind in FireKind::ALL {
        assert_eq!(kind.to_string().parse(), Ok(kind));
        assert_eq!(kind.name().to_uppercase().parse(), Ok(kind));
    }
    let err = "pyre".parse::<FireKind>().unwrap_err();
    assert_eq!(err, UnknownFireKind("pyre".into()));
    let msg = err.to_string();
    assert!(FireKind::ALL.iter().all(|k| msg.contains(k.name())), "{msg}");
}
