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
    view_transformations::{position_world_to_clip, depth_ndc_to_view_z},
};
@if(DEPTH_PREPASS)
import bevy_pbr::prepass::utils::prepass_depth;

struct Flame {
    // The host's fire colour, and the ember strength in `.a`.
    base: vec4<f32>,
    // x: seconds (wrapped hourly), y: the tag's height scale in world units (`FlameLook::max_height`), z: 1 for the material of a
    // site that asked for the depth fade, w: the ember
    // strength.
    params: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> material: Flame;

// The quad's width over its height: narrower than tall, with a margin for the sway.
const FLAME_ASPECT: f32 = 0.6;
// A catching flame's drawn size is its share of the burn to this power: a flame's height goes as the heat release rate to the 2/3
// while it is small (Delichatsios 2007), and the light goes with the burn itself. `docs/research/flame-catch-size.md`.
const CATCH_SIZE_EXPONENT: f32 = 0.6666667;
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
// The depth fade (`docs/research/flame-occlusion.md`): the source point is a third of the way up the flame, the tap ring's radius
// is this fraction of the flame's height, and a tap occludes when the scene is nearer than the source by more than FADE_MARGIN of
// the flame's height, compared in linear view distance (so a perspective and an orthographic camera at any far plane agree).
const SOURCE_HEIGHT: f32 = 0.35;
const FADE_RADIUS: f32 = 0.10;
const FADE_MARGIN: f32 = 0.15;
// The flame from above (`docs/research/flame-overhead.md`): an upright quad presents `cos(elevation)` of its area, so the flame's own
// edges vanish before 75 degrees. Past OVERHEAD_START (50 degrees) the quad turns to face the camera, through the plume's centre
// (so no part of it is left inside the brazier's bowl or below its floor), the upright flame is drawn on it as the image it has on
// screen (a world height `h` lands `h * cos(elevation)` up), and a cross-section of the plume fades in between there and
// OVERHEAD_FULL (76 degrees). At or below OVERHEAD_START the quad is the upright one and every term is exactly its old value.
const OVERHEAD_START: f32 = 0.873;
const OVERHEAD_FULL: f32 = 1.326;
// The camera-facing quad's height, in flame heights: the tongue, the cross-section's ellipse and the rim's ripple all fit in it.
const OVERHEAD_HEIGHT: f32 = 1.0;
// The floor under cos(elevation) when a screen distance is turned back into a world height.
// How much of the tongue's brightness is gone when the camera is straight overhead: design default, judged in captures.
const OVERHEAD_TONGUE: f32 = 0.6;
const OVERHEAD_COS_FLOOR: f32 = 0.05;
// From above the fire is seen against ground it has lit itself, so its core must out-shine that ground: extra emission in the hot core as the
// camera rises (`docs/research/flame-overhead.md`, round 13).
const OVERHEAD_CORE: f32 = 0.9;
// The squash floor of the cross-section's ellipse: sin(10 degrees), so a camera nearly level does not flatten it to a line.
const OVERHEAD_SQUASH: f32 = 0.18;
// The outward flow of the overhead disc's brightness: cycles a second, and how much each layer's features grow over a cycle.
// Design defaults, judged in captures (round 18).
const OVERHEAD_FLOW_RATE: f32 = 1.2;
const OVERHEAD_FLOW_GROWTH: f32 = 1.0;

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
    // Where the fire's source is on the screen, for the depth fade (`FlameSite::depth_fade`): `.xy` the pixel, `.z` its linear view z
    // (negative in front of the camera), `.w` the tap ring's radius in pixels. `.w == 0` is "do not test": a flame that did not
    // ask, or whose source is behind the camera.
    @location(4) @interpolate(flat) fade: vec4<f32>,
    // The depth fade's margin in view-space units: the share of the flame's drawn height a nearer surface must clear to hide the source.
    @location(5) @interpolate(flat) fade_margin: f32,
    // The flame from above: `.x` how far the camera is over the fire (0 at 50 degrees and below, where the quad is upright, 1 at 76 and
    // above), `.y` the cosine and `.z` the sine of the elevation. Flat: one camera, one flame.
    @location(6) @interpolate(flat) overhead: vec3<f32>,
    // The wind's lean: `.x` the flame axis's sideways run per unit of its length (signed, along the camera's right), `.y` the quad's
    // width in flame heights (`FLAME_ASPECT` widened by the lean), `.z` how high the axis still stands (`cos` of the tilt). Flat.
    @location(7) @interpolate(flat) wind: vec3<f32>,
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

// ---- The depth fade: a flame whose source cannot be seen goes out (`FlameSite::depth_fade`) --------------------------------

// The pixel a world point lands on, in the same framebuffer coordinates `@builtin(position)` uses.
fn pixel_of(ndc: vec2<f32>) -> vec2<f32> {
    return view.viewport.xy + vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5) * view.viewport.zw;
}

