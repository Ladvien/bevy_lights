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
const TAU: f32 = 6.2831853;
const PI: f32 = 3.14159265;
// The silhouette: its widest half width (in flame heights), where its tip is, and the profile's own peak (so the widest
// point is FLAME_HALF_WIDTH).
const FLAME_HALF_WIDTH: f32 = 0.20;
const FLAME_TIP: f32 = 0.90;
const PROFILE_PEAK: f32 = 1.172;
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
    // fire's `kindle_gain`. Flat, because it is one number for the whole flame.
    @location(2) @interpolate(flat) took: f32,
    // The fire's flicker this frame (0.70..1.00 of its base), read from the tag: the same number the light's intensity
    // was multiplied by, so the drawn fire and the light it casts are one fire. Flat, for the same reason as `took`.
    @location(3) @interpolate(flat) brightness: f32,
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
    // The flame's catch rides on its instance transform (its scale, 0..1), so each flame's colour is its own
    // fire's — one shared material cannot hold a per-flame number, and the transform can.
    let took = clamp(length(world_from_local[0].xyz), 0.0, 1.0);
    let right = normalize(vec3<f32>(view.world_from_view[0].x, 0.0, view.world_from_view[0].z));
    let up = vec3<f32>(0.0, 1.0, 0.0);
    // The flame's `MeshTag` (`pack_flame_tag` in `src/model/tag.rs`): height in the high 16 bits, unsigned-normalised,
    // which `unpack2x16unorm` reads back as `.y`; the clock phase in the next byte and the fire's own brightness in the
    // low byte, both plain 0..255 shares. The brightness is the fire's flicker, written every frame from the same
    // function the light's intensity is, so the flame breathes with the ground under it.
    let raw = mesh_functions::get_tag(vertex_in.instance_index);
    let tag = unpack2x16unorm(raw);
    let size = tag.y * material.params.y;
    let billboard = origin + right * (vertex.position.x * size * FLAME_ASPECT) + up * (vertex.position.y * size + size * 0.5);
    out.position = position_world_to_clip(billboard);
    out.uv = vec2<f32>(vertex.position.x, vertex.position.y) + vec2<f32>(0.5, 0.5);
    out.phase = f32((raw >> 8u) & 0xffu) / 255.0 * TAU;
    out.took = took;
    out.brightness = f32(raw & 0xffu) / 255.0;
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
    let mask = smoothstep(0.0, 0.30, body) * smoothstep(0.0, 0.06, y);
    // Hottest along the axis and low down: red rim and tip, the host's fire, a yellow core.
    let heat = clamp(body, 0.0, 1.0) * (1.0 - 0.85 * u);
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
