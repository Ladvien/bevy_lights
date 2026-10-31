//! Fuel beds: the coals under a fire, alive with it.
//!
//! Add [`FuelBed`] to a [`FlameSite`](crate::FlameSite) and the site gains one child, a flat disc (`src/bed.wesl`) at its foot on
//! which a field of coal lumps glows: dark ash with orange and yellow shards in it, hottest under the flame's axis and ragged at the rim.
//! The coals are not a picture. They **breathe with the fire's flicker**, **warm as the fire catches** and, when the fire is put out
//! (a [`Snuff`](crate::Snuff) on the site's light), **go on glowing and cool** over a minute instead of vanishing with the flame, the way a
//! campfire that is out is a ring of dull red going to black. Light the fire again and the coals warm back.
//!
//! One shared material draws every bed; what is a bed's own (its heat, and where in the tile its window starts) rides its `MeshTag`, as a
//! flame's phase and height do, so a scene of fifty campfires is fifty draws of one material. The glow is the temperature ramp of
//! `coal_radiance` moved by the bed's heat (`bed_radiance`), baked once into a table the shader reads; the lumps are the crate's char
//! field made tileable (`char_tile`) and laid at the coals' own size. The design and its sources are in `docs/research/fuel-bed-heat.md`.

use bevy::asset::{RenderAssetUsages, embedded_asset};
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::light::NotShadowCaster;
use bevy::mesh::{MeshTag, MeshVertexBufferLayoutRef};
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin, MeshMaterial3d};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, RenderPipelineDescriptor, SpecializedMeshPipelineError, TextureDimension,
    TextureFormat,
};
use bevy::shader::ShaderRef;
use bevy::time::Real;

use crate::flame::{FlameLook, FlameSite, Sites};
use crate::model::{
    BED_RAMP_FIELD, BED_RAMP_HEAT, BED_TILE, DEFAULT_BED_COOLING, KINDLE_SECONDS, bed_drawn_heat,
    bed_heat_step, bed_offset, bed_ramp, bed_visible_heat, char_tile, flicker_swing, pack_bed_tag,
};

/// The bed shader, embedded in the binary by [`FlamePlugin`](crate::FlamePlugin).
const BED_SHADER_PATH: &str = "embedded://bevy_lights/bed.wesl";

/// How far above the site's foot the disc lies, in metres: far enough that it does not fight the ground it lies on for the depth test.
const BED_LIFT: f32 = 0.012;

/// The side of the coal field's tile, in texels: a tile is `BED_TILE` (0.36 m) across, so a texel is 1.4 mm, under a pixel at every
/// distance a bed is looked at from.
const FIELD_SIDE: u32 = 256;

/// A bed of coals under a fire: add it to a [`FlameSite`](crate::FlameSite) to give the fire a bed that glows with it, remove it to take
/// the bed away.
///
/// A site without one is drawn exactly as it always was: no extra entity, no extra draw. The bed is a flat disc `diameter` wide at the
/// site's foot, an opaque cut-out with a ragged edge (the dark ash and char between the coals are part of it, so the lumps are read against
/// something; an additive glow alone is lost on the ground the same fire has lit), under the flames and under the logs and stones above it,
/// which hide it through the depth test. Its strength is [`FlameLook::strength`](crate::FlameLook::strength). The coals'
/// heat follows the site's fire, [`FlameSite::fire`](crate::FlameSite::fire): the same flicker the flame is drawn with, the fire's
/// [`Kindle`](crate::Kindle) (a fire being lit warms its bed over a few seconds) and its [`Snuff`](crate::Snuff) (a fire put out leaves the
/// coals glowing, cooling with time constant [`cooling`](Self::cooling)). A site with no `fire` is a fire that always burns.
///
/// Give it to the fires that have a bed to show (a campfire, a bonfire, a brazier: [`FireKind::bed`](crate::FireKind::bed) says which) and
/// not to a torch or a candle. One disc is one draw. The default is a campfire's.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
#[reflect(Component, Default)]
pub struct FuelBed {
    /// The bed's width in the host's units: a disc of this diameter, centred on the site, with the coals laid at their own size (a coal is
    /// about 6 cm across), so a wider bed has more of them, not larger ones. A diameter that is not a positive number draws no bed.
    pub diameter: f32,
    /// Which window of the shared coal field this bed shows: two beds with different seeds show different coals, and a bed is the same on
    /// every machine.
    pub seed: u32,
    /// The time constant, in seconds, of the coals' cooling once the fire is put out: their heat falls as `exp(-t / cooling)`, and the glow
    /// is gone in about a minute at the default (a real bed smoulders for tens of minutes, which no scene waits for; a host that wants a
    /// quick hiss sets it to a few seconds). A design default (`docs/research/fuel-bed-heat.md`).
    pub cooling: f32,
}