// The fire's source on screen: its pixel and linear view z, and the radius of the ring of taps around it, from the flame's own drawn
// size so a far flame reads a small ring and a near one a large one. `.w == 0` means no test: the material did not ask for it (the one
// uniform branch an ordinary flame pays), or a point is behind the camera.
fn fade_probe(origin: vec3<f32>, right: vec3<f32>, size: f32) -> vec4<f32> {
    if material.params.z < 0.5 {
        return vec4<f32>(0.0);
    }
    let source = origin + vec3<f32>(0.0, 1.0, 0.0) * (size * SOURCE_HEIGHT);
    let c0 = position_world_to_clip(source);
    let c1 = position_world_to_clip(source + right * (size * FADE_RADIUS));
    if c0.w <= 0.0 || c1.w <= 0.0 {
        return vec4<f32>(0.0);
    }
    let n0 = c0.xyz / c0.w;
    let p0 = pixel_of(n0.xy);
    let p1 = pixel_of(c1.xy / c1.w);
    return vec4<f32>(p0, depth_ndc_to_view_z(n0.z), clamp(distance(p0, p1), 2.0, 24.0));
}

@if(DEPTH_PREPASS)
// 1 when the tap at `pixel` does not hide the source, 0 when the scene there is nearer than the source by more than `margin` (view
// units). Linear view z is negative forward, so nearer is the larger value. A tap off the screen sees nothing in front of it.
fn tap_clear(pixel: vec2<f32>, source_z: f32, margin: f32) -> f32 {
    let low = view.viewport.xy;
    let high = view.viewport.xy + view.viewport.zw;
    if pixel.x < low.x || pixel.y < low.y || pixel.x >= high.x || pixel.y >= high.y {
        return 1.0;
    }
    let scene_z = depth_ndc_to_view_z(prepass_depth(vec4<f32>(pixel, 0.0, 1.0), 0u));
    return select(1.0, 0.0, scene_z > source_z + margin);
}

