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
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::NoAutoAabb;
use bevy::light::NotShadowCaster;
use bevy::mesh::{MeshTag, MeshVertexBufferLayoutRef};
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin, MeshMaterial3d};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError};
use bevy::shader::ShaderRef;
use bevy::time::Real;

use crate::LightsSystems;
use crate::chain_sets;
use crate::flicker::{Flicker, Kindle, Snuff};
use crate::model::{
    DEFAULT_FIRE_DIAMETER, FLICKER_DEPTH, KINDLE_SECONDS, flame_cull_extents, flame_tilt, flicker_swing,
    kindle_gain, pack_flame_tag, set_flame_brightness, snuff_gain, unpack_flame_tag,
};
use crate::pool::PoolCandidate;

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
        crate::embers::build(app);
        app.add_plugins(MaterialPlugin::<FlameMaterial>::default())
            .init_resource::<FlameLook>()
            .init_resource::<FlameAssets>()
            .register_type::<FlameLook>()
            .register_type::<FlameWind>()
            .register_type::<FlameGust>()
            .register_type::<FlameSite>()
            .register_type::<Flame>()
            .add_systems(
                Update,
                (
                    gust_wind,
                    dress_flames,
                    update_flames,
                    crate::embers::dress_embers,
                    crate::embers::update_embers,
                )
                    .chain()
                    .in_set(LightsSystems::Flame),
            )
            .add_systems(Startup, build_flame_assets);
        crate::report::watch::<FlameSite>(app);
        crate::report::watch::<crate::FlameEmbers>(app);
        crate::report::watch_resource::<FlameLook>(app);
        crate::report::watch_resource::<FlameWind>(app);
        crate::report::watch_resource::<FlameGust>(app);
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
    /// `z` 1 on the material of the sites that asked for [`FlameSite::depth_fade`] and 0 on the rest (the
    /// per-flame catch is read from each flame's own instance scale, so one fire's catch cannot tint
    /// another's), `w` the ember strength.
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
///
/// The site is **not** tied to a light unless [`FlameSite::fire`] says so. A fire whose light has to sit somewhere other than the fire
/// (a sconce's pool held out over the wall face, a window glow) is a light in one place and a site
/// parented to the bracket in another; set `fire` to the light, or give both the same `phase`, `diameter` and `depth`, and they
/// breathe together. A light with no fire on it simply has no site.
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
    /// Fade the flame out when its fire's source cannot be seen: the flame's upper half stops drawing over the
    /// wall a torch is behind, so the player does not meet "a lone flame with no torch" (the question of
    /// Sanctuary's playtest reports 1792667477 and 1792667613).
    ///
    /// The shader reads the depth prepass at the flame's source, a third of the way up it, and over a ring of
    /// taps around it, and multiplies the flame's emission by the share of them the camera can see, so a wall
    /// edge crossing the source takes the flame down over a few pixels rather than popping it
    /// (`docs/research/flame-occlusion.md`). A tap hides the source when the scene there is nearer than the source by more than
    /// 15 % of the flame's height, compared in linear view distance, so **a perspective and an orthographic camera both work, at
    /// any far plane** (issue #6). **It needs a [`DepthPrepass`](bevy::core_pipeline::prepass::DepthPrepass) on the
    /// camera**: without one there is no depth to read and the flame draws whole. `false` (the default a host that
    /// does not care writes) is the flame exactly as it always was, with no depth read at all. May be changed at
    /// any time; [`FlamePlugin`] moves the flame to the right material.
    pub depth_fade: bool,
    /// The light this flame is the fire of, when the host has one: the entity carrying the [`Flicker`] (or, for a prop the
    /// pool lights, the [`PoolCandidate`]) whose breath this flame must share. The flame's **brightness** then reads
    /// that entity's `diameter`, `depth` and `phase` every frame instead of this site's own, so a flame and its light
    /// cannot disagree about how deep the fire dips and a `Flicker::set_depth` moves both together. A prop's
    /// candidate with no depth is [`FLICKER_DEPTH`](crate::FLICKER_DEPTH) and one with no phase keeps this site's, the
    /// pool's own rules.
    ///
    /// The site and its light need not be related in the hierarchy (a torch's flame and its light are often
    /// siblings), which is why this is a link and not a read of the parent. `None` (the default a host that does not
    /// care writes), an entity that was despawned, and an entity with neither component all leave the site's own
    /// `diameter`, `depth` and `phase` in charge, exactly as before. The link is only read: the host owns both
    /// lifetimes. The shape's noise phase does not follow it (`docs/research/flame-fire-link.md`).
    ///
    /// The flame's **catch** follows the named entity's [`Kindle`] too: while it runs, the flame's drawn size is
    /// [`kindle_gain`](crate::kindle_gain) of its `elapsed`, so a fire re-lit by inserting `Kindle::default()` shrinks to
    /// a point and grows with its light instead of burning on at full height over a dark floor, and it is whole once the
    /// kindle settles (`docs/research/flame-relight.md`). The site's own [`kindle`](FlameSite::kindle) does not apply
    /// while a linked kindle runs.
    pub fire: Option<Entity>,
}

