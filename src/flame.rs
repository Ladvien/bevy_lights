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
use crate::model::{kindle_gain, pack_flame_tag};

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
    /// `z` unused, `w` the ember strength.
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
    /// motivates catch together. `0.0` (the default) is a fire that has been burning all along and needs
    /// no catch; a host lighting a fire sets it, usually to
    /// [`KINDLE_SECONDS`](crate::KINDLE_SECONDS).
    pub kindle: f32,
}

/// The drawn flame: the child mesh [`FlamePlugin`] spawns under each [`FlameSite`].
///
/// Its `Transform` is the site's height scale times the fire's catch (a `FlameSite` with a
/// [`kindle`](FlameSite::kindle) grows from nothing over that many seconds); the shader's own tip and
/// noise animation is untouched by it.
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
/// The child carries the site's height and phase on its [`MeshTag`] (see
/// [`pack_flame_tag`](crate::pack_flame_tag)), stands at the site's own origin, and is neither a
/// shadow caster nor frustum culled — the vertex stage scales the unit quad, so the mesh's own
/// `Aabb` does not cover what is drawn.
///
/// Spawned through `queue_silenced`: a site despawned in the same frame it was created (a short
/// fire's lifetime running out, a carried torch going away) must not panic the insert.
fn dress_flames(
    mut commands: Commands,
    sites: Query<(Entity, &FlameSite), Without<Flamed>>,
    assets: Res<FlameAssets>,
) {
    if !assets.built {
        return;
    }
    let max_height = assets.max_height;
    for (site, flame) in &sites {
        let catch = flame.kindle;
        let tag = MeshTag::new(pack_flame_tag(flame.phase, flame.height, max_height));
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

/// Ticks the flames' shared clock and applies a changed [`FlameLook`].
///
/// Real time (`Time<Real>`), so a paused world's fires still flicker; wrapped hourly, so the noise's
/// input stays small in a long session. One write a frame, whatever the number of flames.
fn update_flames(
    time: Res<Time<Real>>,
    look: Res<FlameLook>,
    assets: Res<FlameAssets>,
    mut materials: ResMut<Assets<FlameMaterial>>,
    mut flames: Query<(&mut Flame, &mut Transform, &ChildOf)>,
    sites: Query<&FlameSite>,
) {
    if !assets.built {
        return;
    }
    // The catch: a flame lit with its fire grows from nothing over the site's `kindle`, on the same
    // `kindle_gain` curve the light uses, so the flame and the light it motivates catch together. A site
    // with no kindle is left alone — a fire burning all along costs nothing here.
    let dt = time.delta_secs();
    for (mut flame, mut transform, parent) in &mut flames {
        if flame.kindled >= f32::MAX {
            continue;
        }
        flame.kindled = (flame.kindled + dt).min(f32::MAX);
        let catch = sites.get(parent.parent()).map_or(0.0, |site| site.kindle);
        let gain = if catch > 0.0 { kindle_gain(flame.kindled.min(catch)) } else { 1.0 };
        // Width and height both: a fire that is catching is smaller in both, and a flame that grew tall
        // while staying wide would read as a sheet. The shader's own animation is in the tag and is
        // untouched.
        transform.scale = Vec3::splat(gain.max(0.001));
        if flame.kindled >= catch {
            flame.kindled = f32::MAX;
            transform.scale = Vec3::ONE;
        }
    }
    let elapsed = time.elapsed_secs_wrapped();
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
    use crate::model::unpack_flame_tag;

    /// The headless half of the plugin: assets, a look, and the two systems, with no render app (a
    /// `MaterialPlugin` needs one, and there is no GPU in a test).
    fn dressing_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Assets::<Mesh>::default())
            .insert_resource(Assets::<FlameMaterial>::default())
            .init_resource::<FlameLook>()
            .init_resource::<FlameAssets>();
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
            .spawn((FlameSite { height: 0.9, phase: 0.4, kindle: 0.0 }, Transform::default()))
            .id();
        app.update();
        let children = flame_children(&mut app);
        assert_eq!(children.len(), 1, "one flame");
        assert_eq!(children[0].0, site, "a child of the site");
        app.update();
        assert_eq!(flame_children(&mut app).len(), 1, "still one flame");
    }

    #[test]
    fn a_flames_tag_carries_its_sites_phase_and_height() {
        let mut app = dressing_app();
        app.world_mut().spawn((FlameSite { height: 0.35, phase: 2.5, kindle: 0.0 }, Transform::default()));
        app.update();
        let max_height = app.world().resource::<FlameAssets>().max_height;
        let children = flame_children(&mut app);
        assert_eq!(children.len(), 1);
        let (phase, height) = unpack_flame_tag(children[0].1, max_height);
        assert!((phase - 2.5).abs() < 1e-3, "phase {phase}");
        assert!((height - 0.35).abs() < 1e-3, "height {height}");
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
        let mut app = app();
        let site = FlameSite { height: 0.9, phase: 0.0, kindle: KINDLE_SECONDS };
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
        app.world_mut().spawn((FlameSite { height: 0.9, phase: 0.0, kindle: 0.0 }, Transform::default()));
        app.update();
        app.update();
        assert_eq!(scale_of(&mut app), 1.0);
    }
}
