// The embers of a fire (`src/embers.rs`, `FlameEmbers`): sparks that leave a flame, climb the plume and cool as they go. Drawn on ONE
// camera-facing quad per flame, as a function of the clock alone (no particle state, no buffers, no simulation): every spark's start,
// lane, speed and life are an integer hash of its slot and its cycle, so a spark is a pure function of time and a street of braziers
// costs one draw and sixteen capsule distances per pixel of the quad.
//
// Production practice (`docs/research/flame-embers.md`): spawn many fast particles, "use vertex shader to extrude particle along velocity
// to do motion blur/hide strobing" (SIGGRAPH 2011 Advances, Fire/Flamethrowers/Thrusters), so each spark here is a capsule along its own
// velocity, and "crossfade ... over the lifetime", so each spark's colour and brightness fall with its temperature. The cooling is
// Newton's law, the term Scintilla's ember model (SIGGRAPH Asia 2023, Sec. 5.7) integrates with Stefan-Boltzmann loss.
//
// The quad faces the camera at any elevation and every length is in flame heights, so a spark's world rise `h` lands `h * cos(elevation)`
// up the screen and its lane toward the camera lands `z * sin(elevation)` down it: the same sparks read as a plume from the side and as
// a scatter over the bowl from above. The flame's own instance scale carries over to the quad (its length along x is the fire's catch,
// the other two are the wind's lean), as `flame.wesl` reads them, because the quad is the flame's child.
//
// Additive blending, no depth write, no prepass: the same material contract as the flame.

import bevy_pbr::render::{
    forward_io::{Vertex, decompress_vertex},
    mesh_functions,
    mesh_view_bindings::view,
    view_transformations::{position_world_to_clip, depth_ndc_to_view_z},
};
@if(DEPTH_PREPASS)
import bevy_pbr::prepass::utils::prepass_depth;

struct Embers {
    // The fire's colour (`FlameLook::color`, or a site's `FlameSite::color`); `.a` is the strength.
    base: vec4<f32>,
    // x: seconds (wrapped hourly), y: the tag's height scale in world units (`FlameLook::max_height`), z: 1 for the material of a site
    // that asked for the depth fade, w: the strength.
    params: vec4<f32>,
    // A spark's three colours as it cools, the shipped orange ones moved to the fire's hue by the CPU (`retint` in
    // `src/model/fire_colour.rs`): almost cool, in the middle of its life, and at its hottest. For the shipped orange they are the numbers
    // this shader used to hold as constants, `(0.80, 0.11, 0.01)`, `(1.0, 0.42, 0.07)` and `(1.0, 0.78, 0.30)`.
    dull: vec4<f32>,
    mild: vec4<f32>,
    hot: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> material: Embers;

const TAU: f32 = 6.2831853;
// Sparks drawn per flame: a spark's slot. Every slot emits one spark per cycle, and the cycle is `SLOTS / rate` seconds, so the flame
// emits `rate` sparks a second and never more than `SLOTS` are alive.
const SLOTS: u32 = 16u;
// The tag's rate byte is `rate / MAX_RATE`; `MAX_EMBER_RATE` in `src/embers.rs` is this number.
const MAX_RATE: f32 = 8.0;
// The same law as the flame's size (`flame.wesl`), so the sparks do not outgrow the flame they leave.
const CATCH_SIZE_EXPONENT: f32 = 0.6666667;
// How far up the quad reaches, in flame heights above the foot, and how wide it is either side of the axis.
const RISE_MAX: f32 = 2.6;
const HALF_WIDTH: f32 = 0.7;
// The quad's margin around the sparks, in flame heights.
const MARGIN: f32 = 0.35;
// How far a spark's drift outruns the flame's lean: a spark has no body to hold it in the plume.
const DRIFT: f32 = 1.5;
// A spark's life, in seconds, and the drag its climb decays with (per second).
const LIFE_MIN: f32 = 0.9;
const LIFE_MAX: f32 = 1.8;
const DRAG: f32 = 1.3;
// The streak: how many seconds of its own motion the capsule trails behind the spark.
const STREAK: f32 = 0.035;
// A spark's radius in flame heights (the glow's core), and the floor under it in pixels, below which the spark's energy is kept and its
// size is not (so a far fire's sparks shrink in brightness, not into aliasing).
const RADIUS_MIN: f32 = 0.010;
const RADIUS_MAX: f32 = 0.018;
const RADIUS_PIXELS: f32 = 1.2;
// Emission at a spark's hottest, against a flame's core of 1: sparks are brighter than the flame they leave, and bloom.
const SPARK_GAIN: f32 = 4.0;
// The depth fade, the flame's own (`flame.wesl`): the source is a third of the way up the flame, the ring of taps is this fraction of its
// height, and a tap hides the source when the scene is nearer by FADE_MARGIN of the flame's height.
const SOURCE_HEIGHT: f32 = 0.35;
const FADE_RADIUS: f32 = 0.10;
const FADE_MARGIN: f32 = 0.15;

struct EmberVertexOutput {
    @builtin(position) position: vec4<f32>,
    // The pixel's place on the plane the sparks live in, in flame heights: `.x` across, `.y` up the screen from the flame's foot.
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) phase: f32,
    // The fire's catch (its instance scale).
    @location(2) @interpolate(flat) took: f32,
    // `.x` the sparks a second, `.y` the signed sideways drift per flame height of rise (along the camera's right), `.z` the cosine and `.w`
    // the sine of the camera's elevation over the source.
    @location(3) @interpolate(flat) shape: vec4<f32>,
    @location(4) @interpolate(flat) fade: vec4<f32>,
    @location(5) @interpolate(flat) fade_margin: f32,
}

