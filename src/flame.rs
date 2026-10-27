//! Billboarded shader flames: the light a fire has with no light at all.
//!
//! A flame is drawn on every [`FlameSite`]: an additive billboard (`src/flame.wesl`) that stands on
//! the site's origin, [`FlameSite::height`] world units tall, and animated by one shared clock plus
//! the site's own phase. It lights nothing and casts no shadow, so a street of a hundred braziers
//! costs one draw per flame and nothing in the clustered-lighting pass.
//!
//! Why a shader rather than a sprite: an additive billboard tinted by the scene's fire colour reads
//! as fire under any exposure, and one shader's worth of fbm noise is cheaper than the art. The
//! shape, its noise and their provenance are documented at the top of `src/flame.wesl`.

use bevy::asset::embedded_asset;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::NotShadowCaster;
use bevy::mesh::{MeshTag, MeshVertexBufferLayoutRef};
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin, MeshMaterial3d};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError};
use bevy::shader::ShaderRef;
use bevy::time::Real;

use crate::LightsSystems;
use crate::chain_sets;
use crate::model::{flicker_swing, kindle_gain, pack_flame_tag, set_flame_brightness};

/// The flame shader, embedded in the binary by [`FlamePlugin`].
const FLAME_SHADER_PATH: &str = "embedded://bevy_lights/flame.wesl";

/// [`FlamePlugin`]: draws a flame on every [`FlameSite`] and ticks their shared clock.
///
/// Adds `MaterialPlugin::<FlameMaterial>`, so it needs a render app (any app with a window or a
/// render target). A headless test that only exercises the dressing uses the private systems
/// directly, the way `src/flame.rs`'s own tests do.
pub struct FlamePlugin;

impl Plugin for FlamePlugin {
    fn build(&self, app: &mut App) {
        chain_sets(app);
        embedded_asset!(app, "flame.wesl");
        app.add_plugins(MaterialPlugin::<FlameMaterial>::default())
            .init_resource::<FlameLook>()
            .init_resource::<FlameAssets>()
            .register_type::<FlameLook>()
            .register_type::<FlameSite>()
            .add_systems(Update, (dress_flames, update_flames).chain().in_set(LightsSystems::Flame))
            .add_systems(Startup, build_flame_assets);
    }
}

/// The flame material: the host's fire colour (`.a` is the ember's strength, which scales the flame)
/// and the animation's clock and shape.
///
/// A host never builds one of these. [`FlamePlugin`] keeps a single material for every flame in the
/// scene, because the numbers that make two fires burn differently ride on each flame's
/// [`MeshTag`], not in the material. Change [`FlameLook`] to change the colour, the strength or the
/// height scale.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct FlameMaterial {
    #[uniform(0)]
    base: LinearRgba,
    /// `x` seconds (wrapped hourly), `y` the tag's height scale in world units ([`FlameLook::max_height`]),
    /// `z` unused (the per-flame catch is read from each flame's own instance scale, so one fire's catch
    /// cannot tint another's), `w` the ember strength.
    #[uniform(0)]
    params: Vec4,
}

