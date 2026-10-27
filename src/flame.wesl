// The flame of a fire (`src/flame.rs`, `FlameMaterial`): a billboarded 2D flame, so a torch or a brazier burns
// rather than glows. A host draws one on each `FlameSite` it spawns (Sanctuary report 1792016494: "Torches and
// braziers need a shader flame of some sort. Right now they just glow.").
//
// The shape is ONE flame: a single teardrop silhouette whose coordinates are distorted by scrolling fbm noise, with an
// amplitude that grows with height, so the head is steady and the tip licks. This is the "2D distortion" flame that
// Nguyen, "Fire in the Vulcan Demo", GPU Gems (2004) ch. 6, section 6.1, calls "perfect for creating candle-like
// flames"; the octave sum is the turbulence function of Akenine-Moller et al., Real-Time Rendering, 4th ed.,
// section 6.3. (The first version tiled the quad into five columns, a row of tongues: decision 0228.)
//
// The noise is a 2D simplex noise with the classic fbm over it; both are web sources home-still cannot index, so their
// provenance is UNVERIFIED. Adapted here: the quad is billboarded by the vertex stage and sized in world units; the flame's
// clock is the material's seconds (one shared material, written once a frame) plus the fire's own phase, and its
// height is the fire's own too: both ride on the mesh's `MeshTag` (`get_tag`, packed by `pack_flame_tag`), so two fires are
// out of step and a torch's flame is smaller than a brazier's; the gradient hash is an integer hash of the lattice point
// instead of the `fract(sin)` hash; and the host's fire colour tints the ramp so every scene's fire matches its
// own light.
//
// Cheap by design: a billboard, four octaves of 2D noise (three fields) and no march, so a street of braziers costs
// almost nothing. Additive blending, no depth write, no prepass.

import bevy_pbr::render::{
    forward_io::{Vertex, decompress_vertex},
    mesh_functions,
    mesh_view_bindings::view,
    view_transformations::position_world_to_clip,
};

struct Flame {
    // The host's fire colour, and the ember strength in `.a`.
    base: vec4<f32>,
    // x: seconds (wrapped hourly), y: the tag's height scale in world units (`FlameLook::max_height`), z: unused, w: the ember
    // strength.
    params: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> material: Flame;

// The quad's width over its height: narrower than tall, with a margin for the sway.
const FLAME_ASPECT: f32 = 0.6;
// The same ratio when the flame is seen down its own axis: wide enough to hold the disc's diameter, `4 * FLAME_HALF_WIDTH`
// of the flame's height, which is 0.8. A margin over it, so the turbulence that ripples the disc's edge is not clipped.
const FLAME_OVERHEAD_ASPECT: f32 = 0.9;
const TAU: f32 = 6.2831853;
const PI: f32 = 3.14159265;
// The silhouette: its widest half width (in flame heights), where its tip is, and the profile's own peak (so the widest
// point is FLAME_HALF_WIDTH).
const FLAME_HALF_WIDTH: f32 = 0.20;
const FLAME_TIP: f32 = 0.90;
const PROFILE_PEAK: f32 = 1.172;
// Where the upright flame turns into its own cross-section: the camera's elevation over the fire, in radians. Below
// `OVERHEAD_START` the flame is the upright tongue exactly as it was, so no ground-level view changes at all; above
// `OVERHEAD_FULL` it is the round puff, and the turn between them is smooth so a camera rising does not pop.
// 50 deg is where a third-person camera's own default sits, still upright; 76 deg is steep enough that the upright quad
// has lost four fifths of its area. Both are design defaults, calibrated in captures
// (`docs/research/flame-overhead.md`).
const OVERHEAD_START: f32 = 0.873;
const OVERHEAD_FULL: f32 = 1.326;
// The ramp: the cool rim and tip, then the host's fire colour, then the hot core.
const EDGE_RED: vec3<f32> = vec3<f32>(0.70, 0.06, 0.005);
const CORE_YELLOW: vec3<f32> = vec3<f32>(1.0, 0.78, 0.30);
// What a flame's colour tends to while its fire is still catching. A flame fed on volatiles with little soot is
// cool and blue; a young, small fire is redder and dimmer than the steady one, and the hot yellow core only arrives
// once the fuel is alight. Planck's law is why this is a *temperature*: cooler emission is redder (the crate's
// `blackbody_colour`), so the catch pulls the ramp towards the red end and lets it go as the fire takes.
const EMBER_RED: vec3<f32> = vec3<f32>(0.95, 0.20, 0.02);

// The flame's own stage IO: only the quad's uv and the fire's phase cross to the fragment.
struct FlameVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) phase: f32,
    // How far this flame's own fire has caught: its instance scale, which `update_flames` writes from the
    // fire's `kindle_gain`. The vertex stage multiplies it into the flame's drawn size, so a flame lit with
    // its light grows from nothing with it; the fragment mixes the ember colour in by the same number. Flat,
    // because it is one number for the whole flame.
    @location(2) @interpolate(flat) took: f32,
    // The fire's flicker this frame (0.70..1.00 of its base), read from the tag: the same number the light's intensity
    // was multiplied by, so the drawn fire and the light it casts are one fire. Flat, for the same reason as `took`.
    @location(3) @interpolate(flat) brightness: f32,
    // How far the camera is above the fire, 0 from the side to 1 overhead: the flame is drawn upright at 0 and as its
    // own cross-section at 1. Flat, because it is one number for the whole flame.
    @location(4) @interpolate(flat) cross: f32,
    // `sin` of the camera's elevation over this flame: the vertical squash of the horizontal disc it sees. Clamped to a
    // floor so the ellipse never becomes a line, and flat because it is one number for the whole flame.
    @location(5) @interpolate(flat) squash: f32,
}

