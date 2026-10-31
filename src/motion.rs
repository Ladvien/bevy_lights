//! [`FlameMotion`] and [`FlameVelocity`]: a fire that moves leans against its own motion.
//!
//! A flame leans downwind of the air *relative to the fire*, so a torch carried through still air has a wind of its own: the flame
//! streams behind its bearer, more at a jog, and stands up again over about half a second when they stop. [`FlamePlugin`](crate::FlamePlugin)
//! tracks every [`FlameSite`]'s ground velocity from its `GlobalTransform` and [`update_flames`](crate::flame) and the pool's light both
//! lean on the wind less that velocity, so the flame, its sparks and smoke, and the light standing in it agree (`docs/research/flame-motion.md`).

use bevy::prelude::*;

use crate::FlameSite;
use crate::model::{MOTION_RESPONSE, smooth_velocity};

/// How a fire that moves feels its own motion: the wind it leans in is the wind less `gain` times its velocity.
///
/// `gain` 1 is physics (a torch walked at 1.4 m/s meets a 1.4 m/s wind) and 0 is the crate as it was, every flame leaning only in
/// [`FlameWind`](crate::FlameWind). `response` is the time constant of the filter that turns the site's steps into a velocity, in
/// seconds: a fixed-step character drawn at the display's rate steps unevenly, and a swinging hand reverses each half second, and
/// the flame should follow neither frame by frame. [`FlamePlugin`](crate::FlamePlugin) adds it with the defaults.
///
/// A fire whose site moves by more than [`MAX_TRACKED_SPEED`](crate::MAX_TRACKED_SPEED) in a frame has been teleported and takes no
/// velocity from it. The tilt law is SI, so a host whose world is not in metres converts first, as for the wind.
///
/// ```
/// # use bevy::prelude::*;
/// # use bevy_lights::*;
/// # fn setup(mut commands: Commands) {
/// // Half the physical lean: a calmer carried flame.
/// commands.insert_resource(FlameMotion { gain: 0.5, ..default() });
/// # }
/// ```
#[derive(Resource, Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
#[reflect(Resource, Default)]
pub struct FlameMotion {
    /// How much of a fire's own velocity counts as wind. `1.0` (the default) is physical; `0.0` turns the lean off.
    pub gain: f32,
    /// The time constant of the velocity filter, in seconds ([`MOTION_RESPONSE`](crate::MOTION_RESPONSE) by default).
    pub response: f32,
}

impl Default for FlameMotion {
    fn default() -> Self {
        Self { gain: 1.0, response: MOTION_RESPONSE }
    }
}

impl FlameMotion {
    /// What is unusable in these settings, field by field; empty when they are fine. See
    /// [`Flicker::problems`](crate::Flicker::problems).
    pub fn problems(&self) -> Vec<crate::Problem> {
        use crate::model::problem::{non_negative, positive};
        let mut out = Vec::new();
        non_negative(&mut out, "gain", self.gain);
        positive(&mut out, "response", self.response);
        out
    }
}

/// A [`FlameSite`]'s own velocity on the ground plane, in metres per second: what [`FlameMotion`] turns into wind.
///
/// [`FlameSite`] requires it and [`FlamePlugin`](crate::FlamePlugin) writes it every frame from the site's `GlobalTransform`, so a host
/// reads it (a carried fire's speed is the fire's own, whoever moved it) and never writes it. It is zero for a fire that has not
/// moved, and for a fire's first two frames, while its transform settles.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Default)]
pub struct FlameVelocity {
    /// The smoothed velocity, on the world's XZ plane (`x` is the world's X and `y` its Z), as [`FlameWind::velocity`](crate::FlameWind::velocity).
    pub ground: Vec2,
    /// Where the site was when it was last measured, once it has been seen long enough to trust.
    last: Option<Vec3>,
    /// How many frames the site has been seen: the first is the site spawned with an identity `GlobalTransform` that propagation has
    /// not yet written, the second its real position, and only from the third is a step a motion and not a spawn.
    seen: u8,
}

/// How many sites are moving this frame, so the pool can skip its search for a light's site on a frame where nothing moves and
/// there is no wind.
#[derive(Resource, Debug, Default)]
pub(crate) struct MovingSites(pub(crate) usize);