impl FlameSite {
    /// What is unusable in these settings, field by field; empty when they are fine. See
    /// [`Flicker::problems`](crate::Flicker::problems). `fire` is not checked: whether the entity still
    /// exists is the host's.
    pub fn problems(&self) -> Vec<crate::Problem> {
        use crate::model::problem::{finite, non_negative, positive, within};
        let mut out = Vec::new();
        positive(&mut out, "height", self.height);
        finite(&mut out, "phase", self.phase);
        non_negative(&mut out, "kindle", self.kindle);
        if let Some(d) = self.diameter {
            positive(&mut out, "diameter", d);
        }
        within(&mut out, "depth", self.depth, 0.0, crate::MAX_FLICKER_DEPTH);
        out
    }
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
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[reflect(Component)]
pub struct Flame {
    /// Seconds since this flame's fire was lit, when the site asked for a catch. Counted on
    /// `Time<Real>`, like the flame's own clock, so a paused world's flames still move — the catch is
    /// part of the drawn fire, not of the light's.
    pub kindled: f32,
    /// The entity this flame is the fire of: the parent of its [`FlameSite`] — the torch, the sconce, the
    /// light — so "which torch is that flame" is one component read. `None` when the site is a root entity
    /// or its parent is itself a `FlameSite` (a nested site belongs to the outer one, not to a prop).
    ///
    /// Kept current every frame by [`FlamePlugin`], so a site the host reparents or unparents is followed
    /// rather than left pointing at its old torch. A despawned parent takes the site and the flame with it
    /// (Bevy despawns descendants), so the value never resolves to whatever reuses the entity's slot.
    pub source: Option<Entity>,
}

/// How the scene's flames look: the fire colour every flame is tinted from, how strongly it adds,
/// and the height scale a [`FlameSite`]'s height is measured against.
///
/// [`FlamePlugin`] `init_resource`s this, so a host that inserts its own first overrides it. The
/// defaults are a campfire: a warm orange at full strength, four world units of headroom for the
/// tallest flame a scene has.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
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

impl FlameLook {
    /// What is unusable in these settings, field by field; empty when they are fine. See
    /// [`Flicker::problems`](crate::Flicker::problems).
    pub fn problems(&self) -> Vec<crate::Problem> {
        use crate::model::problem::{non_negative, positive};
        let mut out = Vec::new();
        non_negative(&mut out, "color.red", self.color.red);
        non_negative(&mut out, "color.green", self.color.green);
        non_negative(&mut out, "color.blue", self.color.blue);
        non_negative(&mut out, "strength", self.strength);
        positive(&mut out, "max_height", self.max_height);
        out
    }
}

/// The wind the flames lean in: a velocity on the ground plane, in metres per second.
///
/// The host copies its weather's wind into this one resource (the wind belongs to `bevy_elements`
/// or the game; this crate reads it and owns nothing of it). Every [`FlameSite`]'s flame then leans
/// *downwind* by [`flame_tilt`] of the wind's speed and that fire's own `diameter`
/// ([`DEFAULT_FIRE_DIAMETER`] when it has none), so a brazier bends less than a torch in the same
/// gust, and every flame in the scene bends the same way. The flame keeps its length and its foot:
/// only its tip moves, and it is drawn that much shorter on screen.
///
/// The default is no wind, which draws every flame exactly as it was drawn before this resource
/// existed. The crate does not add it: insert it (or `init_resource` it) when there is a wind, and
/// write `velocity` each frame the wind changes. The tilt law is SI, so a host whose world is not in
/// metres converts first. `x` is the world's X and `y` the world's Z (`Vec3::new(v.x, 0.0, v.y)` is
/// the direction the air moves).
///
/// The lean is drawn in the flame's own quad, and the pool's light for the fire moves with it: a pooled
/// light on the [`FlameSite`]'s entity (or one the site's `fire` names) is carried to where the leaned flame's
/// middle is ([`lean_light_shift`](crate::lean_light_shift), `docs/research/flame-lean-light.md`).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
#[reflect(Resource)]
pub struct FlameWind {
    /// The direction the air moves and how fast, in metres per second, on the world's XZ plane.
    pub velocity: Vec2,
}

impl FlameWind {
    /// What is unusable in these settings, field by field; empty when they are fine. See
    /// [`Flicker::problems`](crate::Flicker::problems).
    pub fn problems(&self) -> Vec<crate::Problem> {
        use crate::model::problem::finite;
        let mut out = Vec::new();
        finite(&mut out, "velocity.x", self.velocity.x);
        finite(&mut out, "velocity.y", self.velocity.y);
        out
    }
}

/// A wind with gusts: add it and [`FlamePlugin`] writes [`FlameWind::velocity`] every frame, so the
/// flames lean and ease with the air instead of standing at one angle.
///
/// The wind is `mean` (metres per second on the world's XZ plane, as [`FlameWind::velocity`]) with a
/// stationary Gaussian fluctuation of relative size `intensity` (the turbulence intensity,
/// `σ / |mean|`) and correlation time `timescale` seconds; the heading wanders by about `intensity`
/// radians too. It is [`gust_velocity`](crate::model::gust_velocity) of `Time<Real>`, like the
/// flames' own clock, so it is the same on every machine and a paused world's fires still sway. The
/// `seed` picks the night: two seeds give two different, equally plausible winds.
///
/// The host still owns the *mean* (weather is `bevy_elements`'): write `mean` when the weather
/// changes. Without a `FlameGust` the plugin never touches [`FlameWind`], so a host that drives its
/// own wind each frame is unaffected. A zero `mean` is still air: a gust is relative to a mean.
/// Defaults are design defaults (`docs/research/flame-gust.md`).
#[derive(Resource, Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
#[reflect(Resource)]
pub struct FlameGust {
    /// The mean wind, in metres per second on the world's XZ plane.
    pub mean: Vec2,
    /// Turbulence intensity, `σ / |mean|`: 0 is a steady wind, 0.12 a faint sway, 0.25 a lively
    /// courtyard.
    pub intensity: f32,
    /// How long a gust stays correlated, in seconds (clamped at 0.05 s from below).
    pub timescale: f32,
    /// Which wind: the same seed is the same wind.
    pub seed: u32,
}

impl Default for FlameGust {
    fn default() -> Self {
        Self {
            mean: Vec2::ZERO,
            intensity: crate::model::GUST_INTENSITY,
            timescale: crate::model::GUST_TIMESCALE,
            seed: 0,
        }
    }
}

impl FlameGust {
    /// What is unusable in these settings, field by field; empty when they are fine. See
    /// [`Flicker::problems`](crate::Flicker::problems).
    pub fn problems(&self) -> Vec<crate::Problem> {
        use crate::model::problem::{finite, non_negative};
        let mut out = Vec::new();
        finite(&mut out, "mean.x", self.mean.x);
        finite(&mut out, "mean.y", self.mean.y);
        non_negative(&mut out, "intensity", self.intensity);
        non_negative(&mut out, "timescale", self.timescale);
        out
    }
}

/// Writes the gusting wind into [`FlameWind`], inserting it if the host has not.
fn gust_wind(
    real: Res<Time<Real>>,
    gust: Option<Res<FlameGust>>,
    wind: Option<ResMut<FlameWind>>,
    mut commands: Commands,
) {
    let Some(gust) = gust else { return };
    let (x, z) = crate::model::gust_velocity(
        (gust.mean.x, gust.mean.y),
        gust.intensity,
        gust.timescale,
        gust.seed,
        real.elapsed_secs_f64(),
    );
    let velocity = Vec2::new(x, z);
    match wind {
        Some(mut wind) => {
            if wind.velocity != velocity {
                wind.velocity = velocity;
            }
        }
        None => commands.insert_resource(FlameWind { velocity }),
    }
}

/// Writes a flame's instance scale: `x` is its catch (the size the vertex stage multiplies in), and
/// `y` and `z` are `1 +` the downwind lean (`sin θ` along world X and Z). Lengths, not directions, so a
/// parent's rotation cannot change them; `1 +` keeps them positive. Written only when it changes, so a
/// flame in still air costs nothing.
///
/// The flame's culling box goes with it. The scale is the shader's data, not a size, so it distorts the
/// box the frustum test builds from the transform; [`flame_cull_extents`] divides the distortion out, and
/// so the box is rewritten on exactly the frames the scale is, and not otherwise.
fn set_scale(transform: &mut Transform, bound: &mut Aabb, height: f32, took: f32, lean: Vec2) {
    let scale = Vec3::new(took, 1.0 + lean.x, 1.0 + lean.y);
    if transform.scale != scale {
        transform.scale = scale;
        bound.half_extents = Vec3A::from(flame_cull_extents(height, took, lean.to_array()));
    }
}

/// The one mesh, the two materials, and the height scale every flame in the scene shares.
#[derive(Resource, Default)]
pub(crate) struct FlameAssets {
    mesh: Handle<Mesh>,
    material: Handle<FlameMaterial>,
    /// The same material with the depth fade on, for the sites that set [`FlameSite::depth_fade`]. Two
    /// shared materials rather than one per flame, so a scene with the fade off draws exactly as it did.
    faded_material: Handle<FlameMaterial>,
    /// The height scale the material was built with ([`FlameLook::max_height`] at `Startup`): a
    /// [`FlameSite`]'s height is packed as a fraction of it, so the CPU packer and the shader must
    /// agree on it.
    max_height: f32,
    /// Whether the handles have been made. The mesh and the material are built at `Startup`, after
    /// the asset server exists, and a host may add sites before that.
    built: bool,
}

impl FlameAssets {
    /// Whether the handles have been made.
    pub(crate) fn built(&self) -> bool {
        self.built
    }

    /// The height scale a flame's tag is packed against.
    pub(crate) fn max_height(&self) -> f32 {
        self.max_height
    }