// ---- The depth fade, as `flame.wesl` has it: a fire whose source cannot be seen sheds no sparks -----------------------------

fn pixel_of(ndc: vec2<f32>) -> vec2<f32> {
    return view.viewport.xy + vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5) * view.viewport.zw;
}

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
fn tap_clear(pixel: vec2<f32>, source_z: f32, margin: f32) -> f32 {
    let low = view.viewport.xy;
    let high = view.viewport.xy + view.viewport.zw;
    if pixel.x < low.x || pixel.y < low.y || pixel.x >= high.x || pixel.y >= high.y {
        return 1.0;
    }
    let scene_z = depth_ndc_to_view_z(prepass_depth(vec4<f32>(pixel, 0.0, 1.0), 0u));
    return select(1.0, 0.0, scene_z > source_z + margin);
}

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
fn source_visibility(fade: vec4<f32>, margin: f32) -> f32 {
    return 1.0;
}

// -----------------------------------------------------------------------------------------------------------------------

@vertex
fn vertex(vertex_in: Vertex) -> EmberVertexOutput {
    var out: EmberVertexOutput;
    let vertex = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
    let origin = world_from_local[3].xyz;
    let took = clamp(length(world_from_local[0].xyz), 0.0, 1.0);
    let right = normalize(vec3<f32>(view.world_from_view[0].x, 0.0, view.world_from_view[0].z));
    let up = vec3<f32>(0.0, 1.0, 0.0);
    // The tag is the flame's (`pack_flame_tag`): the height in the high 16 bits, the clock phase in the next byte, and in the low byte
    // the rate, as a share of MAX_RATE.
    let raw = mesh_functions::get_tag(vertex_in.instance_index);
    let tag = unpack2x16unorm(raw);
    let size = tag.y * material.params.y * pow(max(took, 0.0001), CATCH_SIZE_EXPONENT);
    let centre = origin + up * (size * SOURCE_HEIGHT);
    let to_eye = view.world_from_view[3].xyz - centre;
    let sin_e = clamp(to_eye.y / max(length(to_eye), 1e-4), 0.0, 1.0);
    let cos_e = sqrt(max(1.0 - sin_e * sin_e, 0.0));
    // The wind's lean, as the flame reads it: `1 + sin θ · direction` on the instance scale's other two lengths. A spark drifts DRIFT
    // times as far sideways as the flame's axis leans.
    let wind_world = vec2<f32>(length(world_from_local[1].xyz), length(world_from_local[2].xyz)) - vec2<f32>(1.0);
    let drift = DRIFT * dot(wind_world, vec2<f32>(right.x, right.z));
    let reach = drift * RISE_MAX;
    let x_low = -HALF_WIDTH + min(reach, 0.0);
    let x_high = HALF_WIDTH + max(reach, 0.0);
    let v_low = -MARGIN;
    let v_high = RISE_MAX * cos_e + MARGIN;
    let screen_up = normalize(cross(normalize(to_eye), right));
    let across = mix(x_low, x_high, vertex.position.x + 0.5);
    let climb = mix(v_low, v_high, vertex.position.y + 0.5);
    let billboard = origin + right * (across * size) + screen_up * (climb * size);
    out.position = position_world_to_clip(billboard);
    out.uv = vec2<f32>(across, climb);
    out.phase = f32((raw >> 8u) & 0xffu) / 255.0 * TAU;
    out.took = took;
    out.shape = vec4<f32>(f32(raw & 0xffu) / 255.0 * MAX_RATE, drift, cos_e, sin_e);
    out.fade = fade_probe(origin, right, size);
    out.fade_margin = size * FADE_MARGIN;
    return out;
}

fn pcg(v: u32) -> u32 {
    let s = v * 747796405u + 2891336453u;
    let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
    return (w >> 22u) ^ w;
}