// The share of the source the camera can see: sixteen taps on a Vogel disc (radius sqrt((i + 0.5) / 16), turned by the golden angle
// each step) of the ring's radius around the source's pixel, smoothed. The taps' offsets along x are all different, so a vertical wall
// edge crossing the source flips them one at a time and takes the flame down in sixteenths instead of in lumps.
@if(DEPTH_PREPASS)
fn source_visibility(fade: vec4<f32>, margin: f32) -> f32 {
    if fade.w <= 0.0 {
        return 1.0;
    }
    let c = fade.xy;
    let r = fade.w;
    let d = fade.z;
    var seen = tap_clear(c + vec2<f32>(0.17678, 0.00000) * r, d, margin);
    seen += tap_clear(c + vec2<f32>(-0.22577, 0.20683) * r, d, margin);
    seen += tap_clear(c + vec2<f32>(0.03456, -0.39377) * r, d, margin);
    seen += tap_clear(c + vec2<f32>(0.28457, 0.37117) * r, d, margin);
    seen += tap_clear(c + vec2<f32>(-0.52222, -0.09237) * r, d, margin);
    seen += tap_clear(c + vec2<f32>(0.49470, -0.31468) * r, d, margin);
    seen += tap_clear(c + vec2<f32>(-0.16547, 0.61553) * r, d, margin);
    seen += tap_clear(c + vec2<f32>(-0.31556, -0.60759) * r, d, margin);
    seen += tap_clear(c + vec2<f32>(0.68464, 0.25003) * r, d, margin);
    seen += tap_clear(c + vec2<f32>(-0.71226, 0.29401) * r, d, margin);
    seen += tap_clear(c + vec2<f32>(0.34335, -0.73373) * r, d, margin);
    seen += tap_clear(c + vec2<f32>(0.25373, 0.80893) * r, d, margin);
    seen += tap_clear(c + vec2<f32>(-0.76475, -0.44319) * r, d, margin);
    seen += tap_clear(c + vec2<f32>(0.89713, -0.19723) * r, d, margin);
    seen += tap_clear(c + vec2<f32>(-0.54751, 0.77877) * r, d, margin);
    seen += tap_clear(c + vec2<f32>(-0.12649, -0.97609) * r, d, margin);
    return smoothstep(0.0, 1.0, seen / 16.0);
}

@if(!DEPTH_PREPASS)
// A camera with no `DepthPrepass` has no depth to read: the fade is not compiled in and every flame is whole.
fn source_visibility(fade: vec4<f32>, margin: f32) -> f32 {
    return 1.0;
}

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
    // The flame's drawn size carries its fire's catch: a flame lit with its light grows from nothing
    // over the same envelope the light flares by (`kindle_gain`), so the source and the light it
    // motivates are one event in two channels. `took` is the instance scale `update_flames` writes,
    // read above; multiplying it in here is the whole difference between a documented catch and a
    // flame that appears whole. `docs/research/flame-kindle.md` measures it; the 2/3 power is the height law
    // in `docs/research/flame-catch-size.md`, so the flame is as far along as the light it motivates.
    let size = tag.y * material.params.y * pow(max(took, 0.0001), CATCH_SIZE_EXPONENT);
    // The camera's elevation over the flame's source: 0 level, 1.57 straight down.
    let centre = origin + up * (size * SOURCE_HEIGHT);
    let to_eye = view.world_from_view[3].xyz - centre;
    let sin_e = clamp(to_eye.y / max(length(to_eye), 1e-4), 0.0, 1.0);
    let elevation = asin(sin_e);
    let topdown = smoothstep(OVERHEAD_START, OVERHEAD_FULL, elevation);
    // The wind's lean rides on the instance scale's other two lengths (`1 + sin θ · direction`, world X and Z), which a parent's
    // rotation cannot change: the flame's axis leans from vertical by θ along that direction. Projected on the camera's right it is
    // how far the tip sits to the side (in flame lengths, signed), and `cos θ` is how high it still stands. From straight overhead
    // the plume's cross-section is what is seen and it is not translated, so the lean fades out with the overhead blend.
    let wind_world = (vec2<f32>(length(world_from_local[1].xyz), length(world_from_local[2].xyz)) - vec2<f32>(1.0)) * (1.0 - topdown);
    let sin_tilt = min(length(wind_world), 0.995);
    let lean = dot(wind_world, vec2<f32>(right.x, right.z));
    let upright = sqrt(1.0 - sin_tilt * sin_tilt);
    let width = FLAME_ASPECT + abs(lean);
    let across = vertex.position.x * width + 0.5 * lean;
    var billboard = origin + right * (across * size) + up * (vertex.position.y * size + size * 0.5);
    if sin_e > sin(OVERHEAD_START) {
        // Facing the camera: screen up is what the view direction and the horizontal right leave.
        let screen_up = normalize(cross(normalize(to_eye), right));
        billboard = centre + right * (across * size) + screen_up * (vertex.position.y * size * OVERHEAD_HEIGHT);
    }
    out.wind = vec3<f32>(lean, width, upright);
    out.overhead = vec3<f32>(topdown, sqrt(max(1.0 - sin_e * sin_e, 0.0)), sin_e);
    out.position = position_world_to_clip(billboard);
    out.uv = vec2<f32>(vertex.position.x, vertex.position.y) + vec2<f32>(0.5, 0.5);
    out.phase = f32((raw >> 8u) & 0xffu) / 255.0 * TAU;
    out.took = took;
    out.brightness = f32(raw & 0xffu) / 255.0;
    out.fade = fade_probe(origin, right, size);
    out.fade_margin = size * FADE_MARGIN;
    return out;
}