    /// The shared material a site's flame is drawn with.
    fn material_for(&self, depth_fade: bool) -> &Handle<FlameMaterial> {
        if depth_fade { &self.faded_material } else { &self.material }
    }
}

/// The marker that a [`FlameSite`] already has its flame: `Without<Flamed>` on the site is why a
/// second frame does not twin it.
#[derive(Component)]
pub(crate) struct Flamed;

/// Builds the flame mesh and material once, at `Startup`: the quad the vertex stage billboards, and
/// the material every flame shares.
pub(crate) fn build_flame_assets(
    mut assets: ResMut<FlameAssets>,
    look: Res<FlameLook>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<FlameMaterial>>,
) {
    // A billboard quad: the vertex stage in `flame.wesl` reads its local x and y, faces it at the
    // camera and scales it by the flame's height, so the model is a unit square in the local XY
    // plane about its origin.
    assets.mesh = meshes.add(Rectangle::new(1.0, 1.0));
    assets.material = materials.add(material_for(*look, false));
    assets.faded_material = materials.add(material_for(*look, true));
    assets.max_height = look.max_height;
    assets.built = true;
}

/// The material for a [`FlameLook`]: its colour (green clamped, so a palette's oversaturated fire does
/// not blow the ramp out) and its strength as the ember's weight. `depth_fade` is the material of the
/// sites that asked for [`FlameSite::depth_fade`]: the same flame with the shader's depth test turned on.
fn material_for(look: FlameLook, depth_fade: bool) -> FlameMaterial {
    FlameMaterial {
        base: LinearRgba {
            red: look.color.red,
            green: look.color.green.min(1.0),
            blue: look.color.blue,
            alpha: look.strength,
        },
        params: Vec4::new(0.0, look.max_height, if depth_fade { 1.0 } else { 0.0 }, look.strength),
    }
}

/// Gives every [`FlameSite`] without one a single [`Flame`] child.
///
/// The child carries the site's height, phase and starting brightness on its [`MeshTag`] (see
/// [`pack_flame_tag`](crate::pack_flame_tag)), stands at the site's own origin, and is not a shadow
/// caster. Its culling box is the crate's own ([`flame_cull_extents`]), because the vertex stage scales
/// the unit quad and reads the transform's scale as data, so the mesh's own `Aabb` neither covers what
/// is drawn nor survives the scale: a flame outside every camera's frustum is not extracted or drawn.
/// The starting brightness is the fire's swing at the frame it spawns, so the very first frame a flame
/// is drawn it already matches its light.
///
/// Spawned through `queue_silenced`: a site despawned in the same frame it was created (a short
/// fire's lifetime running out, a carried torch going away) must not panic the insert.
pub(crate) fn dress_flames(
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
        let packed = pack_flame_tag(flame.phase, flame.height, brightness, max_height);
        let tag = MeshTag::new(packed);
        // The culling box of a flame at full scale: `update_flames` rewrites it with the scale.
        let height = unpack_flame_tag(packed, max_height).1;
        let bound = Aabb {
            center: Vec3A::ZERO,
            half_extents: Vec3A::from(flame_cull_extents(height, 1.0, [0.0; 2])),
        };
        let mesh = assets.mesh.clone();
        let material = assets.material_for(flame.depth_fade).clone();
        commands.entity(site).queue_silenced(move |mut site: EntityWorldMut| {
            site.insert(Flamed).with_children(|parent| {
                parent.spawn((
                    Name::new("Flame"),
                    Flame { kindled: if catch > 0.0 { 0.0 } else { f32::MAX }, source: None },
                    tag,
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::IDENTITY,
                    NotShadowCaster,
                    // Bevy's own box is the unit square's, which neither covers the drawn flame nor survives
                    // the scale (`flame_cull_extents`): the box is this crate's, and Bevy must leave it alone.
                    bound,
                    NoAutoAabb,
                ));
            });
        });
    }
}