// ---- 2D simplex noise and the classic fbm over it (provenance UNVERIFIED, see header) ---------------------------------

// The gradient at a lattice point: an integer hash of the point's coordinates, so its quality does not fall off
// as the clock grows, unlike `fract(sin(x) * 43758.5)`. Jarzynski & Olano (2020, JCGT, "Hash Functions for GPU
// Rendering") study multidimensional GPU hashes; home-still returned only the abstract, so whether this is their
// hash, and its constants, are UNVERIFIED.
fn hash(p: vec2<f32>) -> vec2<f32> {
    var v = bitcast<vec2<u32>>(vec2<i32>(floor(p))) * 1664525u + 1013904223u;
    v.x += v.y * 1664525u;
    v.y += v.x * 1664525u;
    v = v ^ (v >> vec2<u32>(16u));
    v.x += v.y * 1664525u;
    v.y += v.x * 1664525u;
    v = v ^ (v >> vec2<u32>(16u));
    return -1.0 + 2.0 * vec2<f32>(v) / 4294967295.0;
}

fn noise(p: vec2<f32>) -> f32 {
    let k1 = 0.366025404;
    let k2 = 0.211324865;
    let i = floor(p + (p.x + p.y) * k1);
    let a = p - i + (i.x + i.y) * k2;
    let o = select(vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), a.x > a.y);
    let b = a - o + k2;
    let c = a - 1.0 + 2.0 * k2;
    let h = max(0.5 - vec3<f32>(dot(a, a), dot(b, b), dot(c, c)), vec3<f32>(0.0));
    let n = h * h * h * h * vec3<f32>(dot(a, hash(i + 0.0)), dot(b, hash(i + o)), dot(c, hash(i + 1.0)));
    return dot(n, vec3<f32>(70.0));
}

fn fbm(uv_in: vec2<f32>) -> f32 {
    var uv = uv_in;
    var f = 0.0;
    let m = mat2x2<f32>(1.6, 1.2, -1.2, 1.6);
    f = 0.5000 * noise(uv);
    uv = m * uv;
    f += 0.2500 * noise(uv);
    uv = m * uv;
    f += 0.1250 * noise(uv);
    uv = m * uv;
    f += 0.0625 * noise(uv);
    return 0.5 + 0.5 * f;
}

// -----------------------------------------------------------------------------------------------------------------------