@fragment
fn fragment(in: FlameVertexOutput) -> @location(0) vec4<f32> {
    let t = material.params.x + in.phase;
    // Height up the flame from its foot (a 4% margin under it) and the distance across in flame heights, so the noise is round. Upright,
    // the quad's height is the flame's; facing the camera, a screen height `sy` above the plume's centre is `sy / cos(elevation)` of
    // world height above it.
    let facing = in.overhead.z > sin(OVERHEAD_START);
    let sy = (in.uv.y - 0.5) * OVERHEAD_HEIGHT;
    let rise_screen = select(in.uv.y, SOURCE_HEIGHT + sy / max(in.overhead.y, OVERHEAD_COS_FLOOR), facing);
    // The wind's lean (`docs/research/flame-wind.md`): the flame is drawn as the image of a flame leaning downwind, which on screen
    // is its axis `(lean, cos θ)`. The quad was widened and shifted downwind in the vertex stage, so the screen point is read back
    // into the flame's own frame: `rise` along that axis (its foreshortened length undone) and `x` across it. With no wind the axis is
    // `(0, 1)`, and this is the plain `rise = rise_screen`, `x = (uv.x - 0.5) * FLAME_ASPECT`, to the bit.
    let axis_length = max(length(vec2<f32>(in.wind.x, in.wind.z)), 1e-4);
    let axis = vec2<f32>(in.wind.x, in.wind.z) / axis_length;
    let screen_point = vec2<f32>((in.uv.x - 0.5) * in.wind.y + 0.5 * in.wind.x, rise_screen);
    let rise = dot(screen_point, axis) / axis_length;
    let y = (rise - 0.04) / 0.96;
    let x = screen_point.x * axis.y - screen_point.y * axis.x;
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
    // From above the tongue is a short smear along the view and the cross-section is the shape, so the tongue gives way as the disc
    // arrives: its foot is softened (a hard line across the coal bed is the tongue's seat) and it is dimmed by OVERHEAD_TONGUE, as the
    // fifth power of the blend so a flame still in view at 60-70 degrees keeps its brightness until the disc has taken over.
    // Straight down the upright flame is edge-on: a one-pixel slab of its widest row that pokes past the disc. It is gone by 85 degrees.
    let tongue = (1.0 - OVERHEAD_TONGUE * pow(in.overhead.x, 5.0)) * (1.0 - smoothstep(0.970, 0.996, in.overhead.z));
    let foot = smoothstep(0.0, 0.06 + 0.30 * in.overhead.x, y);
    var mask = smoothstep(0.0, 0.30, body) * foot * tongue;
    // Hottest along the axis and low down: red rim and tip, the host's fire, a yellow core.
    // The tongue's heat leaves with its foot from above: a heat that stopped dead at the seat stepped the colour across the disc it overlaps.
    var heat = clamp(body, 0.0, 1.0) * (1.0 - 0.85 * u) * mix(1.0, foot, in.overhead.x);
    // From above the plume's cross-section is what is seen: a horizontal disc of the plume's widest radius, rippled by the same
    // turbulence, drawn as the ellipse it projects to (`R` wide, `R * sin(elevation)` tall) and faded in with the camera's rise.
    if in.overhead.x > 0.0 {
        let q = vec2<f32>(x, sy / max(in.overhead.z, OVERHEAD_SQUASH));
        // Lobed: the radius is a noise of the direction from the axis (periodic in angle, no seam), scrolling in time, so the outline
        // has tongues that lean and part (`docs/research/flame-overhead.md`, round 7). The brightness is billowy, a noise of the
        // position, not of the direction: a direction-only noise draws rays out of the centre (round 15).
        let rq = length(q);
        let dir = q / max(rq, 1e-4);
        let lobes = fbm(dir * 0.9 + vec2<f32>(t * 0.6, 3.7));
        let radius = FLAME_HALF_WIDTH * (1.2 + 0.8 * (lobes - 0.5));
        let disc = 1.0 - rq / radius;
        // The brightness flows outward from the axis, the way a flame's billows swell as they rise towards a camera above them: two
        // layers sampled at a scale that shrinks over a cycle (so their features grow about the centre), half a cycle apart and
        // cross-faded, each fading as its scale is reset (round 18). A noise dragged one way across the disc read as a wind.
        let cycle = fract(t * OVERHEAD_FLOW_RATE);
        let cycle_b = fract(cycle + 0.5);
        let weight_b = abs(2.0 * cycle - 1.0);
        let weight_a = 1.0 - weight_b;
        let layer_a = fbm(q * 8.0 / (1.0 + OVERHEAD_FLOW_GROWTH * cycle) + vec2<f32>(5.1, 1.3));
        let layer_b = fbm(q * 8.0 / (1.0 + OVERHEAD_FLOW_GROWTH * cycle_b) + vec2<f32>(-2.7, 8.4));
        let streak = clamp(0.5 + (weight_a * (layer_a - 0.5) + weight_b * (layer_b - 0.5)) * inverseSqrt(weight_a * weight_a + weight_b * weight_b), 0.0, 1.0);
        let licks = mix(1.0, clamp(0.3 + 1.4 * streak, 0.0, 1.3), smoothstep(0.0, 0.35, rq / radius));
        mask = max(mask, smoothstep(0.0, 0.18, disc) * licks * in.overhead.x);
        // The core is a round heat, only its outline is lobed: a heat that followed the lobes drew rays out of the centre.
        let core_radius = 0.5 * (radius + FLAME_HALF_WIDTH);
        heat = max(heat, clamp(1.0 - rq / core_radius, 0.0, 1.0) * (1.0 - 0.45 * rq / core_radius) * mix(0.75, 1.1, streak) * in.overhead.x);
        // The flame's foot on the bed: a second, smaller billow where the plume stands on its coals, found by the same screen offset
        // the tongue's seat uses. Without it the cloud hovers half a flame above a bed that is lit and crisp below it (round 17).
        let qf = vec2<f32>(x, (sy + (SOURCE_HEIGHT - 0.04) * in.overhead.y) / max(in.overhead.z, OVERHEAD_SQUASH));
        let rf = length(qf);
        let foot_radius = FLAME_HALF_WIDTH * (0.80 + 0.5 * (fbm(qf / max(rf, 1e-4) * 0.9 + vec2<f32>(t * 0.5, 9.1)) - 0.5));
        let foot_disc = 1.0 - rf / foot_radius;
        let foot_streak = fbm(qf * 8.0 + vec2<f32>(2.3, -t * 1.3));
        let foot_mask = smoothstep(0.0, 0.42, foot_disc) * clamp(0.45 + 1.0 * foot_streak, 0.0, 1.1) * in.overhead.x;
        mask = max(mask, 0.85 * foot_mask);
        heat = max(heat, clamp(foot_disc * 1.4, 0.0, 1.0) * mix(0.35, 0.65, foot_streak) * in.overhead.x);
    }
    var colour = mix(EDGE_RED, material.base.rgb, smoothstep(0.05, 0.40, heat));
    colour = mix(colour, CORE_YELLOW, smoothstep(0.40, 0.80, heat));
    colour *= 1.0 + OVERHEAD_CORE * in.overhead.x * smoothstep(0.30, 0.80, heat);
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
    return vec4<f32>(colour * mask * material.params.w * in.brightness * source_visibility(in.fade, in.fade_margin), 0.0);
}