impl Default for FuelBed {
    fn default() -> Self {
        Self { diameter: 0.7, seed: 0, cooling: DEFAULT_BED_COOLING }
    }
}

impl FuelBed {
    /// What is unusable in these settings, field by field; empty when they are fine. See
    /// [`Flicker::problems`](crate::Flicker::problems).
    pub fn problems(&self) -> Vec<crate::Problem> {
        let mut out = Vec::new();
        crate::model::problem::positive(&mut out, "diameter", self.diameter);
        crate::model::problem::positive(&mut out, "cooling", self.cooling);
        out
    }

    /// The disc's width, or `None` for a diameter that is not a positive number (no bed to draw).
    fn width(&self) -> Option<f32> {
        (self.diameter.is_finite() && self.diameter > 0.0).then_some(self.diameter)
    }
}

/// The bed material: the colour of every coal at every heat, the clock and the scene's strength, and the field of lumps. Private: a host
/// never builds one.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct FuelBedMaterial {
    /// `x` seconds (wrapped hourly), `y` the tile's width in metres, `z` the strength ([`FlameLook::strength`](crate::FlameLook::strength)).
    #[uniform(0)]
    params: Vec4,
    /// `bed_ramp`: [`BED_RAMP_HEAT`] rows of [`BED_RAMP_FIELD`] columns of the coals' colour.
    #[uniform(0)]
    ramp: [Vec4; BED_RAMP_HEAT * BED_RAMP_FIELD],
    /// The tileable char field, one byte a texel.
    #[texture(1)]
    #[sampler(2)]
    field: Handle<Image>,
}

