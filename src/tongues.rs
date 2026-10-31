//! Tongues: a wide fire as several flames.
//!
//! A [`Flame`] is one teardrop, `0.4` of its height wide: a candle's and a torch's shape, and the wrong one for a campfire's 0.8 m bed under
//! a 0.7 m flame or a bonfire's 3 m bed under a 2 m one, which drew one slender tongue in the middle of a heap of fuel. Add
//! [`FlameTongues`] to a [`FlameSite`] and the site keeps its own flame as the middle tongue and gains `count - 1` more [`Flame`]s round it,
//! each a [`FlameTongue`], of different heights and clocks laid out by [`flame_tongue`](crate::flame_tongue).
//!
//! A tongue is an ordinary flame, a child of the site with its own offset, so everything a flame does it does: it catches and goes out
//! with its fire, leans with the wind, fades behind a wall edge by the depth fade and is culled by a box of its own. A fire that does not
//! ask for tongues is drawn exactly as it always was. The design and its sources are in `docs/research/flame-tongues.md`.

use bevy::prelude::*;

use crate::flame::{Flame, FlameAssets, FlameLook, FlameMaterial, FlameSite, Flamed, flame_parts};
use crate::model::{MAX_TONGUES, flame_tongue};

/// A wide fire's tongues: add it to a [`FlameSite`] to draw the fire as `count` flames over a bed `spread` wide, remove it to go back to one.
///
/// The site's own flame is the middle tongue (at the site's origin and [`FlameSite::height`], the tallest); the other `count - 1` stand on
/// the bed round it, shorter the further out they are and each on a clock of its own, so the fire is a mound of tongues that part and rejoin
/// as one fire breathes. A tongue never stands taller than three times its distance from the middle one, the height at which two flames that
/// far apart are one flame (`docs/research/flame-tongues.md`). Each tongue is one more draw, so give the component to the fires that are
/// wider than they are tall (a campfire, a bonfire: [`FireKind::tongues`](crate::FireKind::tongues) says which) and not to a torch.
///
/// The sparks ([`FlameEmbers`](crate::FlameEmbers)) and the smoke ([`FlameSmoke`](crate::FlameSmoke)) stay one quad each, on the middle
/// tongue. Changing the component at run time rebuilds the extra tongues on the next frame; removing it takes them away. The default is a
/// campfire's.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
#[reflect(Component, Default)]
pub struct FlameTongues {
    /// How many tongues, `1..=`[`MAX_TONGUES`], the site's own among them. `1` is the plain flame.
    pub count: u8,
    /// The width of the bed the tongues' feet are spread over, in the host's units. The feet lie between 0.4 and 1.0 of its radius from the
    /// middle, so a fire's tongues are as wide as its bed is.
    pub spread: f32,
}

impl Default for FlameTongues {
    fn default() -> Self {
        Self { count: 5, spread: 0.5 }
    }
}

impl FlameTongues {
    /// What is unusable in these settings, field by field; empty when they are fine. See
    /// [`Flicker::problems`](crate::Flicker::problems).
    pub fn problems(&self) -> Vec<crate::Problem> {
        let mut out = Vec::new();
        crate::model::problem::within(&mut out, "count", f32::from(self.count), 1.0, f32::from(MAX_TONGUES));
        crate::model::problem::positive(&mut out, "spread", self.spread);
        out
    }
}

/// An extra tongue of a [`FlameTongues`] fire: a [`Flame`] that is not the site's own, the one a host counts a fire's tongues by
/// (`With<FlameTongue>`) and that the sparks and the smoke skip. The host never spawns one.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component)]
pub struct FlameTongue {
    /// Which tongue, `1..`[`MAX_TONGUES`]: the middle one (`0`) is the site's own flame.
    pub index: u8,
    /// How the tongue leans towards the middle one: the sine of its tilt times the unit direction to it, in the site's own frame
    /// (`[x, z]` of the site's axes). [`FlamePlugin`](crate::FlamePlugin) turns it to the world's axes and adds it to the wind's lean
    /// each frame, so a fire's outer tongues lean in and the fire converges on a tip.
    pub lean: Vec2,
}

/// What a site's extra tongues were last built from, so a frame in which nothing changed does nothing.
#[derive(Component, Clone, Copy, PartialEq)]
pub(crate) struct Tongued {
    count: u8,
    spread: f32,
    height: f32,
    phase: f32,
}