@vertex
fn vertex(vertex_in: Vertex) -> FlameVertexOutput {
    var out: FlameVertexOutput;
    let vertex = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);

    // Billboard: the quad's own X follows the camera's right and its Y stays the world's up, so the flame stands upright
    // and always faces the orthographic camera. The quad is a unit square in local XY about its origin; its height in
    // units is the tag's (a fraction of `params.y`), and its base is lifted to its own origin. Its width is a
    // fraction of that, `FLAME_ASPECT`.
    let origin = world_from_local[3].xyz;
    // ---- The overhead cross-section -------------------------------------------------------------------------------
    // An upright quad is an *axial* billboard (Akenine-Moller et al., Real-Time Rendering 4th ed., section 13.6.3): its
    // normal is horizontal, so the area it presents to the camera is `cos(elevation)` of its face and it collapses to a
    // line as the camera goes overhead. The reference names both the failure and the fix — "if the viewer flies over
    // the trees and looks down, the illusion is ruined, as the trees will appear nearly edge-on and look like the
    // cutouts they are. One workaround is to add a horizontal cross section", and section 13.6.2's note that "overhead
    // each type of billboard, the world-oriented version will fully face it, while the axial version will be more
    // affixed to the scene". A flame is *axisymmetric* (Xia & Zhang 2018 assume "laminar axisymmetric incompressible
    // flows without swirling"), so the honest cross-section is the widest part of the plume seen down its axis.
    //
    // The quad itself always stays **upright and camera-facing**; only the silhouette inside it changes. Tilting the
    // quad's own axis toward horizontal, which this change tried first, draws the tongue on a plate leaning away from
    // the camera — strictly worse than either the upright flame or the disc, measured at 60 deg in
    // `docs/research/flame-overhead.md`. A horizontal disc of radius `R` seen from elevation `e` projects to an ellipse
    // `R` wide and `R·sin(e)` tall, so the disc is drawn *inside* the upright quad as that squashed ellipse, which is
    // exactly what the camera would see.
    let to_cam = normalize(view.world_position - origin);
    let elevation = asin(clamp(to_cam.y, -1.0, 1.0));
    let cross = smoothstep(OVERHEAD_START, OVERHEAD_FULL, elevation);
    // The quad's width turns from the standing tongue's narrow face to one wide enough to hold the disc's diameter,
    // which is `4 * FLAME_HALF_WIDTH` of the flame's height in world units. Its height never changes.
    let quad_x = mix(FLAME_ASPECT, FLAME_OVERHEAD_ASPECT, cross);
    // The flame's catch rides on its instance transform (its scale, 0..1), so each flame's colour is its own
    // fire's — one shared material cannot hold a per-flame number, and the transform can.
    let took = clamp(length(world_from_local[0].xyz), 0.0, 1.0);
    let right = normalize(vec3<f32>(view.world_from_view[0].x, 0.0, view.world_from_view[0].z));
    // The flame's `MeshTag` (`pack_flame_tag` in `src/model/tag.rs`): height in the high 16 bits, unsigned-normalised,
    // which `unpack2x16unorm` reads back as `.y`; the clock phase in the next byte and the fire's own brightness in the
    // low byte, both plain 0..255 shares. The brightness is the fire's flicker, written every frame from the same
    // function the light's intensity is, so the flame breathes with the ground under it.
    let raw = mesh_functions::get_tag(vertex_in.instance_index);
    let tag = unpack2x16unorm(raw);
    // The flame's drawn size carries its fire's catch: a flame lit with its light grows from nothing
    // over the same envelope the light flares by (`kindle_gain`), so the source and the light it
    // motivates are one event in two channels. `took` is the instance scale `update_flames` writes,
    // read above; multiplying it in here is the whole difference between a documented catch and a
    // flame that appears whole. `docs/research/flame-kindle.md` measures it.
    let size = tag.y * material.params.y * took;
    // The quad stands on the fire's own origin, lifted by half its own height so its foot is on the coals. Nothing here
    // depends on the camera: the silhouette inside it does, in the fragment stage.
    let billboard = origin + right * (vertex.position.x * size * quad_x)
        + vec3<f32>(0.0, 1.0, 0.0) * (vertex.position.y * size + size * 0.5);
    out.position = position_world_to_clip(billboard);
    out.uv = vec2<f32>(vertex.position.x, vertex.position.y) + vec2<f32>(0.5, 0.5);
    out.phase = f32((raw >> 8u) & 0xffu) / 255.0 * TAU;
    out.took = took;
    out.brightness = f32(raw & 0xffu) / 255.0;
    out.cross = cross;
    out.squash = max(sin(elevation), 0.18);
    return out;
}