/// Differences each site's world position into its smoothed ground velocity.
///
/// Runs ahead of the pool, so the light and the flame lean on the same frame's velocity. A frame with no virtual time passing (a paused
/// world, the first frame) changes nothing. A site that has not moved since it was last measured and whose velocity is already zero
/// is skipped: a fire that stands still costs one comparison.
pub(crate) fn track_flame_velocity(
    time: Res<Time>,
    motion: Option<Res<FlameMotion>>,
    mut moving: ResMut<MovingSites>,
    mut sites: Query<(&GlobalTransform, &mut FlameVelocity), With<FlameSite>>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let response = motion.as_deref().map_or(MOTION_RESPONSE, |m| m.response);
    let mut count = 0;
    for (global, mut velocity) in &mut sites {
        let here = global.translation();
        if velocity.seen >= 2 && velocity.ground == Vec2::ZERO && velocity.last == Some(here) {
            continue;
        }
        if velocity.seen < 2 {
            velocity.seen += 1;
            velocity.last = Some(here);
            continue;
        }
        let step = velocity.last.map_or([0.0, 0.0], |last| [here.x - last.x, here.z - last.z]);
        let previous = velocity.ground.to_array();
        let next = Vec2::from(smooth_velocity(previous, step, dt, response));
        if next != velocity.ground {
            velocity.ground = next;
        }
        if velocity.last != Some(here) {
            velocity.last = Some(here);
        }
        if next != Vec2::ZERO {
            count += 1;
        }
    }
    if moving.0 != count {
        moving.0 = count;
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    const STEP: f32 = 1.0 / 60.0;

    fn site() -> FlameSite {
        FlameSite {
            height: 0.35,
            phase: 0.0,
            kindle: 0.0,
            diameter: Some(0.1),
            depth: crate::FLICKER_DEPTH,
            depth_fade: false,
            fire: None,
            color: None,
        }
    }

    /// The tracker alone, on a fixed 1/60 s step, with the transform propagation the host's own `PostUpdate` runs.
    fn tracking_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::transform::TransformPlugin)
            .init_resource::<FlameMotion>()
            .init_resource::<MovingSites>()
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(STEP)))
            .add_systems(Update, track_flame_velocity);
        app.update();
        app
    }

    fn velocity(app: &App, site: Entity) -> Vec2 {
        app.world().get::<FlameVelocity>(site).map(|v| v.ground).unwrap_or(Vec2::NAN)
    }

    fn walk(app: &mut App, site: Entity, speed: Vec3, frames: usize) {
        for _ in 0..frames {
            if let Some(mut at) = app.world_mut().get_mut::<Transform>(site) {
                at.translation += speed * STEP;
            }
            app.update();
        }
    }

    #[test]
    fn a_site_walked_at_a_pace_feels_that_pace_and_stops_when_it_stops() {
        let mut app = tracking_app();
        let site = app.world_mut().spawn((site(), Transform::from_xyz(10.0, 0.0, 4.0))).id();
        walk(&mut app, site, Vec3::new(1.4, 0.0, -0.7), 60);
        let v = velocity(&app, site);
        assert!((v - Vec2::new(1.4, -0.7)).length() < 0.02, "{v:?}");
        assert_eq!(app.world().resource::<MovingSites>().0, 1);
        walk(&mut app, site, Vec3::ZERO, 120);
        assert_eq!(velocity(&app, site), Vec2::ZERO, "a stopped fire is a still fire");
        assert_eq!(app.world().resource::<MovingSites>().0, 0);
    }

    #[test]
    fn a_site_spawned_away_from_the_origin_is_not_a_site_that_arrived_from_it() {
        // Its `GlobalTransform` is the identity on the frame it is spawned and its real place only after propagation: the step between
        // the two is a spawn, not a motion, even when it is small enough to pass for a walk.
        let mut app = tracking_app();
        let site = app.world_mut().spawn((site(), Transform::from_xyz(0.4, 0.0, 0.0))).id();
        for _ in 0..30 {
            app.update();
            assert_eq!(velocity(&app, site), Vec2::ZERO);
        }
    }

    #[test]
    fn a_site_carried_by_a_moving_parent_moves_with_it_and_its_own_swing_adds() {
        let mut app = tracking_app();
        let site = app.world_mut().spawn((site(), Transform::from_xyz(0.0, 1.0, 0.0))).id();
        let bearer = app.world_mut().spawn(Transform::from_xyz(3.0, 0.0, 3.0)).add_child(site).id();
        walk(&mut app, bearer, Vec3::new(0.0, 0.0, 1.0), 90);
        let v = velocity(&app, site);
        assert!((v - Vec2::new(0.0, 1.0)).length() < 0.02, "{v:?}");
    }

    #[test]
    fn a_teleport_is_not_a_velocity() {
        let mut app = tracking_app();
        let site = app.world_mut().spawn((site(), Transform::from_xyz(1.0, 0.0, 1.0))).id();
        walk(&mut app, site, Vec3::new(1.0, 0.0, 0.0), 60);
        let before = velocity(&app, site);
        app.world_mut().get_mut::<Transform>(site).unwrap().translation += Vec3::new(200.0, 0.0, 0.0);
        // The tracker reads the transform the last frame's propagation wrote: two updates see the jump.
        app.update();
        app.update();
        assert!(
            (velocity(&app, site) - before).length() < 0.02,
            "a 200 m step in a frame is a respawn: {before:?}"
        );
        walk(&mut app, site, Vec3::new(1.0, 0.0, 0.0), 30);
        assert!((velocity(&app, site).x - 1.0).abs() < 0.05, "and the walk goes on from where it landed");
    }

    #[test]
    fn a_paused_world_holds_the_velocity_it_had() {
        let mut app = tracking_app();
        let site = app.world_mut().spawn((site(), Transform::default())).id();
        walk(&mut app, site, Vec3::X, 60);
        let moving = velocity(&app, site);
        app.world_mut().resource_mut::<Time<Virtual>>().pause();
        // A paused world's transforms do not move; the flame holds the lean it had.
        for _ in 0..30 {
            app.update();
        }
        assert_eq!(velocity(&app, site), moving);
    }

    #[test]
    fn motion_settings_are_checked_like_the_crates_other_settings() {
        assert!(FlameMotion::default().problems().is_empty());
        let bad = FlameMotion { gain: -1.0, response: 0.0 };
        let fields: Vec<_> = bad.problems().iter().map(|p| p.field).collect();
        assert_eq!(fields, ["gain", "response"]);
    }
}