/// Keeps every [`FlameTongues`] site's extra tongues in step with the component: builds them when a site has been dressed, rebuilds them
/// when the count, the spread or the site's height or phase changes, and takes them away when the component is removed.
///
/// The extras are spawned through `queue_silenced`, like the site's own flame: a site despawned in the frame it was dressed must not
/// panic. A new tongue starts with the middle flame's catch clock, so a fire still catching has its tongues catch together and a fire that
/// has caught grows its new tongue in a settled state.
#[allow(clippy::too_many_arguments)]
pub(crate) fn dress_tongues(
    mut commands: Commands,
    sites: Query<(Entity, &FlameSite, &FlameTongues, Option<&Tongued>, Option<&Children>), With<Flamed>>,
    mut removed: RemovedComponents<FlameTongues>,
    children: Query<&Children>,
    tongues: Query<(), With<FlameTongue>>,
    middles: Query<&Flame, Without<FlameTongue>>,
    mut assets: ResMut<FlameAssets>,
    mut materials: ResMut<Assets<FlameMaterial>>,
    look: Res<FlameLook>,
    time: Res<Time>,
) {
    for site in removed.read() {
        let Ok(kids) = children.get(site) else { continue };
        for kid in kids.iter().filter(|kid| tongues.contains(*kid)) {
            commands.entity(kid).try_despawn();
        }
        commands.entity(site).try_remove::<Tongued>();
    }
    if !assets.built() {
        return;
    }
    let t = time.elapsed_secs();
    for (entity, site, wanted, built, kids) in &sites {
        let count = wanted.count.clamp(1, MAX_TONGUES);
        let now = Tongued { count, spread: wanted.spread, height: site.height, phase: site.phase };
        if built == Some(&now) {
            continue;
        }
        let kids: &[Entity] = kids.map_or(&[], |kids| &kids[..]);
        for kid in kids.iter().filter(|kid| tongues.contains(**kid)) {
            commands.entity(*kid).try_despawn();
        }
        let kindled = kids.iter().find_map(|kid| middles.get(*kid).ok()).map_or(f32::MAX, |middle| middle.kindled);
        let mut extras = Vec::with_capacity(usize::from(count));
        for index in 1..count {
            let tongue = flame_tongue(index, count, wanted.spread, site.height, site.phase);
            let offset = Vec3::new(tongue.foot[0], 0.0, tongue.foot[1]);
            let parts =
                flame_parts(site, tongue.height, site.phase + tongue.phase, kindled, offset, &mut assets, &mut materials, &look, t);
            extras.push((Name::new("Flame tongue"), FlameTongue { index, lean: Vec2::from(tongue.lean) }, parts));
        }
        commands.entity(entity).queue_silenced(move |mut site: EntityWorldMut| {
            site.insert(now).with_children(|parent| {
                for extra in extras {
                    parent.spawn(extra);
                }
            });
        });
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::flame::{build_flame_assets, dress_flames, update_flames};
    use crate::model::{FLICKER_DEPTH, MERGE_RATIO, unpack_flame_tag};
    use bevy::mesh::MeshTag;

    fn tongue_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Assets::<Mesh>::default())
            .insert_resource(Assets::<FlameMaterial>::default())
            .init_resource::<FlameLook>()
            .init_resource::<FlameAssets>()
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(
                1.0 / 60.0,
            )));
        app.add_systems(Startup, build_flame_assets)
            .add_systems(Update, (dress_flames, dress_tongues, update_flames).chain());
        app.update();
        app
    }

    fn site(height: f32, phase: f32) -> FlameSite {
        FlameSite {
            color: None,
            height,
            phase,
            kindle: 0.0,
            diameter: Some(0.8),
            depth: FLICKER_DEPTH,
            depth_fade: false,
            fire: None,
        }
    }

    /// Every flame of `site`, the middle one first: `(entity, is a tongue, transform, height)`.
    fn flames_of(app: &mut App, site: Entity) -> Vec<(Entity, bool, Transform, f32)> {
        let max_height = app.world().resource::<FlameAssets>().max_height();
        let mut found: Vec<_> = app
            .world_mut()
            .query_filtered::<(Entity, &ChildOf, &Transform, &MeshTag, Has<FlameTongue>), With<Flame>>()
            .iter(app.world())
            .filter(|(_, parent, ..)| parent.parent() == site)
            .map(|(entity, _, at, tag, tongue)| (entity, tongue, *at, unpack_flame_tag(tag.value, max_height).1))
            .collect();
        found.sort_by_key(|(_, tongue, ..)| *tongue);
        found
    }

    #[test]
    fn a_site_with_tongues_has_that_many_flames_and_the_middle_one_is_its_own() {
        let mut app = tongue_app();
        let at = app
            .world_mut()
            .spawn((site(0.7, 0.4), FlameTongues { count: 5, spread: 0.5 }, Transform::default()))
            .id();
        app.update();
        app.update();
        let flames = flames_of(&mut app, at);
        assert_eq!(flames.len(), 5, "five flames");
        assert!(!flames[0].1, "the first is the site's own, not a tongue");
        assert_eq!(flames[0].2.translation, Vec3::ZERO);
        assert!((flames[0].3 - 0.7).abs() < 1e-3);
        assert!(flames[1..].iter().all(|(_, tongue, ..)| *tongue));
        for (_, _, at, height) in &flames[1..] {
            let distance = at.translation.x.hypot(at.translation.z);
            assert!(distance <= 0.25 + 1e-5 && distance > 0.0, "a foot on the bed: {distance}");
            assert_eq!(at.translation.y, 0.0, "a foot on the bed's plane");
            assert!(*height < 0.7 && *height <= MERGE_RATIO * distance + 1e-3, "{height} at {distance}");
        }
    }

    #[test]
    fn a_second_frame_does_not_twin_the_tongues_and_a_site_without_the_component_has_one_flame() {
        let mut app = tongue_app();
        let wide = app.world_mut().spawn((site(0.7, 0.4), FlameTongues { count: 3, spread: 0.5 })).id();
        let plain = app.world_mut().spawn(site(0.35, 1.0)).id();
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(flames_of(&mut app, wide).len(), 3);
        assert_eq!(flames_of(&mut app, plain).len(), 1);
    }

    #[test]
    fn changing_the_component_rebuilds_the_tongues_and_removing_it_takes_them_away() {
        let mut app = tongue_app();
        let at = app.world_mut().spawn((site(0.7, 0.4), FlameTongues { count: 5, spread: 0.5 })).id();
        app.update();
        app.update();
        assert_eq!(flames_of(&mut app, at).len(), 5);
        let before: Vec<_> = flames_of(&mut app, at).iter().map(|f| f.2.translation).collect();
        app.world_mut().entity_mut(at).get_mut::<FlameTongues>().unwrap().count = 3;
        app.update();
        app.update();
        assert_eq!(flames_of(&mut app, at).len(), 3, "two tongues fewer");
        app.world_mut().entity_mut(at).get_mut::<FlameTongues>().unwrap().spread = 1.0;
        app.update();
        app.update();
        let wider = flames_of(&mut app, at);
        assert_eq!(wider.len(), 3, "a wider bed keeps the count");
        assert!(
            wider[1..].iter().any(|f| f.2.translation.x.hypot(f.2.translation.z) > 0.25),
            "and puts a foot beyond the old bed"
        );
        assert_ne!(before[1], wider[1].2.translation);
        app.world_mut().entity_mut(at).remove::<FlameTongues>();
        app.update();
        app.update();
        let one = flames_of(&mut app, at);
        assert_eq!(one.len(), 1, "back to the site's own flame");
        assert!(!one[0].1);
    }

    #[test]
    fn an_unchanged_fire_does_not_rebuild_its_tongues() {
        let mut app = tongue_app();
        let at = app.world_mut().spawn((site(0.7, 0.4), FlameTongues::default())).id();
        app.update();
        app.update();
        let first: Vec<Entity> = flames_of(&mut app, at).iter().map(|f| f.0).collect();
        // A write that changes nothing (a panel's slider released where it was) marks the component changed.
        app.world_mut().entity_mut(at).get_mut::<FlameTongues>().unwrap().spread = 0.5;
        app.update();
        app.update();
        assert_eq!(flames_of(&mut app, at).iter().map(|f| f.0).collect::<Vec<_>>(), first, "the same entities");
    }

    #[test]
    fn two_fires_do_not_lay_their_tongues_alike() {
        let mut app = tongue_app();
        let one = app.world_mut().spawn((site(0.7, 0.4), FlameTongues::default())).id();
        let other = app.world_mut().spawn((site(0.7, 2.2), FlameTongues::default())).id();
        app.update();
        app.update();
        let feet = |app: &mut App, site| -> Vec<Vec3> { flames_of(app, site).iter().map(|f| f.2.translation).collect() };
        assert_ne!(feet(&mut app, one), feet(&mut app, other));
    }

    #[test]
    fn the_tongues_are_all_one_fires_catch() {
        let mut app = tongue_app();
        let at = app
            .world_mut()
            .spawn((FlameSite { kindle: 0.6, ..site(0.7, 0.4) }, FlameTongues { count: 4, spread: 0.5 }))
            .id();
        app.update();
        app.update();
        let flames = flames_of(&mut app, at);
        assert_eq!(flames.len(), 4);
        let scale = flames[0].2.scale.x;
        assert!(scale < 1.0, "the fire is still catching: {scale}");
        for flame in &flames[1..] {
            assert!((flame.2.scale.x - scale).abs() < 1e-6, "every tongue catches with the fire: {flame:?}");
        }
    }

    #[test]
    fn a_bad_count_or_spread_is_reported_and_a_good_one_is_not() {
        assert!(FlameTongues::default().problems().is_empty());
        assert!(FlameTongues { count: 1, spread: 0.1 }.problems().is_empty());
        let fields = |t: FlameTongues| t.problems().iter().map(|p| p.field).collect::<Vec<_>>();
        assert_eq!(fields(FlameTongues { count: 0, spread: 0.5 }), ["count"]);
        assert_eq!(fields(FlameTongues { count: MAX_TONGUES + 1, spread: 0.5 }), ["count"]);
        assert_eq!(fields(FlameTongues { count: 3, spread: 0.0 }), ["spread"]);
        assert_eq!(fields(FlameTongues { count: 3, spread: f32::NAN }), ["spread"]);
    }
}