@fragment
fn fragment(in: FlameVertexOutput) -> @location(0) vec4<f32> {
    let t = material.params.x + in.phase;
    // Height up the flame from its foot (a 4% margin under it) and the distance across in flame heights, so the noise is round.
    let y = (in.uv.y - 0.04) / 0.96;
    let x = (in.uv.x - 0.5) * FLAME_ASPECT;
    let s = clamp(y, 0.0, 1.0);
    // Rising turbulence: fbm fields scrolled down the flame, so their features climb it.
    let n1 = fbm(vec2<f32>(x * 2.5, y * 1.8 - t * 1.7));
    let n2 = fbm(vec2<f32>(x * 2.5 + 7.3, y * 1.8 - t * 1.7 + 3.1));
    let n3 = fbm(vec2<f32>(x * 4.0 + 2.1, y * 1.6 - t * 2.4));
    // The flame leans and its distortion grows with height: steady at the head, licking at the tip.
    let lean = 0.04 * sin(t * 1.7) + 0.02 * sin(t * 3.1 + 1.0);
    let dx = x + (n1 - 0.5) * 0.30 * s + lean * s * s;
    let dy = y + (n2 - 0.5) * 0.12 * s;
    // The silhouette: a quarter as wide as its widest where it leaves the head, widest about a third up, tapering to the tip.
    let u = clamp(dy / FLAME_TIP, 0.0, 1.0);
    // `max(sin(..), 0.0)`: `sin(PI)` is slightly negative in f32, and `pow` of a negative base is undefined in WGSL.
    let profile = (0.25 * (1.0 - u) + pow(max(sin(PI * pow(u, 0.62)), 0.0), 0.9)) / PROFILE_PEAK;
    let half_width = FLAME_HALF_WIDTH * profile + 1e-4;
    var body = 1.0 - abs(dx) / half_width;
    // Noise eats into the upper flame, so its top licks instead of ending in a smooth point.
    body -= (1.0 - n3) * 0.9 * pow(u, 1.8);
    body = select(body, -1.0, dy < 0.0);
    // ---- Seen down its own axis ------------------------------------------------------------------------------------
    // A flame is axisymmetric, so seen down its axis it is not a teardrop: it is the round widest cross-section of the
    // plume, `2 * FLAME_HALF_WIDTH` of its height across, with the same turbulence rippling its edge and the same hot
    // core in the middle. That disc is horizontal, so the camera sees it as an ellipse `R` wide and `R·sin(elevation)`
    // tall; `body_round` is that ellipse, drawn in the quad's own upright face, and `in.cross` mixes it in as the camera
    // rises so the two silhouettes are one continuous turn rather than two shapes swapped at a threshold.
    let radius_x = FLAME_HALF_WIDTH * (1.0 + (n1 - 0.5) * 0.5) + 1e-4;
    // The disc's radius is the plume's half-width in world units, and the quad's half-width is `quad_x/2` of the flame's
    // height while its half-height is `1/2`. So the disc's **semi-axis** in the quad's uv is `radius/quad_x` across and
    // `radius·squash` up, and `radial` is 1 exactly on the ellipse. Derived from the values the vertex stage used, so the
    // two stages cannot disagree.
    let quad_x = mix(FLAME_ASPECT, FLAME_OVERHEAD_ASPECT, in.cross);
    let radial = length(vec2<f32>((in.uv.x - 0.5) * quad_x / radius_x,
        (in.uv.y - 0.5) / (radius_x * in.squash)));
    let body_round = 1.0 - radial;
    let body_final = mix(body, body_round - (1.0 - n3) * 0.4, in.cross);
    // The base band seats the upright flame's foot on its fire. A cross-section has no foot, so it fades out as the
    // silhouette turns; leaving it on would erase the whole disc, because every fragment of it is at the base's height.
    let seat = mix(smoothstep(0.0, 0.06, y), 1.0, in.cross);
    let mask = smoothstep(0.0, 0.30, body_final) * seat;
    // Hottest along the axis and low down from the side; hottest in the middle from above, where `u` means nothing.
    let heat_side = clamp(body, 0.0, 1.0) * (1.0 - 0.85 * u);
    let heat_round = clamp(body_round, 0.0, 1.0) * (1.0 - 0.45 * clamp(radial, 0.0, 1.0));
    let heat = mix(heat_side, heat_round, in.cross);
    var colour = mix(EDGE_RED, material.base.rgb, smoothstep(0.05, 0.40, heat));
    colour = mix(colour, CORE_YELLOW, smoothstep(0.40, 0.80, heat));
    // A fire that is still catching is redder, and each flame knows its own progress (`in.took`): the steady ramp
    // is pulled towards the ember at the start and released as the fuel catches. Per flame, not per scene, so a
    // host lighting one torch does not tint every other fire in the world. The whole flame goes, not only its core:
    // a catching fire has no yellow core at all yet.
    colour = mix(EMBER_RED * max(heat, 0.35), colour, in.took);
    // Alpha 0 makes `AlphaMode::Add`'s blend state (`PREMULTIPLIED_ALPHA_BLENDING`, src + (1 - src.a) * dst) purely
    // additive: the same trick as `premultiply_alpha` in `bevy_pbr`'s `pbr_functions.wesl`. Any alpha above 0 would
    // darken what stands behind the flame.
    // The fire's own flicker scales the emission as it scales the light: a flame that held still while its pool moved was
    // a source that did not motivate its own light. `docs/research/flame-flicker.md` measures it.
    return vec4<f32>(colour * mask * material.params.w * in.brightness, 0.0);
}