impl Material for FuelBedMaterial {
    fn vertex_shader() -> ShaderRef {
        BED_SHADER_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        BED_SHADER_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        // An opaque cut-out: the shader `discard`s past the bed's ragged edge, and what it keeps is drawn before every transparent layer, so
        // the additive flames burn over it.
        AlphaMode::Mask(0.5)
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
        // Seen from either side: the disc's winding is not the camera's business.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// Registers the bed's shader, material, assets and systems; [`FlamePlugin`](crate::FlamePlugin) calls it.
pub(crate) fn build(app: &mut App) {
    embedded_asset!(app, "bed.wesl");
    app.add_plugins(MaterialPlugin::<FuelBedMaterial>::default())
        .init_resource::<BedAssets>()
        .register_type::<FuelBed>()
        .add_systems(Startup, build_bed_assets);
}

/// The disc, the one material every bed shares, and the heat under which a bed is too dark to draw.
#[derive(Resource, Default)]
pub(crate) struct BedAssets {
    mesh: Handle<Mesh>,
    material: Handle<FuelBedMaterial>,
    dark: f32,
    built: bool,
}

/// The disc of a site: a child of the site, the bed's own heat, and nothing the host spawns.
#[derive(Component)]
pub(crate) struct Bed;

/// The heat of a bed's coals: the thermal state the fire's flicker is drawn over, `0..=1`.
#[derive(Component)]
pub(crate) struct BedHeat(f32);

/// The marker that a site already has its disc.
#[derive(Component)]
pub(crate) struct Bedded;

/// The coal field as an image: [`char_tile`] over one tile, a byte a texel, with its mip chain (a bed seen from far away is a few pixels
/// of the average, not a crawl of single texels). Repeating, linear.
fn field_image() -> Image {
    let side = FIELD_SIDE as usize;
    let mut level: Vec<u8> = Vec::with_capacity(side * side);
    for y in 0..side {
        for x in 0..side {
            let field = char_tile(x as f32 / side as f32, y as f32 / side as f32, 7);
            level.push((field * 255.0).round() as u8);
        }
    }
    let mut data = level.clone();
    let mut width = side;
    let mut mips = 1;
    while width > 1 {
        let next = width / 2;
        let mut smaller = Vec::with_capacity(next * next);
        for y in 0..next {
            for x in 0..next {
                let sum = [(0, 0), (1, 0), (0, 1), (1, 1)]
                    .into_iter()
                    .map(|(dx, dy)| u32::from(level[(2 * y + dy) * width + 2 * x + dx]))
                    .sum::<u32>();
                smaller.push(((sum + 2) / 4) as u8);
            }
        }
        data.extend_from_slice(&smaller);
        level = smaller;
        width = next;
        mips += 1;
    }
    // `Image::new` asserts the data is one level's size (in a debug build, where a host develops); the mip chain follows the base level
    // in the same buffer, which is what the texture's upload takes, so the image is made empty and filled.
    let mut image = Image::new_uninit(
        Extent3d { width: FIELD_SIDE, height: FIELD_SIDE, depth_or_array_layers: 1 },
        TextureDimension::D2,
        TextureFormat::R8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.data = Some(data);
    image.texture_descriptor.mip_level_count = mips;
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}

fn build_bed_assets(
    mut assets: ResMut<BedAssets>,
    look: Res<FlameLook>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<FuelBedMaterial>>,
) {
    let mut ramp = [Vec4::ZERO; BED_RAMP_HEAT * BED_RAMP_FIELD];
    for (slot, entry) in ramp.iter_mut().zip(bed_ramp()) {
        *slot = Vec4::from(entry);
    }
    assets.mesh = meshes.add(Plane3d::default().mesh().size(1.0, 1.0));
    assets.material = materials.add(FuelBedMaterial {
        params: Vec4::new(0.0, BED_TILE, look.strength, 0.0),
        ramp,
        field: images.add(field_image()),
    });
    assets.dark = bed_visible_heat();
    assets.built = true;
}

/// What a bed's disc is read and written through: its parent, its heat, its tag, its transform and its visibility.
type BedParts = (
    &'static ChildOf,
    &'static mut BedHeat,
    &'static mut MeshTag,
    &'static mut Transform,
    &'static mut Visibility,
);

/// Keeps every [`FuelBed`] site's disc in step with the component and with the fire: dresses a site that has the component and no disc,
/// takes the disc away when the component goes, and each frame moves the coals' heat (warming while the fire burns, cooling while it is
/// out), writes the heat the disc is drawn at into its tag when the byte changes, hides a bed that has gone dark and ticks the shared
/// clock and a changed [`FlameLook`].
///
/// The heat runs on virtual time, as the flicker does: a paused world's coals hold. The shape's clock (the shimmer of the lumps) is the
/// material's and runs on real time, as the flame's does.
#[allow(clippy::too_many_arguments)]
pub(crate) fn update_beds(
    mut commands: Commands,
    real: Res<Time<Real>>,
    time: Res<Time>,
    look: Res<FlameLook>,
    assets: Res<BedAssets>,
    mut materials: ResMut<Assets<FuelBedMaterial>>,
    undressed: Query<(Entity, &FlameSite, &FuelBed), Without<Bedded>>,
    fuel: Query<(&FlameSite, &FuelBed)>,
    mut beds: Query<BedParts, With<Bed>>,
    mut removed: RemovedComponents<FuelBed>,
    children: Query<&Children>,
    marks: Query<(), With<Bed>>,
    owners: Sites,
) {
    if !assets.built {
        return;
    }
    for site in removed.read() {
        if let Ok(kids) = children.get(site) {
            for kid in kids.iter().filter(|kid| marks.contains(*kid)) {
                commands.entity(kid).try_despawn();
            }
        }
        commands.entity(site).try_remove::<Bedded>();
    }
    for (entity, site, wanted) in &undressed {
        // A fire still catching starts its bed cold, and a fire already put out starts it cold too: only a fire that has been burning has
        // a bed that is already hot.
        let cold =
            owners.snuff_of(site).is_some() || owners.kindle_of(site).is_some_and(|k| k < KINDLE_SECONDS);
        let heat = if cold { 0.0 } else { 1.0 };
        let swing = flicker_swing(site.diameter, site.depth, time.elapsed_secs(), site.phase);
        let tag = pack_bed_tag(bed_drawn_heat(heat, swing), bed_offset(wanted.seed));
        let width = wanted.width().unwrap_or(0.0);
        let mesh = assets.mesh.clone();
        let material = assets.material.clone();
        commands.entity(entity).queue_silenced(move |mut site: EntityWorldMut| {
            site.insert(Bedded).with_children(|parent| {
                parent.spawn((
                    Name::new("Fuel bed"),
                    Bed,
                    BedHeat(heat),
                    MeshTag::new(tag),
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                    Transform::from_xyz(0.0, BED_LIFT, 0.0).with_scale(Vec3::new(width, 1.0, width)),
                    NotShadowCaster,
                    // Stated here as well as required by `Mesh3d`, because `update_beds` writes it and a headless app has no plugin that
                    // makes a mesh visible.
                    Visibility::Inherited,
                ));
            });
        });
    }
    let t = time.elapsed_secs();
    let dt = time.delta_secs();
    for (parent, mut heat, mut tag, mut at, mut visibility) in &mut beds {
        let Ok((site, wanted)) = fuel.get(parent.parent()) else { continue };
        let burning = owners.snuff_of(site).is_none();
        let now = bed_heat_step(heat.0, burning, dt, wanted.cooling);
        if now != heat.0 {
            heat.0 = now;
        }
        let (diameter, depth, phase) = owners.fire_of(site);
        let drawn = bed_drawn_heat(now, flicker_swing(diameter, depth, t, phase));
        let packed = pack_bed_tag(drawn, bed_offset(wanted.seed));
        if packed != tag.value {
            tag.value = packed;
        }
        let width = wanted.width();
        let scale = Vec3::new(width.unwrap_or(0.0), 1.0, width.unwrap_or(0.0));
        if at.scale != scale {
            at.scale = scale;
        }
        let wanted_visibility =
            if width.is_some() && drawn >= assets.dark { Visibility::Inherited } else { Visibility::Hidden };
        if *visibility != wanted_visibility {
            *visibility = wanted_visibility;
        }
    }
    if !beds.is_empty() {
        if let Some(mut made) = materials.get_mut(&assets.material) {
            made.params.x = real.elapsed_secs_wrapped();
            if look.is_changed() {
                made.params.z = look.strength;
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::flicker::Snuff;
    use crate::model::{FLICKER_DEPTH, unpack_bed_tag};

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(Assets::<Mesh>::default())
            .insert_resource(Assets::<Image>::default())
            .insert_resource(Assets::<FuelBedMaterial>::default())
            .init_resource::<FlameLook>()
            .init_resource::<BedAssets>()
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_secs_f64(1.0 / 60.0),
            ));
        app.add_systems(Startup, build_bed_assets).add_systems(Update, update_beds);
        app.update();
        app
    }

    fn site(fire: Option<Entity>) -> FlameSite {
        FlameSite {
            color: None,
            height: 0.7,
            phase: 0.4,
            kindle: 0.0,
            diameter: Some(0.8),
            depth: FLICKER_DEPTH,
            depth_fade: false,
            fire,
        }
    }

    /// The discs of `site`: `(entity, drawn heat, transform, visibility)`.
    fn discs(app: &mut App, site: Entity) -> Vec<(Entity, f32, Transform, Visibility)> {
        app.world_mut()
            .query_filtered::<(Entity, &ChildOf, &MeshTag, &Transform, &Visibility), With<Bed>>()
            .iter(app.world())
            .filter(|(_, parent, ..)| parent.parent() == site)
            .map(|(entity, _, tag, at, visibility)| (entity, unpack_bed_tag(tag.value).0, *at, *visibility))
            .collect()
    }

    fn run(app: &mut App, seconds: f32) {
        for _ in 0..(seconds * 60.0).round() as usize {
            app.update();
        }
    }

    #[test]
    fn only_a_site_that_asks_gets_a_disc_and_it_lies_flat_at_the_foot_the_width_of_the_bed() {
        let mut app = app();
        let plain = app.world_mut().spawn((site(None), Transform::default())).id();
        let bedded = app
            .world_mut()
            .spawn((site(None), FuelBed { diameter: 1.4, ..default() }, Transform::default()))
            .id();
        app.update();
        app.update();
        assert!(discs(&mut app, plain).is_empty(), "a site without a bed has no disc");
        let found = discs(&mut app, bedded);
        assert_eq!(found.len(), 1);
        let at = found[0].2;
        assert_eq!(at.scale, Vec3::new(1.4, 1.0, 1.4));
        assert!(
            at.translation.x == 0.0
                && at.translation.z == 0.0
                && at.translation.y > 0.0
                && at.translation.y < 0.05
        );
        assert_eq!(found[0].3, Visibility::Inherited);
    }

    #[test]
    fn a_removed_bed_takes_its_disc_away_and_can_come_back_and_a_changed_diameter_resizes_it() {
        let mut app = app();
        let at = app.world_mut().spawn((site(None), FuelBed::default(), Transform::default())).id();
        app.update();
        app.update();
        assert_eq!(discs(&mut app, at).len(), 1);
        app.world_mut().entity_mut(at).insert(FuelBed { diameter: 2.0, ..default() });
        app.update();
        assert_eq!(discs(&mut app, at).len(), 1, "a changed bed is the same disc");
        assert_eq!(discs(&mut app, at)[0].2.scale.x, 2.0);
        app.world_mut().entity_mut(at).remove::<FuelBed>();
        app.update();
        app.update();
        assert!(discs(&mut app, at).is_empty());
        app.world_mut().entity_mut(at).insert(FuelBed::default());
        app.update();
        app.update();
        assert_eq!(discs(&mut app, at).len(), 1, "a bed given back is dressed again");
    }

    #[test]
    fn a_bed_that_is_not_a_positive_width_draws_nothing_and_never_panics() {
        let mut app = app();
        for diameter in [f32::NAN, 0.0, -1.0, f32::INFINITY] {
            let at = app
                .world_mut()
                .spawn((site(None), FuelBed { diameter, ..default() }, Transform::default()))
                .id();
            app.update();
            app.update();
            let found = discs(&mut app, at);
            assert_eq!(found.len(), 1);
            assert_eq!(found[0].3, Visibility::Hidden, "diameter {diameter}");
        }
    }

    #[test]
    fn a_fire_that_burns_keeps_its_bed_hot_and_a_fire_put_out_cools_it_and_a_relight_warms_it_back() {
        let mut app = app();
        let light = app.world_mut().spawn_empty().id();
        let at = app
            .world_mut()
            .spawn((site(Some(light)), FuelBed { cooling: 10.0, ..default() }, Transform::default()))
            .id();
        run(&mut app, 2.0);
        let burning = discs(&mut app, at)[0].1;
        assert!(burning > 0.9, "a burning fire's bed is hot: {burning}");

        app.world_mut().entity_mut(light).insert(Snuff { elapsed: 1.2 });
        run(&mut app, 1.0);
        let a_second_later = discs(&mut app, at)[0].1;
        assert!(
            a_second_later > 0.8 * burning,
            "a second after the douse the coals are still glowing: {a_second_later}"
        );
        run(&mut app, 10.0);
        let cooled = discs(&mut app, at)[0].1;
        assert!(cooled < 0.5 * burning && cooled > 0.2 * burning, "one time constant on: {cooled}");

        app.world_mut().entity_mut(light).remove::<Snuff>();
        run(&mut app, 10.0);
        let relit = discs(&mut app, at)[0].1;
        assert!(relit > 0.9, "a fire lit again warms its bed back: {relit}");
    }

    #[test]
    fn coals_that_have_gone_dark_are_not_drawn_and_are_again_when_the_fire_is_relit() {
        let mut app = app();
        let light = app.world_mut().spawn(Snuff { elapsed: 1.2 }).id();
        let at = app
            .world_mut()
            .spawn((site(Some(light)), FuelBed { cooling: 2.0, ..default() }, Transform::default()))
            .id();
        run(&mut app, 1.0);
        assert_eq!(
            discs(&mut app, at)[0].3,
            Visibility::Hidden,
            "a bed dressed under a fire already out is cold"
        );
        app.world_mut().entity_mut(light).remove::<Snuff>();
        run(&mut app, 6.0);
        assert_eq!(discs(&mut app, at)[0].3, Visibility::Inherited);
        app.world_mut().entity_mut(light).insert(Snuff { elapsed: 1.2 });
        run(&mut app, 30.0);
        assert_eq!(
            discs(&mut app, at)[0].3,
            Visibility::Hidden,
            "the glow is gone after the cooling has run"
        );
    }

    #[test]
    fn a_fire_still_catching_starts_its_bed_cold_and_a_fire_that_has_burned_starts_it_hot() {
        let mut app = app();
        let catching = app.world_mut().spawn(crate::Kindle::default()).id();
        let settled = app.world_mut().spawn(crate::Kindle { elapsed: 5.0 }).id();
        let new_fire =
            app.world_mut().spawn((site(Some(catching)), FuelBed::default(), Transform::default())).id();
        let old_fire =
            app.world_mut().spawn((site(Some(settled)), FuelBed::default(), Transform::default())).id();
        app.update();
        app.update();
        let cold = discs(&mut app, new_fire)[0].1;
        let hot = discs(&mut app, old_fire)[0].1;
        assert!(cold < 0.2, "{cold}");
        assert!(hot > 0.9, "{hot}");
    }

    #[test]
    fn beds_of_different_seeds_show_different_windows_and_the_flicker_moves_the_heat_a_little() {
        let mut app = app();
        let make = |app: &mut App, seed| {
            app.world_mut().spawn((site(None), FuelBed { seed, ..default() }, Transform::default())).id()
        };
        let (a, b, c) = (make(&mut app, 1), make(&mut app, 2), make(&mut app, 1));
        app.update();
        app.update();
        let offset = |app: &mut App, site: Entity| {
            let (disc, ..) = discs(app, site)[0];
            unpack_bed_tag(app.world().get::<MeshTag>(disc).unwrap().value).1
        };
        assert_ne!(offset(&mut app, a), offset(&mut app, b));
        assert_eq!(offset(&mut app, a), offset(&mut app, c));
        let mut seen = Vec::new();
        for _ in 0..240 {
            app.update();
            seen.push(discs(&mut app, a)[0].1);
        }
        let (low, high) = seen.iter().fold((f32::MAX, f32::MIN), |(l, h), v| (l.min(*v), h.max(*v)));
        assert!(high > low && high - low < 0.1, "the heat breathes by a few per cent: {low}..{high}");
    }

    #[test]
    fn a_fuel_beds_problems_name_what_is_unusable() {
        assert!(FuelBed::default().problems().is_empty());
        assert_eq!(FuelBed { diameter: 0.0, ..default() }.problems().len(), 1);
        assert_eq!(FuelBed { cooling: f32::NAN, ..default() }.problems().len(), 1);
    }
}