/// What [`update_flames`] reads and writes of a flame: its catch and source, its transform and the culling box that
/// goes with the transform's scale, its tag and material, and the site it is a child of.
type FlameParts = (
    &'static mut Flame,
    &'static mut Transform,
    &'static mut Aabb,
    &'static mut MeshTag,
    &'static mut MeshMaterial3d<FlameMaterial>,
    &'static ChildOf,
);

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
pub(crate) fn update_flames(
    real: Res<Time<Real>>,
    time: Res<Time>,
    look: Res<FlameLook>,
    assets: Res<FlameAssets>,
    mut materials: ResMut<Assets<FlameMaterial>>,
    mut flames: Query<FlameParts>,
    sites: Sites,
) {
    if !assets.built {
        return;
    }
    // The catch: a flame lit with its fire grows from nothing over the site's `kindle`, on the same
    // `kindle_gain` curve the light uses, so the flame and the light it motivates catch together. A site
    // with no kindle is left alone — a fire burning all along costs nothing here.
    let dt = real.delta_secs();
    let t = time.elapsed_secs();
    // The wind this frame: speed and the unit direction it blows towards, on the ground plane.
    let wind = sites.wind.as_deref().map_or((0.0, Vec2::ZERO), |w| {
        let speed = w.velocity.length();
        (speed, if speed > 0.0 && speed.is_finite() { w.velocity / speed } else { Vec2::ZERO })
    });
    let max_height = assets.max_height;
    for (mut flame, mut transform, mut bound, mut tag, mut material, parent) in &mut flames {
        let height = unpack_flame_tag(tag.value, max_height).1;
        let (site, owner) = sites.sites.get(parent.parent()).map_or((None, None), |(s, o)| (Some(s), o));
        let source = owner.map(ChildOf::parent).filter(|owner| !sites.sites.contains(*owner));
        if flame.source != source {
            flame.source = source;
        }
        let mut lean = Vec2::ZERO;
        if let Some(site) = site {
            // A site whose `depth_fade` changed moves its flame to the other shared material; the comparison
            // reads through `Deref`, so a flame that did not change is not marked changed.
            let wanted = assets.material_for(site.depth_fade);
            if material.0 != *wanted {
                material.0 = wanted.clone();
            }
            // Only when the brightness byte actually changes: `MeshTag` is in `bevy_pbr`'s
            // `Changed<MeshTag>` re-extraction set, so a write here costs a render-world re-extract for
            // that flame. The byte holds 1/255 of the swing, so at 60 fps a 2.6 Hz fire skips some
            // frames; a fire that never changes costs nothing at all.
            let (diameter, depth, phase) = sites.fire_of(site);
            if wind.0 > 0.0 && wind.0.is_finite() {
                lean = wind.1 * flame_tilt(wind.0, diameter.unwrap_or(DEFAULT_FIRE_DIAMETER)).sin();
            }
            let brightness = flicker_swing(diameter, depth, t, phase);
            let packed = set_flame_brightness(tag.value, brightness);
            if packed != tag.value {
                tag.value = packed;
            }
        }
        // A linked fire put out is its light's `Snuff`, on the same `snuff_gain` the light's intensity is multiplied by,
        // so the flame leaves with its light. It outranks a catch, as `Snuff` outranks `Kindle` on the light; removing
        // it hands the scale back through the settled branch below.
        if let Some(elapsed) = site.and_then(|site| sites.snuff_of(site)) {
            set_scale(&mut transform, &mut bound, height, snuff_gain(elapsed).max(0.001), lean);
            if flame.kindled < f32::MAX {
                flame.kindled = f32::MAX;
            }
            continue;
        }
        // A linked fire's catch is its light's `Kindle`, on the same `kindle_gain` the light's intensity uses, so a
        // fire put out and lit again is born small again in step with its light. While that kindle runs it owns the
        // scale and the site's own catch is finished; a `Kindle` already settled leaves the site's own catch alone.
        if let Some(elapsed) =
            site.and_then(|site| sites.kindle_of(site)).filter(|elapsed| *elapsed < KINDLE_SECONDS)
        {
            set_scale(&mut transform, &mut bound, height, kindle_gain(elapsed).max(0.001), lean);
            if flame.kindled < f32::MAX {
                flame.kindled = f32::MAX;
            }
            continue;
        }
        if flame.kindled >= f32::MAX {
            // Done catching. A link whose `Kindle` has gone leaves the scale where it stopped; put it back.
            set_scale(&mut transform, &mut bound, height, 1.0, lean);
            continue;
        }
        flame.kindled = (flame.kindled + dt).min(f32::MAX);
        let catch = site.map_or(0.0, |site| site.kindle);
        let gain = if catch > 0.0 { kindle_gain(flame.kindled.min(catch)) } else { 1.0 };
        // Width and height both: a fire that is catching is smaller in both, and a flame that grew tall
        // while staying wide would read as a sheet. The brightness is in the tag and is written above.
        set_scale(&mut transform, &mut bound, height, gain.max(0.001), lean);
        if flame.kindled >= catch {
            flame.kindled = f32::MAX;
            set_scale(&mut transform, &mut bound, height, 1.0, lean);
        }
    }
    let elapsed = real.elapsed_secs_wrapped();
    for handle in [&assets.material, &assets.faded_material] {
        let Some(mut made) = materials.get_mut(handle) else {
            continue;
        };
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

/// The flame sites, and the lights their [`FlameSite::fire`] links may name: one parameter, since `update_flames` reads both.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Sites<'w, 's> {
    sites: Query<'w, 's, (&'static FlameSite, Option<&'static ChildOf>)>,
    lights: Query<'w, 's, (Option<&'static Flicker>, Option<&'static PoolCandidate>)>,
    kindles: Query<'w, 's, &'static Kindle>,
    snuffs: Query<'w, 's, &'static Snuff>,
    wind: Option<Res<'w, FlameWind>>,
}

impl Sites<'_, '_> {
    /// The `elapsed` of the [`Kindle`] on the entity a site's [`FlameSite::fire`] names, when it has one.
    fn kindle_of(&self, site: &FlameSite) -> Option<f32> {
        self.kindles.get(site.fire?).ok().map(|kindle| kindle.elapsed)
    }

    /// The `elapsed` of the [`Snuff`] on the entity a site's [`FlameSite::fire`] names, when it has one.
    fn snuff_of(&self, site: &FlameSite) -> Option<f32> {
        self.snuffs.get(site.fire?).ok().map(|snuff| snuff.elapsed)
    }

    /// The `(diameter, depth, phase)` a site's flame breathes by: its light's when [`FlameSite::fire`] names one that still
    /// has a [`Flicker`] or a [`PoolCandidate`], otherwise the site's own. A `Flicker` wins over a candidate on the same
    /// entity, since the pool's own light is the `Flicker`. A candidate's missing depth is [`FLICKER_DEPTH`] and its missing
    /// phase the site's, as the pool reads them.
    fn fire_of(&self, site: &FlameSite) -> (Option<f32>, f32, f32) {
        let own = (site.diameter, site.depth, site.phase);
        let Some((flicker, candidate)) = site.fire.and_then(|light| self.lights.get(light).ok()) else {
            return own;
        };
        match (flicker, candidate) {
            (Some(flicker), _) => (flicker.diameter, flicker.depth, flicker.phase),
            (None, Some(candidate)) => (
                candidate.diameter,
                candidate.depth.unwrap_or(FLICKER_DEPTH),
                candidate.phase.unwrap_or(site.phase),
            ),
            (None, None) => own,
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::model::{FLICKER_DEPTH, SNUFF_SECONDS, unpack_flame_tag};
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
                FlameSite {
                    height: 0.9,
                    phase: 0.4,
                    kindle: 0.0,
                    diameter: None,
                    depth: FLICKER_DEPTH,
                    depth_fade: false,
                    fire: None,
                },
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
            FlameSite {
                height: 0.35,
                phase: 2.5,
                kindle: 0.0,
                diameter: Some(0.34),
                depth: FLICKER_DEPTH,
                depth_fade: false,
                fire: None,
            },
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
            FlameSite {
                height: 0.35,
                phase: 2.5,
                kindle: 0.0,
                diameter: None,
                depth: FLICKER_DEPTH,
                depth_fade: false,
                fire: None,
            },
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
            FlameSite {
                height: 0.35,
                phase: 2.5,
                kindle: 0.0,
                diameter: Some(0.34),
                depth: FLICKER_DEPTH,
                depth_fade: false,
                fire: None,
            },
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

    /// Issues #4 and #3: a flame whose site names its light breathes by the light's depth, diameter and phase, not
    /// by the site's own, so the two cannot be set apart; and the light's `set_depth` reaches the flame.
    #[test]
    fn a_flame_breathes_by_the_light_its_site_names() {
        let mut app = dressing_app();
        let light =
            app.world_mut().spawn(Flicker { phase: 1.3, diameter: Some(0.3), depth: 0.30, ..default() }).id();
        let site = FlameSite {
            height: 0.35,
            phase: 2.5,
            kindle: 0.0,
            diameter: None,
            depth: FLICKER_DEPTH,
            depth_fade: false,
            fire: Some(light),
        };
        app.world_mut().spawn((site, Transform::default()));
        for _ in 0..40 {
            app.update();
        }
        let max_height = app.world().resource::<FlameAssets>().max_height;
        let brightness = |app: &mut App| {
            let t = app.world().resource::<Time>().elapsed_secs();
            (unpack_flame_tag(flame_children(app)[0].1, max_height).2, t)
        };
        let (got, t) = brightness(&mut app);
        let theirs = flicker_swing(Some(0.3), 0.30, t, 1.3);
        let own = flicker_swing(None, FLICKER_DEPTH, t, 2.5);
        assert!((own - theirs).abs() > 2.0 / 255.0, "the fixture must tell the two apart: {own} vs {theirs}");
        assert!((got - theirs).abs() < 1.0 / 255.0, "brightness {got} against the light's {theirs}");

        app.world_mut().get_mut::<Flicker>(light).expect("the light").set_depth(0.05);
        app.update();
        let (got, t) = brightness(&mut app);
        let calmer = flicker_swing(Some(0.3), 0.05, t, 1.3);
        assert!((got - calmer).abs() < 1.0 / 255.0, "brightness {got} after set_depth, wanted {calmer}");
    }

    /// A prop the pool lights has a `PoolCandidate` rather than a `Flicker`; the flame reads the depth the pool hands
    /// its slot, `FLICKER_DEPTH` for a candidate with none, and its own phase for a candidate with none.
    #[test]
    fn a_flame_reads_a_pool_candidate_the_way_the_pool_does() {
        let mut app = dressing_app();
        let candidate = app.world_mut().spawn(PoolCandidate { depth: Some(0.30), ..default() }).id();
        let bare = app.world_mut().spawn(PoolCandidate::default()).id();
        let site = |fire| FlameSite {
            height: 0.35,
            phase: 2.5,
            kindle: 0.0,
            diameter: None,
            depth: 0.0,
            depth_fade: false,
            fire: Some(fire),
        };
        let deep = app.world_mut().spawn((site(candidate), Transform::default())).id();
        let plain = app.world_mut().spawn((site(bare), Transform::default())).id();
        for _ in 0..40 {
            app.update();
        }
        let max_height = app.world().resource::<FlameAssets>().max_height;
        let t = app.world().resource::<Time>().elapsed_secs();
        for (site, depth) in [(deep, 0.30), (plain, FLICKER_DEPTH)] {
            let tag =
                flame_children(&mut app).into_iter().find(|(parent, _)| *parent == site).expect("a flame").1;
            let wanted = flicker_swing(None, depth, t, 2.5);
            let got = unpack_flame_tag(tag, max_height).2;
            assert!((got - wanted).abs() < 1.0 / 255.0, "depth {depth}: brightness {got}, wanted {wanted}");
        }
    }

    /// A link to nothing (despawned, or an entity with no fire on it) falls back to the site's own numbers.
    #[test]
    fn a_dangling_fire_link_falls_back_to_the_sites_own_values() {
        let mut app = dressing_app();
        let gone = app.world_mut().spawn_empty().id();
        app.world_mut().despawn(gone);
        let bare = app.world_mut().spawn_empty().id();
        for fire in [gone, bare] {
            app.world_mut().spawn((
                FlameSite {
                    height: 0.35,
                    phase: 2.5,
                    kindle: 0.0,
                    diameter: Some(0.34),
                    depth: 0.25,
                    depth_fade: false,
                    fire: Some(fire),
                },
                Transform::default(),
            ));
        }
        for _ in 0..40 {
            app.update();
        }
        let max_height = app.world().resource::<FlameAssets>().max_height;
        let t = app.world().resource::<Time>().elapsed_secs();
        let wanted = flicker_swing(Some(0.34), 0.25, t, 2.5);
        let flames = flame_children(&mut app);
        assert_eq!(flames.len(), 2);
        for (_, tag) in flames {
            let got = unpack_flame_tag(tag, max_height).2;
            assert!((got - wanted).abs() < 1.0 / 255.0, "brightness {got}, wanted {wanted}");
        }
    }

    /// A fire put out and lit again: re-inserting `Kindle` on the light the site names takes the flame back to a point and
    /// grows it on the light's own `kindle_gain`; once the kindle settles the flame is whole again. A host that never
    /// links, and a link to a light with no `Kindle`, keep a settled flame at full size.
    #[test]
    fn a_relit_fire_catches_its_flame_with_its_light() {
        let mut app = dressing_app();
        let light = app.world_mut().spawn(Kindle { elapsed: KINDLE_SECONDS }).id();
        let site = |fire| {
            (
                FlameSite {
                    height: 0.35,
                    phase: 0.0,
                    kindle: 0.0,
                    diameter: None,
                    depth: FLICKER_DEPTH,
                    depth_fade: false,
                    fire,
                },
                Transform::default(),
            )
        };
        let linked = app.world_mut().spawn(site(Some(light))).id();
        let free = app.world_mut().spawn(site(None)).id();
        for _ in 0..5 {
            app.update();
        }
        let scales = |app: &mut App| {
            app.world_mut()
                .query_filtered::<(&Transform, &ChildOf), With<Flame>>()
                .iter(app.world())
                .map(|(t, c)| (c.parent(), t.scale.x))
                .collect::<Vec<_>>()
        };
        assert!(scales(&mut app).iter().all(|(_, s)| *s == 1.0), "a settled fire draws whole");

        // The host re-lights the fire; the flame follows the light's elapsed time, frame by frame.
        for step in [0.0, 0.15, 0.3, 0.45] {
            app.world_mut().entity_mut(light).insert(Kindle { elapsed: step });
            app.update();
            assert_eq!(flame_children(&mut app).len(), 2);
            let scales = scales(&mut app);
            let scale_of = |site| scales.iter().find(|(parent, _)| *parent == site).map(|(_, s)| *s);
            let wanted = kindle_gain(step).max(0.001);
            let got = scale_of(linked).unwrap_or_default();
            assert!((got - wanted).abs() < 1e-6, "linked flame {got} at elapsed {step}, wanted {wanted}");
            assert_eq!(scale_of(free), Some(1.0), "the unlinked flame is untouched");
        }

        // Settled again: back to exactly whole, and a removed `Kindle` mid-way does not strand the flame small.
        app.world_mut().entity_mut(light).insert(Kindle { elapsed: 0.2 });
        app.update();
        app.world_mut().entity_mut(light).remove::<Kindle>();
        app.update();
        app.update();
        assert!(
            scales(&mut app).iter().all(|(_, s)| *s == 1.0),
            "a flame is whole once its light's kindle is gone"
        );
    }

    /// A fire put out: a `Snuff` on the light the site names shrinks the flame on the light's own `snuff_gain`, an
    /// unlinked flame is left alone, and removing the `Snuff` makes the flame whole again.
    #[test]
    fn a_snuffed_fire_takes_its_flame_out_with_its_light() {
        let mut app = dressing_app();
        let light = app.world_mut().spawn(Kindle { elapsed: KINDLE_SECONDS }).id();
        let site = |fire| {
            (
                FlameSite {
                    height: 0.35,
                    phase: 0.0,
                    kindle: 0.0,
                    diameter: None,
                    depth: FLICKER_DEPTH,
                    depth_fade: false,
                    fire,
                },
                Transform::default(),
            )
        };
        let linked = app.world_mut().spawn(site(Some(light))).id();
        let free = app.world_mut().spawn(site(None)).id();
        for _ in 0..5 {
            app.update();
        }
        let scale_of = |app: &mut App, site: Entity| {
            app.world_mut()
                .query_filtered::<(&Transform, &ChildOf), With<Flame>>()
                .iter(app.world())
                .find(|(_, c)| c.parent() == site)
                .map(|(t, _)| t.scale.x)
                .unwrap_or_default()
        };
        for step in [0.0, 0.3, 0.6, 0.9, SNUFF_SECONDS] {
            app.world_mut().entity_mut(light).insert(Snuff { elapsed: step });
            app.update();
            let wanted = snuff_gain(step).max(0.001);
            let got = scale_of(&mut app, linked);
            assert!((got - wanted).abs() < 1e-6, "linked flame {got} at elapsed {step}, wanted {wanted}");
            assert_eq!(scale_of(&mut app, free), 1.0, "the unlinked flame is untouched");
        }
        app.world_mut().entity_mut(light).remove::<Snuff>();
        app.update();
        app.update();
        assert_eq!(scale_of(&mut app, linked), 1.0, "a flame is whole once its light's snuff is gone");
    }

    /// A sized fire and an un-sized one must not read the same, or the size a host gives the light would
    /// not reach the flame. The two forms part company only once the clock has moved: at t = 0 every
    /// fire's swing is 0.85, whatever its size.
    #[test]
    fn a_sized_flame_and_an_unsized_one_do_not_read_alike() {
        fn brightness_of(diameter: Option<f32>) -> f32 {
            let mut app = dressing_app();
            app.world_mut().spawn((
                FlameSite {
                    height: 0.35,
                    phase: 2.5,
                    kindle: 0.0,
                    diameter,
                    depth: FLICKER_DEPTH,
                    depth_fade: false,
                    fire: None,
                },
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

    /// Issue #2: `depth_fade` is opt-in per site. A site that did not ask is drawn with the material that has
    /// the depth read off, exactly as before the field existed; a site that did is drawn with the one that has
    /// it on, whichever the host sets it to later, and a look change reaches both materials.
    #[test]
    fn a_site_that_asks_for_the_depth_fade_is_drawn_with_the_material_that_has_it() {
        fn fade_of(app: &mut App, site: Entity) -> f32 {
            let flame = app
                .world()
                .get::<Children>(site)
                .and_then(|c| c.first().copied())
                .expect("a dressed site has a flame");
            let handle = app
                .world()
                .get::<MeshMaterial3d<FlameMaterial>>(flame)
                .expect("a flame has a material")
                .0
                .clone();
            app.world()
                .resource::<Assets<FlameMaterial>>()
                .get(&handle)
                .expect("the material exists")
                .params
                .z
        }
        let mut app = dressing_app();
        let site = |depth_fade| FlameSite {
            height: 0.9,
            phase: 0.0,
            kindle: 0.0,
            diameter: None,
            depth: FLICKER_DEPTH,
            depth_fade,
            fire: None,
        };
        let plain = app.world_mut().spawn(site(false)).id();
        let faded = app.world_mut().spawn(site(true)).id();
        app.update();
        assert_eq!(fade_of(&mut app, plain), 0.0);
        assert_eq!(fade_of(&mut app, faded), 1.0);

        app.world_mut().get_mut::<FlameSite>(plain).expect("the site").depth_fade = true;
        app.world_mut().get_mut::<FlameSite>(faded).expect("the site").depth_fade = false;
        app.update();
        assert_eq!(fade_of(&mut app, plain), 1.0);
        assert_eq!(fade_of(&mut app, faded), 0.0);

        app.world_mut().resource_mut::<FlameLook>().strength = 0.5;
        app.update();
        for site in [plain, faded] {
            let flame = app.world().get::<Children>(site).and_then(|c| c.first().copied()).expect("a flame");
            let handle =
                app.world().get::<MeshMaterial3d<FlameMaterial>>(flame).expect("a material").0.clone();
            let material =
                app.world().resource::<Assets<FlameMaterial>>().get(&handle).expect("exists").clone();
            assert!((material.params.w - 0.5).abs() < 1e-6, "{:?}", material.params);
        }
    }

    /// Issue #3: a flame need not be where its light is. A site is its own entity, parented to whatever
    /// the host says the fire is on (a sconce's bracket), while the light hangs elsewhere; the flame
    /// stands at the site's origin and goes when the prop goes — no anchor type needed.
    #[test]
    fn a_site_on_a_prop_keeps_its_flame_at_the_site_and_leaves_with_the_prop() {
        let mut app = dressing_app();
        let prop = app.world_mut().spawn(Transform::from_xyz(4.0, 0.0, 0.0)).id();
        let site = app
            .world_mut()
            .spawn((
                FlameSite {
                    height: 0.35,
                    phase: 0.7,
                    kindle: 0.0,
                    diameter: None,
                    depth: FLICKER_DEPTH,
                    depth_fade: false,
                    fire: None,
                },
                Transform::from_xyz(0.0, 1.5, 0.0),
                ChildOf(prop),
            ))
            .id();
        app.update();
        let flames = flame_children(&mut app);
        assert_eq!(flames.len(), 1, "one flame: {flames:?}");
        assert_eq!(flames[0].0, site, "the flame is the site's child, wherever the site stands");
        let local = app
            .world_mut()
            .query_filtered::<&Transform, With<Flame>>()
            .single(app.world())
            .copied()
            .unwrap_or_default();
        assert_eq!(local.translation, Vec3::ZERO, "the flame's foot is the site's origin");
        app.world_mut().despawn(prop);
        app.update();
        assert!(flame_children(&mut app).is_empty(), "a prop that goes takes its flame with it");
    }

    /// Issue #2: a flame names the entity it is the fire of — its site's parent — so a diagnostic can
    /// answer "whose torch is that"; a root site, or one nested under another site, has none, and a
    /// site the host reparents is followed rather than left pointing at its old torch.
    #[test]
    fn a_flame_names_the_prop_its_site_hangs_on_and_follows_a_reparent() {
        let site_of = || FlameSite {
            height: 0.35,
            phase: 0.0,
            kindle: 0.0,
            diameter: None,
            depth: FLICKER_DEPTH,
            depth_fade: false,
            fire: None,
        };
        let mut app = dressing_app();
        let torch = app.world_mut().spawn(Transform::default()).id();
        let other = app.world_mut().spawn(Transform::default()).id();
        let on_torch = app.world_mut().spawn((site_of(), ChildOf(torch))).id();
        let on_root = app.world_mut().spawn(site_of()).id();
        let nested = app.world_mut().spawn((site_of(), ChildOf(on_root))).id();
        app.update();
        let source_of = |app: &mut App, site: Entity| -> Option<Option<Entity>> {
            app.world_mut()
                .query::<(&Flame, &ChildOf)>()
                .iter(app.world())
                .find(|(_, parent)| parent.parent() == site)
                .map(|(flame, _)| flame.source)
        };
        assert_eq!(source_of(&mut app, on_torch), Some(Some(torch)));
        assert_eq!(source_of(&mut app, on_root), Some(None), "a root site belongs to no prop");
        assert_eq!(source_of(&mut app, nested), Some(None), "a site under a site belongs to the outer site");
        app.world_mut().entity_mut(on_torch).insert(ChildOf(other));
        app.update();
        assert_eq!(source_of(&mut app, on_torch), Some(Some(other)), "a reparented site is followed");
        app.world_mut().entity_mut(on_torch).remove::<ChildOf>();
        app.update();
        assert_eq!(source_of(&mut app, on_torch), Some(None), "an unparented site has no source");
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod kindle_tests {
    use super::*;
    use crate::KINDLE_SECONDS;
    use crate::model::FLICKER_DEPTH;
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
            depth_fade: false,
            fire: None,
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
            FlameSite {
                height: 0.9,
                phase: 0.0,
                kindle: 0.0,
                diameter: None,
                depth: FLICKER_DEPTH,
                depth_fade: false,
                fire: None,
            },
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
    use crate::model::FLICKER_DEPTH;
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
            .add_systems(Update, (gust_wind, dress_flames, update_flames).chain());
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
                depth_fade: false,
                fire: None,
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
                depth_fade: false,
                fire: None,
            },
            Transform::default(),
        ));
        app.world_mut().spawn((
            FlameSite {
                height: 0.9,
                phase: 1.3,
                kindle: 0.0,
                diameter: None,
                depth: FLICKER_DEPTH,
                depth_fade: false,
                fire: None,
            },
            Transform::from_xyz(5.0, 0.0, 0.0),
        ));
        app.update();
        app.update();
        let catching = scales(&mut app);
        assert_eq!(catching.len(), 2, "two flames: {catching:?}");
        assert!(catching[0] < 0.2, "the catching one is small: {catching:?}");
        assert!((catching[1] - 1.0).abs() < 1e-5, "the steady one is untouched: {catching:?}");
    }

    /// A site's flame and its instance scale after one more frame.
    fn flame_scale(app: &mut App) -> Vec3 {
        app.update();
        app.world_mut().query_filtered::<&Transform, With<Flame>>().single(app.world()).unwrap().scale
    }

    fn site_with(diameter: Option<f32>) -> FlameSite {
        FlameSite {
            height: 0.9,
            phase: 0.0,
            kindle: 0.0,
            diameter,
            depth: FLICKER_DEPTH,
            depth_fade: false,
            fire: None,
        }
    }

    #[test]
    fn a_flame_leans_downwind_by_its_own_fires_size_and_stands_upright_in_still_air() {
        let mut app = app();
        app.world_mut().spawn((site_with(Some(0.34)), Transform::default()));
        assert_eq!(flame_scale(&mut app), Vec3::ONE, "no wind resource: upright");

        app.insert_resource(FlameWind { velocity: Vec2::new(3.0, 0.0) });
        let scale = flame_scale(&mut app);
        let want = crate::model::flame_tilt(3.0, 0.34).sin();
        assert!((scale.y - (1.0 + want)).abs() < 1e-6, "downwind along +X: {scale:?}");
        assert_eq!(scale.z, 1.0, "nothing across it: {scale:?}");
        assert_eq!(scale.x, 1.0, "the catch is untouched: {scale:?}");

        app.insert_resource(FlameWind { velocity: Vec2::new(0.0, -3.0) });
        let scale = flame_scale(&mut app);
        assert!((scale.z - (1.0 - want)).abs() < 1e-6, "downwind along -Z: {scale:?}");
        assert_eq!(scale.y, 1.0);

        app.insert_resource(FlameWind::default());
        assert_eq!(flame_scale(&mut app), Vec3::ONE, "the wind dropped: upright again");
    }

    #[test]
    fn a_larger_fire_leans_less_in_the_same_wind() {
        let mut app = app();
        app.insert_resource(FlameWind { velocity: Vec2::new(2.0, 0.0) });
        app.world_mut().spawn((site_with(Some(0.2)), Transform::default()));
        app.world_mut().spawn((site_with(Some(2.0)), Transform::default()));
        app.update();
        app.update();
        let mut lean: Vec<(f32, f32)> = app
            .world_mut()
            .query_filtered::<(&Transform, &ChildOf), With<Flame>>()
            .iter(app.world())
            .map(|(t, c)| (site_diameter(app.world(), c.parent()), t.scale.y - 1.0))
            .collect();
        lean.sort_by(|a, b| a.0.total_cmp(&b.0));
        assert!(lean[0].1 > lean[1].1 && lean[1].1 > 0.0, "{lean:?}");
    }

    fn site_diameter(world: &World, site: Entity) -> f32 {
        world.get::<FlameSite>(site).and_then(|s| s.diameter).unwrap_or(f32::NAN)
    }

    #[test]
    fn a_wind_that_is_not_a_number_or_infinite_leaves_the_flame_upright() {
        let mut app = app();
        app.world_mut().spawn((site_with(None), Transform::default()));
        for velocity in [Vec2::new(f32::NAN, 0.0), Vec2::new(f32::INFINITY, 0.0)] {
            app.insert_resource(FlameWind { velocity });
            assert_eq!(flame_scale(&mut app), Vec3::ONE, "{velocity:?}");
        }
    }

    #[test]
    fn a_gust_drives_the_wind_and_the_lean_changes_smoothly_frame_to_frame() {
        let mut app = app();
        app.world_mut().spawn((site_with(Some(0.34)), Transform::default()));
        app.insert_resource(FlameGust { mean: Vec2::new(2.0, 0.0), seed: 5, ..default() });
        let mut leans = Vec::new();
        for _ in 0..600 {
            let scale = flame_scale(&mut app);
            leans.push(scale.y - 1.0);
        }
        let (lo, hi) = leans.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &l| (lo.min(l), hi.max(l)));
        assert!(hi - lo > 0.1, "the lean never moved: {lo}..{hi}");
        let worst = leans.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0, f32::max);
        assert!(worst < 0.05, "one frame jumped the lean by {worst}");
        assert!(app.world().get_resource::<FlameWind>().is_some(), "the plugin inserted the wind it drives");
    }

    #[test]
    fn without_a_gust_a_hosts_own_wind_is_left_alone() {
        let mut app = app();
        app.world_mut().spawn((site_with(Some(0.34)), Transform::default()));
        app.insert_resource(FlameWind { velocity: Vec2::new(3.0, 0.0) });
        for _ in 0..30 {
            app.update();
        }
        assert_eq!(app.world().resource::<FlameWind>().velocity, Vec2::new(3.0, 0.0));
    }

    #[test]
    fn a_gust_with_no_turbulence_is_the_steady_mean_and_a_bad_setting_is_reported() {
        let mut app = app();
        app.insert_resource(FlameGust { mean: Vec2::new(0.0, 2.0), intensity: 0.0, ..default() });
        app.update();
        assert_eq!(app.world().resource::<FlameWind>().velocity, Vec2::new(0.0, 2.0));
        let bad = FlameGust { mean: Vec2::new(f32::NAN, 0.0), intensity: -1.0, ..default() };
        let fields: Vec<_> = bad.problems().iter().map(|p| p.field).collect();
        assert_eq!(fields, ["mean.x", "intensity"]);
        assert!(FlameGust::default().problems().is_empty());
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
pub(crate) mod culling_tests {
    use super::*;
    use crate::model::{FLAME_REACH, FLICKER_DEPTH};
    use crate::{FireKind, KINDLE_SECONDS};
    use bevy::camera::primitives::{Frustum, Sphere};
    use bevy::camera::{PerspectiveProjection, Projection};
    use bevy::math::Affine3A;
    use bevy::time::TimeUpdateStrategy;
    use core::f32::consts::{FRAC_PI_2, TAU};

    /// A seeded stream, so the sweep is the same on every machine.
    struct Draw(u64);

    impl Draw {
        fn unit(&mut self) -> f32 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 >> 40) as f32 / (1u64 << 24) as f32
        }

        fn range(&mut self, low: f32, high: f32) -> f32 {
            low + (high - low) * self.unit()
        }
    }

    fn smoothstep(low: f32, high: f32, x: f32) -> f32 {
        let t = ((x - low) / (high - low)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    /// Points on the flame's quad, a 5 x 5 grid of its corners and interior, where `src/flame.wesl`'s vertex
    /// stage puts them: the same constants, the same two branches (the upright quad, and the quad that turns to
    /// face a camera looking down on it), the same lean, for a camera at `eye` whose right axis is `camera_right`.
    /// `origin` is the flame's foot, `size` the height it is drawn at, `wind` the instance scale's `y` and `z` less
    /// one.
    fn drawn_points(origin: Vec3, size: f32, wind: Vec2, eye: Vec3, camera_right: Vec3) -> Vec<Vec3> {
        const ASPECT: f32 = 0.6;
        const SOURCE_HEIGHT: f32 = 0.35;
        const OVERHEAD_START: f32 = 0.873;
        const OVERHEAD_FULL: f32 = 1.326;
        const OVERHEAD_HEIGHT: f32 = 1.0;
        let right = Vec3::new(camera_right.x, 0.0, camera_right.z).normalize();
        let centre = origin + Vec3::Y * (size * SOURCE_HEIGHT);
        let to_eye = eye - centre;
        let sin_e = (to_eye.y / to_eye.length().max(1e-4)).clamp(0.0, 1.0);
        let topdown = smoothstep(OVERHEAD_START, OVERHEAD_FULL, sin_e.asin());
        let wind = wind * (1.0 - topdown);
        let lean = wind.dot(Vec2::new(right.x, right.z));
        let width = ASPECT + lean.abs();
        let mut points = Vec::new();
        for i in 0..5 {
            for j in 0..5 {
                let (x, y) = (i as f32 / 4.0 - 0.5, j as f32 / 4.0 - 0.5);
                let across = x * width + 0.5 * lean;
                let mut at = origin + right * (across * size) + Vec3::Y * (y * size + size * 0.5);
                if sin_e > OVERHEAD_START.sin() {
                    let screen_up = to_eye.normalize().cross(right).normalize();
                    at = centre + right * (across * size) + screen_up * (y * size * OVERHEAD_HEIGHT);
                }
                points.push(at);
            }
        }
        points
    }

    /// The whole chain, as a camera sees it, for the quad a fire's vertex stage draws: the quad's points (`drawn_points`, given
    /// the fire's foot, the height it is drawn at, its lean, the camera's eye and its right axis), the box and the scale this
    /// crate gives it (`extents`, cut from a ball of `reach` drawn heights about the foot), and Bevy's own frustum test (the
    /// sphere test and then the oriented box, as `check_visibility` runs them). A quad culled while any of its drawn points is
    /// inside the frame is one that pops out at the edge of the screen.
    pub(crate) fn assert_a_quad_on_screen_is_never_culled(
        extents: fn(f32, f32, [f32; 2]) -> [f32; 3],
        reach: fn([f32; 2]) -> f32,
        drawn_points: fn(Vec3, f32, Vec2, Vec3, Vec3) -> Vec<Vec3>,
    ) {
        let mut draw = Draw(0x9e37_79b9_7f4a_7c15);
        let projection = Projection::Perspective(PerspectiveProjection {
            fov: 1.0,
            aspect_ratio: 16.0 / 9.0,
            near: 0.1,
            far: 1000.0,
            ..default()
        });
        let clip_from_view = projection.get_clip_from_view();
        let (mut on_screen, mut edge) = (0, 0);
        for round in 0..40_000 {
            let origin = Vec3::new(draw.range(-3.0, 3.0), draw.range(0.0, 2.0), draw.range(-3.0, 3.0));
            let height = draw.range(0.04, 3.0);
            let took = if draw.unit() < 0.4 { 1.0 } else { draw.range(0.001, 1.0) };
            let tilt = if draw.unit() < 0.25 { 0.0 } else { draw.range(0.0, 0.995) };
            let heading = draw.range(0.0, TAU);
            let lean = Vec2::new(heading.cos(), heading.sin()) * tilt;
            // A camera at any distance and elevation, aimed so the flame lands anywhere from the middle of the
            // frame to well outside it.
            let around = draw.range(0.0, TAU);
            let elevation = draw.range(-0.3, FRAC_PI_2);
            let distance = draw.range(0.3, 14.0);
            let eye = origin
                + Vec3::new(around.cos() * elevation.cos(), elevation.sin(), around.sin() * elevation.cos())
                    * distance;
            let aim = origin
                + Vec3::Y * (height * 0.4)
                + Vec3::new(draw.range(-1.0, 1.0), draw.range(-1.0, 1.0), draw.range(-1.0, 1.0))
                    * distance
                    * 0.7;
            let camera = GlobalTransform::from(Transform::from_translation(eye).looking_at(aim, Vec3::Y));
            let frustum: Frustum = projection.compute_frustum(&camera);

            // The parent may be turned any way: the scale is read as lengths, so a rotation must not matter.
            let parent = if round % 2 == 0 {
                Quat::IDENTITY
            } else {
                Quat::from_euler(
                    EulerRot::YXZ,
                    draw.range(0.0, TAU),
                    draw.range(-1.5, 1.5),
                    draw.range(0.0, TAU),
                )
            };
            let world_from_local = Affine3A::from_scale_rotation_translation(
                Vec3::new(took, 1.0 + lean.x, 1.0 + lean.y),
                parent,
                origin,
            );
            let global = GlobalTransform::from(world_from_local);
            let [hx, hy, hz] = extents(height, took, lean.to_array());
            let bound = Aabb { center: Vec3A::ZERO, half_extents: Vec3A::new(hx, hy, hz) };

            let size = height * took.powf(2.0 / 3.0);
            let world_from_view = camera.affine();
            let view_from_world = world_from_view.inverse();
            let points = drawn_points(origin, size, lean, eye, camera.right().as_vec3());
            // The constant the box is built from: no point the vertex stage draws is further from the foot than
            // `reach` of its drawn heights, from any side, at any lean.
            let ball = (reach(lean.to_array()) * size) * (1.0 + 1e-4);
            let worst = points.iter().map(|p| p.distance(origin)).fold(0.0, f32::max);
            assert!(
                worst <= ball,
                "round {round}: a point {worst} from the foot of a quad of drawn height {size}, lean {lean}, ball {ball}"
            );
            let all = points.len();
            let seen = points
                .into_iter()
                .filter(|p| {
                    let clip = clip_from_view * view_from_world.transform_point3(*p).extend(1.0);
                    clip.w > 0.0
                        && clip.x.abs() <= clip.w
                        && clip.y.abs() <= clip.w
                        && clip.z > 0.0
                        && clip.z <= clip.w
                })
                .count();
            if seen == 0 {
                continue;
            }
            on_screen += 1;
            if seen < all {
                edge += 1;
            }
            let sphere = Sphere {
                center: world_from_local.transform_point3a(bound.center),
                radius: global.radius_vec3a(bound.half_extents),
            };
            assert!(
                frustum.intersects_sphere(&sphere, false)
                    && frustum.intersects_obb(&bound, &world_from_local, true, false),
                "round {round}: {seen} of {all} points of a quad of a {height} m fire (catch {took}, lean {lean}) are on screen, yet it is culled"
            );
        }
        // The sweep must have exercised what it claims: many frames with the quad on screen, and many of them
        // at the frame's edge, where a box that is too small is the one that fails.
        assert!(on_screen > 8_000, "only {on_screen} samples had the quad on screen");
        assert!(edge > 1_500, "only {edge} samples had it half out of the frame");
    }

    #[test]
    fn a_flame_any_part_of_which_is_on_screen_is_never_culled() {
        // The quad the vertex stage draws (`drawn_points`), the box and the scale this crate gives the flame, and Bevy's
        // own frustum test: a flame culled while any of its drawn points is inside the frame pops out at the screen's edge.
        assert_a_quad_on_screen_is_never_culled(flame_cull_extents, |_| FLAME_REACH, drawn_points);
    }

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

    /// The flame's box as the frustum test sees it, on its three axes: the half-extent in the entity's own
    /// axes times the scale the shader reads.
    fn world_half_extents(app: &mut App) -> (Vec3, Vec3) {
        let (transform, bound) = app
            .world_mut()
            .query_filtered::<(&Transform, &Aabb), With<Flame>>()
            .single(app.world())
            .map(|(t, b)| (*t, *b))
            .unwrap();
        (transform.scale, Vec3::from(bound.half_extents) * transform.scale)
    }

    #[test]
    fn a_flames_box_keeps_its_size_through_the_catch_and_the_lean() {
        // The scale is the shader's data (its catch and its lean), so it changes every frame the fire catches or
        // the wind gusts. The frustum test multiplies the box by it; the box must have been divided by it first,
        // so what the test sees is the cube round the drawn flame at every moment, and not a slab.
        let mut app = app();
        app.insert_resource(FlameWind { velocity: Vec2::new(3.0, -1.5) });
        let height = 0.9;
        app.world_mut().spawn((
            FlameSite {
                height,
                phase: 0.0,
                kindle: KINDLE_SECONDS,
                diameter: None,
                depth: FLICKER_DEPTH,
                depth_fade: false,
                fire: None,
            },
            Transform::default(),
        ));
        let mut seen_small = false;
        for frame in 0..(KINDLE_SECONDS * 60.0) as usize + 6 {
            app.update();
            let (scale, world) = world_half_extents(&mut app);
            // The shader clamps the catch to 1 (a light's flare overshoots it), so the drawn size does too; the
            // box's `x` is divided by the clamped catch and multiplied by the real scale, and may exceed the cube
            // by exactly the overshoot, never fall short of it.
            let size = height * scale.x.min(1.0).powf(2.0 / 3.0);
            let want = Vec3::splat(FLAME_REACH * size);
            let slack = 1e-4 * (1.0 + want.max_element());
            assert!(
                (world.y - want.y).abs() < slack
                    && (world.z - want.z).abs() < slack
                    && world.x >= want.x - slack
                    && world.x <= want.x * scale.x.max(1.0) + slack,
                "frame {frame}: scale {scale}, the box measures {world} and the drawn flame wants {want}"
            );
            seen_small |= scale.x < 0.3 && (scale.y - 1.0).abs() > 0.1;
        }
        assert!(
            seen_small,
            "the run never had a small, leaning flame, so it did not test the catch and the lean together"
        );
    }

    #[test]
    fn a_flame_carries_a_box_of_its_own_and_is_not_left_to_the_units_quad() {
        let mut app = app();
        app.world_mut().spawn((FireKind::Torch.site(), Transform::default()));
        app.update();
        let flame = app.world_mut().query_filtered::<Entity, With<Flame>>().single(app.world()).unwrap();
        let entity = app.world().entity(flame);
        assert!(entity.contains::<Aabb>(), "a flame has a culling box");
        assert!(
            entity.contains::<bevy::camera::visibility::NoAutoAabb>(),
            "which Bevy is told not to recompute"
        );
        assert!(
            !entity.contains::<bevy::camera::visibility::NoFrustumCulling>(),
            "and which the frustum test uses: an off-screen flame is not drawn"
        );
    }
}