impl Material for FlameMaterial {
    fn vertex_shader() -> ShaderRef {
        FLAME_SHADER_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        FLAME_SHADER_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        // Additive: a flame adds light over what stands behind it, and hides nothing.
        AlphaMode::Add
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(depth) = descriptor.depth_stencil.as_mut() {
            depth.depth_write_enabled = Some(false);
        }
        // Seen from either side, whichever way the quad's winding falls as the camera orbits.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// A fire drawn where no light is: a torch's head, a sconce's cage, a brazier whose light is a
/// child entity.
///
/// Spawn it as a child of whatever the flame stands on (a torch in a hand, a brazier's coals), at
/// the local offset the flame's foot belongs. [`FlamePlugin`] gives it a [`Flame`] child the first
/// frame it sees it; nothing else is needed. Two sites with different [`FlameSite::phase`] values
/// burn out of step forever.
// `Visibility` (which pulls in `InheritedVisibility` and `ViewVisibility`), because a site always becomes a parent: `dress_flames` spawns
// the `Flame` under it, and Bevy's propagation warns (B0004) and culls wrongly when a parent of a
// visible entity has no visibility of its own.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[require(Visibility)]
#[reflect(Component)]
pub struct FlameSite {
    /// How tall the flame is, in world units.
    pub height: f32,
    /// The flame's clock phase, in radians: the same idea as [`Flicker::phase`](crate::Flicker), and
    /// usually the same number.
    pub phase: f32,
    /// How long the flame takes to reach its full height after it is lit, in seconds: the same envelope
    /// the fire's *light* uses ([`kindle_gain`](crate::kindle_gain)), so the flame and the light it
    /// motivates catch together. The catch scales the flame's **drawn** size as well as its colour, so a
    /// lit fire grows from a point over these seconds instead of appearing whole.
    /// `0.0` (the default) is a fire that has been burning all along and needs
    /// no catch; a host lighting a fire sets it, usually to
    /// [`KINDLE_SECONDS`](crate::KINDLE_SECONDS).
    pub kindle: f32,
    /// The fire's fuel-bed diameter, in the host's world units, when the host knows it: this sizes the
    /// flame's own flicker by the same puffing law the light's uses.
    ///
    /// **Set it to the same value as the `Flicker::diameter` on the light this flame motivates.** The
    /// flame and its light are one fire, and a flame left un-sized while its light is sized breathes at
    /// a different rate from the ground under it — which is the defect `docs/research/flame-flicker.md`
    /// measures. `None` (the default) uses the crate's un-sized rate, matching a light whose
    /// `Flicker::diameter` is `None`.
    pub diameter: Option<f32>,
    /// How far the flame's brightness dips below its peak: **set it to the same value as
    /// [`Flicker::depth`](crate::Flicker::depth) on the light this flame motivates**, or the flame's
    /// breath and its light's disagree. [`FLICKER_DEPTH`](crate::FLICKER_DEPTH) is the default and
    /// matches a light that never chose one.
    pub depth: f32,
}

/// The drawn flame: the child mesh [`FlamePlugin`] spawns under each [`FlameSite`].
///
/// Its `Transform`'s scale is the fire's catch (a `FlameSite` with a [`kindle`](FlameSite::kindle)
/// grows from nothing over that many seconds, and the shader multiplies that scale into the flame's
/// drawn size); its `MeshTag` carries the
/// site's phase and height **and the fire's flicker**, which [`FlamePlugin`] rewrites every frame from
/// the same function the light's intensity comes from. So the flame breathes with the ground under it
/// instead of holding still while the light moves.
///
/// Public so a host can find, count or recolour the flames it has; a host never spawns one itself.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct Flame {
    /// Seconds since this flame's fire was lit, when the site asked for a catch. Counted on
    /// `Time<Real>`, like the flame's own clock, so a paused world's flames still move — the catch is
    /// part of the drawn fire, not of the light's.
    pub kindled: f32,
}

/// How the scene's flames look: the fire colour every flame is tinted from, how strongly it adds,
/// and the height scale a [`FlameSite`]'s height is measured against.
///
/// [`FlamePlugin`] `init_resource`s this, so a host that inserts its own first overrides it. The
/// defaults are a campfire: a warm orange at full strength, four world units of headroom for the
/// tallest flame a scene has.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[reflect(Resource)]
pub struct FlameLook {
    /// The fire's colour: the middle of the flame's ramp, between its cool red rim and its yellow
    /// core.
    pub color: LinearRgba,
    /// The flame's ember strength: how much light it adds. Above 1 blooms under a bright exposure;
    /// below about 0.35 it reads as flat and daytime.
    pub strength: f32,
    /// The tallest flame the scene draws, in world units: a [`FlameSite`]'s height is packed as a
    /// fraction of this, so it must be set **once, before any site spawns**. A site taller than it
    /// draws at the maximum.
    pub max_height: f32,
}

impl Default for FlameLook {
    fn default() -> Self {
        Self { color: LinearRgba::rgb(1.0, 0.55, 0.18), strength: 1.0, max_height: 4.0 }
    }
}

/// The one mesh, the one material, and the height scale every flame in the scene shares.
#[derive(Resource, Default)]
struct FlameAssets {
    mesh: Handle<Mesh>,
    material: Handle<FlameMaterial>,
    /// The height scale the material was built with ([`FlameLook::max_height`] at `Startup`): a
    /// [`FlameSite`]'s height is packed as a fraction of it, so the CPU packer and the shader must
    /// agree on it.
    max_height: f32,
    /// Whether the handles have been made. The mesh and the material are built at `Startup`, after
    /// the asset server exists, and a host may add sites before that.
    built: bool,
}

/// The marker that a [`FlameSite`] already has its flame: `Without<Flamed>` on the site is why a
/// second frame does not twin it.
#[derive(Component)]
struct Flamed;

/// Builds the flame mesh and material once, at `Startup`: the quad the vertex stage billboards, and
/// the material every flame shares.
fn build_flame_assets(
    mut assets: ResMut<FlameAssets>,
    look: Res<FlameLook>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<FlameMaterial>>,
) {
    // A billboard quad: the vertex stage in `flame.wesl` reads its local x and y, faces it at the
    // camera and scales it by the flame's height, so the model is a unit square in the local XY
    // plane about its origin.
    assets.mesh = meshes.add(Rectangle::new(1.0, 1.0));
    assets.material = materials.add(material_for(*look));
    assets.max_height = look.max_height;
    assets.built = true;
}

/// The material for a [`FlameLook`]: its colour (green clamped, so a palette's oversaturated fire does
/// not blow the ramp out) and its strength as the ember's weight.
fn material_for(look: FlameLook) -> FlameMaterial {
    FlameMaterial {
        base: LinearRgba {
            red: look.color.red,
            green: look.color.green.min(1.0),
            blue: look.color.blue,
            alpha: look.strength,
        },
        params: Vec4::new(0.0, look.max_height, 0.0, look.strength),
    }
}

/// Gives every [`FlameSite`] without one a single [`Flame`] child.
///
/// The child carries the site's height, phase and starting brightness on its [`MeshTag`] (see
/// [`pack_flame_tag`](crate::pack_flame_tag)), stands at the site's own origin, and is neither a
/// shadow caster nor frustum culled — the vertex stage scales the unit quad, so the mesh's own
/// `Aabb` does not cover what is drawn. The starting brightness is the fire's swing at the frame it
/// spawns, so the very first frame a flame is drawn it already matches its light.
///
/// Spawned through `queue_silenced`: a site despawned in the same frame it was created (a short
/// fire's lifetime running out, a carried torch going away) must not panic the insert.
fn dress_flames(
    mut commands: Commands,
    sites: Query<(Entity, &FlameSite), Without<Flamed>>,
    assets: Res<FlameAssets>,
    time: Res<Time>,
) {
    if !assets.built {
        return;
    }
    let max_height = assets.max_height;
    let t = time.elapsed_secs();
    for (site, flame) in &sites {
        let catch = flame.kindle;
        let brightness = flicker_swing(flame.diameter, flame.depth, t, flame.phase);
        let tag = MeshTag::new(pack_flame_tag(flame.phase, flame.height, brightness, max_height));
        let mesh = assets.mesh.clone();
        let material = assets.material.clone();
        commands.entity(site).queue_silenced(move |mut site: EntityWorldMut| {
            site.insert(Flamed).with_children(|parent| {
                parent.spawn((
                    Name::new("Flame"),
                    Flame { kindled: if catch > 0.0 { 0.0 } else { f32::MAX } },
                    tag,
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::IDENTITY,
                    NotShadowCaster,
                    NoFrustumCulling,
                ));
            });
        });
    }
}

/// Ticks the flames' shared clock, writes each flame's own brightness, and applies a changed
/// [`FlameLook`].
///
/// Two clocks, deliberately. The **shape** (the noise climbing the silhouette) runs on real time
/// (`Time<Real>`), so a paused world's fires still move; the **brightness** runs on virtual time and
/// through the same [`flicker_swing`] the light uses, so the flame and the ground it lights breathe
/// together and both hold still when the world is paused. That split is the crate's existing one —
/// see `docs/research/fire-light.md` and `docs/research/flame-flicker.md`.
///
/// The shared clock is one write a frame whatever the number of flames; a flame's brightness is a tag
/// write only on the frames its quantised byte changes, and skipped entirely for a flame whose site has
/// no `FlameSite` parent. `MeshTag` is in `bevy_pbr`'s `Changed<MeshTag>` re-extraction set, so a write
/// past the change is a render-world re-extract for that flame.
fn update_flames(
    real: Res<Time<Real>>,
    time: Res<Time>,
    look: Res<FlameLook>,
    assets: Res<FlameAssets>,
    mut materials: ResMut<Assets<FlameMaterial>>,
    mut flames: Query<(&mut Flame, &mut Transform, &mut MeshTag, &ChildOf)>,
    sites: Query<&FlameSite>,
) {
    if !assets.built {
        return;
    }
    // The catch: a flame lit with its fire grows from nothing over the site's `kindle`, on the same
    // `kindle_gain` curve the light uses, so the flame and the light it motivates catch together. A site
    // with no kindle is left alone — a fire burning all along costs nothing here.
    let dt = real.delta_secs();
    let t = time.elapsed_secs();
    for (mut flame, mut transform, mut tag, parent) in &mut flames {
        let site = sites.get(parent.parent()).ok();
        if let Some(site) = site {
            // Only when the brightness byte actually changes: `MeshTag` is in `bevy_pbr`'s
            // `Changed<MeshTag>` re-extraction set, so a write here costs a render-world re-extract for
            // that flame. The byte holds 1/255 of the swing, so at 60 fps a 2.6 Hz fire skips some
            // frames; a fire that never changes costs nothing at all.
            let brightness = flicker_swing(site.diameter, site.depth, t, site.phase);
            let packed = set_flame_brightness(tag.value, brightness);
            if packed != tag.value {
                tag.value = packed;
            }
        }
        if flame.kindled >= f32::MAX {
            continue;
        }
        flame.kindled = (flame.kindled + dt).min(f32::MAX);
        let catch = site.map_or(0.0, |site| site.kindle);
        let gain = if catch > 0.0 { kindle_gain(flame.kindled.min(catch)) } else { 1.0 };
        // Width and height both: a fire that is catching is smaller in both, and a flame that grew tall
        // while staying wide would read as a sheet. The brightness is in the tag and is written above.
        transform.scale = Vec3::splat(gain.max(0.001));
        if flame.kindled >= catch {
            flame.kindled = f32::MAX;
            transform.scale = Vec3::ONE;
        }
    }
    let elapsed = real.elapsed_secs_wrapped();
    if let Some(mut made) = materials.get_mut(&assets.material) {
        made.params.x = elapsed;
        if look.is_changed() {
            // Rewritten in place: a `FlameLook` change is a colour or strength change, and rebuilding
            // the material would re-prepare every flame's pipeline for nothing.
            made.base = LinearRgba {
                red: look.color.red,
                green: look.color.green.min(1.0),
                blue: look.color.blue,
                alpha: look.strength,
            };
            made.params.y = look.max_height;
            made.params.w = look.strength;
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::model::{FLICKER_DEPTH, unpack_flame_tag};
    use core::f32::consts::TAU;

    /// The headless half of the plugin: assets, a look, and the two systems, with no render app (a
    /// `MaterialPlugin` needs one, and there is no GPU in a test).
    fn dressing_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Assets::<Mesh>::default())
            .insert_resource(Assets::<FlameMaterial>::default())
            .init_resource::<FlameLook>()
            .init_resource::<FlameAssets>()
            // A fixed step, so a test that advances the clock gets the virtual time it asked for: the
            // time plugin rewrites `Time` from the real clock every frame otherwise.
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs_f64(1.0 / 60.0),
            ));
        app.add_systems(Startup, build_flame_assets)
            .add_systems(Update, (dress_flames, update_flames).chain());
        app.update();
        app
    }

    fn flame_children(app: &mut App) -> Vec<(Entity, u32)> {
        app.world_mut()
            .query_filtered::<(&MeshTag, &ChildOf), With<Flame>>()
            .iter(app.world())
            .map(|(tag, child_of)| (child_of.parent(), tag.value))
            .collect()
    }

    #[test]
    fn every_site_gets_one_flame_and_a_second_frame_does_not_twin_it() {
        let mut app = dressing_app();
        let site = app
            .world_mut()
            .spawn((
                FlameSite { height: 0.9, phase: 0.4, kindle: 0.0, diameter: None, depth: FLICKER_DEPTH },
                Transform::default(),
            ))
            .id();
        app.update();
        let children = flame_children(&mut app);
        assert_eq!(children.len(), 1, "one flame");
        assert_eq!(children[0].0, site, "a child of the site");
        app.update();
        assert_eq!(flame_children(&mut app).len(), 1, "still one flame");
    }

    /// A flame whose quantised brightness does not change must not write its tag: `MeshTag` is in
    /// `bevy_pbr`'s `Changed<MeshTag>` re-extraction set, so an unchanged write costs a render-world
    /// re-extract for that flame every frame. Detected through the change tick, which is what that set
    /// keys on — comparing `value` would pass an always-write loop that rewrote the same bytes.
    #[test]
    fn an_unchanged_brightness_leaves_the_tag_unwritten() {
        let mut app = dressing_app();
        app.world_mut().spawn((
            FlameSite { height: 0.35, phase: 2.5, kindle: 0.0, diameter: Some(0.34), depth: FLICKER_DEPTH },
            Transform::default(),
        ));
        app.update();
        let flame =
            app.world_mut().query_filtered::<Entity, With<Flame>>().single(app.world()).expect("one flame");
        // 30 frames is half a second: 1.3 cycles of a 2.6 Hz fire, over a swing of 256 byte steps. The
        // byte moves on most frames but not all, so an always-write loop's 30 ticks are distinguishable
        // from a write-on-change loop's fewer.
        let mut changed = 0;
        let mut last = app.world().entity(flame).get_ref::<MeshTag>().expect("a tag").last_changed();
        for _ in 0..30 {
            app.update();
            // Read-only: a `get_mut` would itself bump the changed tick, which is the thing under test.
            let tag = app.world().entity(flame).get_ref::<MeshTag>().expect("a tag");
            let tick = tag.last_changed();
            if tick != last {
                changed += 1;
                last = tick;
            }
        }
        assert!(changed > 0, "the tag never changed, so the flame is not breathing at all");
        assert!(changed < 30, "the tag changed on all 30 frames: an unchanged byte still re-extracts");
    }

    #[test]
    fn a_flames_tag_carries_its_sites_phase_and_height() {
        let mut app = dressing_app();
        app.world_mut().spawn((
            FlameSite { height: 0.35, phase: 2.5, kindle: 0.0, diameter: None, depth: FLICKER_DEPTH },
            Transform::default(),
        ));
        app.update();
        let max_height = app.world().resource::<FlameAssets>().max_height;
        let children = flame_children(&mut app);
        assert_eq!(children.len(), 1);
        let (phase, height, _brightness) = unpack_flame_tag(children[0].1, max_height);
        assert!((phase - 2.5).abs() < TAU / 256.0, "phase {phase}");
        assert!((height - 0.35).abs() < 1e-3, "height {height}");
    }

    /// The flame's brightness is the fire's own swing: the same function, the same diameter and the same
    /// phase the light is written from. A flame that held still while its pool moved is the defect
    /// `docs/research/flame-flicker.md` measures, so this reads the tag the shader will.
    #[test]
    fn a_flames_brightness_is_the_fires_own_swing() {
        let mut app = dressing_app();
        app.world_mut().spawn((
            FlameSite { height: 0.35, phase: 2.5, kindle: 0.0, diameter: Some(0.34), depth: FLICKER_DEPTH },
            Transform::default(),
        ));
        app.update();
        let max_height = app.world().resource::<FlameAssets>().max_height;
        let t = app.world().resource::<Time>().elapsed_secs();
        let children = flame_children(&mut app);
        assert_eq!(children.len(), 1);
        let (_, _, brightness) = unpack_flame_tag(children[0].1, max_height);
        let expected = flicker_swing(Some(0.34), FLICKER_DEPTH, t, 2.5);
        assert!(
            (brightness - expected).abs() < 1.0 / 255.0,
            "brightness {brightness} against the light's own {expected}"
        );
        assert!((0.70..=1.0).contains(&brightness), "{brightness} outside the swing window");
    }

    /// A sized fire and an un-sized one must not read the same, or the size a host gives the light would
    /// not reach the flame. The two forms part company only once the clock has moved: at t = 0 every
    /// fire's swing is 0.85, whatever its size.
    #[test]
    fn a_sized_flame_and_an_unsized_one_do_not_read_alike() {
        fn brightness_of(diameter: Option<f32>) -> f32 {
            let mut app = dressing_app();
            app.world_mut().spawn((
                FlameSite { height: 0.35, phase: 2.5, kindle: 0.0, diameter, depth: FLICKER_DEPTH },
                Transform::default(),
            ));
            // Eighteen frames of the fixed step: a 2.6 Hz fire and a 1.2 Hz one have parted company by
            // then, and at t = 0 every fire's swing is 0.85 whatever its size.
            for _ in 0..18 {
                app.update();
            }
            let max_height = app.world().resource::<FlameAssets>().max_height;
            unpack_flame_tag(flame_children(&mut app)[0].1, max_height).2
        }
        assert_ne!(brightness_of(Some(0.34)), brightness_of(None));
    }

    #[test]
    fn a_changed_look_rewrites_the_shared_material() {
        let mut app = dressing_app();
        let handle = app.world().resource::<FlameAssets>().material.clone();
        {
            let mut look = app.world_mut().resource_mut::<FlameLook>();
            look.color = LinearRgba::rgb(0.2, 0.9, 0.4);
            look.strength = 0.5;
        }
        app.update();
        let material = app
            .world()
            .resource::<Assets<FlameMaterial>>()
            .get(&handle)
            .cloned()
            .unwrap_or_else(|| panic!("the shared flame material must exist"));
        assert!((material.base.red - 0.2).abs() < 1e-6, "{:?}", material.base);
        assert!((material.base.green - 0.9).abs() < 1e-6, "{:?}", material.base);
        assert!((material.base.alpha - 0.5).abs() < 1e-6, "{:?}", material.base);
        assert!((material.params.w - 0.5).abs() < 1e-6, "{:?}", material.params);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod kindle_tests {
    use super::*;
    use crate::KINDLE_SECONDS;
    use bevy::time::TimeUpdateStrategy;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Assets::<Mesh>::default())
            .insert_resource(Assets::<FlameMaterial>::default())
            .init_resource::<FlameLook>()
            .init_resource::<FlameAssets>()
            .insert_resource(TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(
                1.0 / 60.0,
            )));
        app.add_systems(Startup, build_flame_assets)
            .add_systems(Update, (dress_flames, update_flames).chain());
        app.update();
        app
    }

    fn scale_of(app: &mut App) -> f32 {
        app.world_mut()
            .query_filtered::<&Transform, With<Flame>>()
            .iter(app.world())
            .next()
            .map(|transform| transform.scale.x)
            .unwrap_or(f32::NAN)
    }

    #[test]
    fn a_flame_lit_with_its_fire_grows_from_nothing_and_stops_at_full() {
        // A site that asks for a catch grows over that many seconds, on the same envelope the light
        // uses — so the flame and the light it motivates arrive together instead of the flame popping in
        // whole a frame before the light has begun.
        //
        // **This pins the CPU-side number only.** The drawn size is computed in the shader, by
        // multiplying this scale into `size` (`src/flame.wesl`), and a capture — not this test — is what
        // shows it: for four iterations the scale was written correctly and the shader ignored it, and
        // this test passed the whole time. `docs/research/flame-kindle.md` has the measurement and
        // `/home/ladvien/l18/kindle2_after/` the frames.
        let mut app = app();
        let site = FlameSite {
            height: 0.9,
            phase: 0.0,
            kindle: KINDLE_SECONDS,
            diameter: None,
            depth: FLICKER_DEPTH,
        };
        app.world_mut().spawn((site, Transform::default()));
        app.update();
        // The first frame after the flame is dressed: it is barely there.
        app.update();
        let early = scale_of(&mut app);
        assert!(early < 0.2, "a flame that has just caught is tiny: {early}");

        // Halfway through the catch it is most of the way up, and by the end it is exactly full.
        for _ in 0..((KINDLE_SECONDS / 2.0 * 60.0) as usize) {
            app.update();
        }
        let middle = scale_of(&mut app);
        assert!(middle > early && middle < 1.05, "the flame is still growing at half time: {middle}");
        for _ in 0..((KINDLE_SECONDS * 60.0) as usize) {
            app.update();
        }
        let settled = scale_of(&mut app);
        assert!((settled - 1.0).abs() < 1e-5, "a caught flame is full size and stays there: {settled}");
    }

    #[test]
    fn a_flame_that_has_been_burning_all_along_is_full_size_at_once() {
        // The common case: a site with no kindle is untouched, so nothing changes for a fire that was
        // already alight.
        let mut app = app();
        app.world_mut().spawn((
            FlameSite { height: 0.9, phase: 0.0, kindle: 0.0, diameter: None, depth: FLICKER_DEPTH },
            Transform::default(),
        ));
        app.update();
        app.update();
        assert_eq!(scale_of(&mut app), 1.0);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod catch_colour_tests {
    use super::*;
    use crate::KINDLE_SECONDS;
    use bevy::time::TimeUpdateStrategy;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Assets::<Mesh>::default())
            .insert_resource(Assets::<FlameMaterial>::default())
            .init_resource::<FlameLook>()
            .init_resource::<FlameAssets>()
            .insert_resource(TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(
                1.0 / 60.0,
            )));
        app.add_systems(Startup, build_flame_assets)
            .add_systems(Update, (dress_flames, update_flames).chain());
        app.update();
        app
    }

    /// Each flame's instance scale: the number the shader reads as its own catch, because one shared
    /// material cannot hold a per-flame value and the transform can.
    fn scales(app: &mut App) -> Vec<f32> {
        let mut found: Vec<f32> = app
            .world_mut()
            .query_filtered::<&Transform, With<Flame>>()
            .iter(app.world())
            .map(|transform| transform.scale.x)
            .collect();
        found.sort_by(|a, b| a.total_cmp(b));
        found
    }

    #[test]
    fn a_catching_flame_carries_its_own_progress_for_the_colour() {
        // The flame's scale IS the colour's input: 0..1 while it catches, exactly 1 once it has taken.
        let mut app = app();
        app.world_mut().spawn((
            FlameSite {
                height: 0.9,
                phase: 0.0,
                kindle: KINDLE_SECONDS,
                diameter: None,
                depth: FLICKER_DEPTH,
            },
            Transform::default(),
        ));
        app.update();
        app.update();
        let early = scales(&mut app)[0];
        assert!(early < 0.2, "a catching flame reports a small catch: {early}");

        for _ in 0..((KINDLE_SECONDS * 60.0) as usize + 2) {
            app.update();
        }
        let settled = scales(&mut app)[0];
        assert!((settled - 1.0).abs() < 1e-5, "and exactly 1 once it has taken: {settled}");
    }

    #[test]
    fn one_fire_catching_does_not_tint_another() {
        // The limit this iteration removed: with the catch in the shared uniform, the youngest fire in the frame
        // tinted every flame in the world. Now a steady flame is skipped outright — `Flame::kindled` is `f32::MAX`
        // and `update_flames` continues before it can touch the scale — so there is no shared number left for one
        // fire's catch to reach another through. Note what this test can and cannot catch: the isolation holds
        // *two* ways — the skip, and a site with `kindle: 0` giving a gain of exactly 1 — so it passes under
        // either, and removing one of them does not fail it. It is here to pin the property, not to detect a
        // regression in the loop's shape.
        let mut app = app();
        app.world_mut().spawn((
            FlameSite {
                height: 0.9,
                phase: 0.0,
                kindle: KINDLE_SECONDS,
                diameter: None,
                depth: FLICKER_DEPTH,
            },
            Transform::default(),
        ));
        app.world_mut().spawn((
            FlameSite { height: 0.9, phase: 1.3, kindle: 0.0, diameter: None, depth: FLICKER_DEPTH },
            Transform::from_xyz(5.0, 0.0, 0.0),
        ));
        app.update();
        app.update();
        let catching = scales(&mut app);
        assert_eq!(catching.len(), 2, "two flames: {catching:?}");
        assert!(catching[0] < 0.2, "the catching one is small: {catching:?}");
        assert!((catching[1] - 1.0).abs() < 1e-5, "the steady one is untouched: {catching:?}");
    }
}