// A reproducible number in 0..1 from a seed and which of a spark's traits it is.
fn rnd(seed: u32, trait_id: u32) -> f32 {
    return f32(pcg(seed ^ (trait_id * 2654435761u))) / 4294967295.0;
}

// The distance from `p` to the segment `a`..`b`.
fn segment_distance(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let ab = b - a;
    let along = clamp(dot(p - a, ab) / max(dot(ab, ab), 1e-9), 0.0, 1.0);
    return distance(p, a + ab * along);
}

@fragment
fn fragment(in: EmberVertexOutput) -> @location(0) vec4<f32> {
    // Flame heights per pixel, taken before any branch (a derivative needs uniform control flow).
    let pixel = max(max(abs(dpdx(in.uv.x)), abs(dpdy(in.uv.x))), max(abs(dpdx(in.uv.y)), abs(dpdy(in.uv.y))));
    let rate = in.shape.x;
    if rate < 0.01 || in.took < 0.05 {
        return vec4<f32>(0.0);
    }
    let cycle = f32(SLOTS) / rate;
    let clock = material.params.x / cycle;
    let phase_bits = u32(in.phase * 40.0);
    var light = vec3<f32>(0.0);
    for (var i = 0u; i < SLOTS; i += 1u) {
        let turns = clock + f32(i) / f32(SLOTS) + in.phase / TAU;
        let n = floor(turns);
        let seed = pcg(i * 7919u + u32(n) * 104729u + phase_bits * 1299709u);
        let life = min(mix(LIFE_MIN, LIFE_MAX, rnd(seed, 0u)), cycle);
        // The spark is born somewhere in the cycle's slack, so the emission is not a metronome.
        let born = rnd(seed, 1u) * (cycle - life);
        let age = (turns - n) * cycle - born;
        if age < 0.0 || age >= life {
            continue;
        }
        let rise_start = mix(0.45, 0.80, rnd(seed, 2u));
        let speed = mix(0.45, 1.0, rnd(seed, 3u));
        let lane = (rnd(seed, 4u) - 0.5) * 0.22;
        let depth = (rnd(seed, 5u) - 0.5) * 0.30;
        let sway = mix(0.04, 0.12, rnd(seed, 6u));
        let sway_rate = mix(2.0, 5.0, rnd(seed, 7u));
        let sway_phase = rnd(seed, 8u) * TAU;
        // A spark climbs under drag: the height is the integral of `speed * exp(-DRAG * t)`.
        let held = exp(-DRAG * age);
        let rise = rise_start + speed * (1.0 - held) / DRAG;
        let climb_rate = speed * held;
        let weave = sway * (0.3 + age) * sin(sway_rate * age + sway_phase);
        let weave_rate = sway * (0.3 + age) * sway_rate * cos(sway_rate * age + sway_phase) + sway * sin(sway_rate * age + sway_phase);
        let across = lane + weave + in.shape.y * rise;
        let across_rate = weave_rate + in.shape.y * climb_rate;
        let head = vec2<f32>(across, rise * in.shape.z - depth * in.shape.w);
        let velocity = vec2<f32>(across_rate, climb_rate * in.shape.z);
        let tail = head - velocity * STREAK;
        let share = age / life;
        // Newton cooling: the spark's temperature share falls exponentially from its birth, and its colour and emission follow it.
        let heat = exp(-1.4 * share);
        let radius_true = mix(RADIUS_MAX, RADIUS_MIN, rnd(seed, 9u)) * (1.0 - 0.45 * share);
        let radius = max(radius_true, RADIUS_PIXELS * pixel);
        let reach = segment_distance(in.uv, tail, head) / radius;
        if reach > 5.0 {
            continue;
        }
        // A hot core with a small halo: sparks are points, and a wide glow reads as a smudge.
        let glow = (1.0 - smoothstep(0.5, 1.5, reach)) + 0.18 / (1.0 + reach * reach) * (1.0 - smoothstep(3.0, 5.0, reach));
        var colour = mix(material.dull.rgb, material.mild.rgb, smoothstep(0.25, 0.55, heat));
        colour = mix(colour, material.hot.rgb, smoothstep(0.55, 0.90, heat));
        // A spark too small for the pixel is drawn at the pixel's size and dimmed by the width it lost, so a far fire's sparks fade
        // instead of crawling.
        let energy = radius_true / radius;
        let wink = 0.75 + 0.25 * sin(sway_rate * 9.0 * age + sway_phase);
        let envelope = smoothstep(0.0, 0.08, share) * (1.0 - smoothstep(0.70, 1.0, share));
        light += colour * (glow * heat * sqrt(heat) * envelope * wink * min(energy, 1.0) * SPARK_GAIN);
    }
    let seen = source_visibility(in.fade, in.fade_margin) * in.took * in.took;
    return vec4<f32>(light * material.params.w * seen, 0.0);
}
