// The smoke of a fire (`src/smoke.rs`, `FlameSmoke`): the plume that goes on up above the flame, lit by it. Drawn on ONE camera-facing quad
// per flame as a function of the clock alone (no particle state, no buffers): a column whose centreline leaves the flame's tip and bends
// downwind, whose width is a turbulent plume's, and whose inside is wisps of noise carried up it.
//
// Additive blending, no depth write, no prepass: the same material contract as the flame and the sparks. Smoke at night is visible only by the
// light the fire puts into it, which is exactly what an additive layer draws, and an additive layer joins no sorting (SIGGRAPH 2025 Advances,
// the OIT talk: blended VFX smoke is the case that sorts badly). `docs/research/flame-smoke.md` has the sources and the model.
//
// The quad is the ember quad's (`src/embers.wesl`): the same extents in flame heights, the same vertex stage, so it takes the fire's catch
// and the wind's lean from the flame's own instance scale and `ember_cull_extents` is its culling box. Change `HALF_WIDTH`, `RISE_MAX`,
// `MARGIN` or `LEAN_TOP` here and the port in `src/smoke.rs`'s culling test and `model/tag.rs`'s mirrors must change with them.

import bevy_pbr::render::{
    forward_io::{Vertex, decompress_vertex},
    mesh_functions,
    mesh_view_bindings::view,
    view_transformations::{position_world_to_clip, depth_ndc_to_view_z},
};
@if(DEPTH_PREPASS)
import bevy_pbr::prepass::utils::prepass_depth;

struct Smoke {
    // The fire's colour (`FlameLook::color`, or a site's `FlameSite::color`); `.a` is the strength.
    base: vec4<f32>,
    // x: seconds (wrapped hourly), y: the tag's height scale in world units (`FlameLook::max_height`), z: 1 for the material of a site
    // that asked for the depth fade, w: the strength.
    params: vec4<f32>,
    // The smoke's two colours, the sparks' orange and dull-red stops moved to the fire's hue by the CPU (`retint`): the lit smoke just above
    // the flame, and what it has cooled to near the top of the column.
    warm: vec4<f32>,
    cool: vec4<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> material: Smoke;

const TAU: f32 = 6.2831853;
// The same law as the flame's size (`flame.wesl`), so the smoke does not outgrow the flame it leaves.
const CATCH_SIZE_EXPONENT: f32 = 0.6666667;
// How far up the quad reaches, in flame heights above the foot, and how wide it is either side of the axis: the ember quad's.
const RISE_MAX: f32 = 2.6;
const HALF_WIDTH: f32 = 0.7;
const MARGIN: f32 = 0.35;
// How far downwind the column's top is, in flame heights per unit of the flame's lean: the ember quad's `DRIFT * RISE_MAX`. A plume in a
// crosswind is a bent-over plume, which rises as `x ∝ z^(3/2)` (Briggs), so the column's centreline is `LEAN_BEND * lean * rise^1.5`:
// the flame's tip (`rise = 1`) to within 7 %, and `LEAN_TOP` at `RISE_MAX` (`LEAN_TOP / RISE_MAX^1.5`).
const LEAN_TOP: f32 = 3.9;
const LEAN_BEND: f32 = 0.9303;
// Emission of a column at density 1, against a flame's core of 1.
const SMOKE_GAIN: f32 = 0.85;
// The flame's middle, where its light is and where the plume's virtual origin is, in flame heights.
const SOURCE: f32 = 0.35;
// The depth fade, the flame's own (`flame.wesl`): the source is a third of the way up the flame, the ring of taps is this fraction of its
// height, and a tap hides the source when the scene is nearer by FADE_MARGIN of the flame's height.
const SOURCE_HEIGHT: f32 = 0.35;
const FADE_RADIUS: f32 = 0.10;
const FADE_MARGIN: f32 = 0.15;

struct SmokeVertexOutput {
    @builtin(position) position: vec4<f32>,
    // The pixel's place on the plane the smoke lives in, in flame heights: `.x` across, `.y` up the screen from the flame's foot.
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) phase: f32,
    // The fire's catch (its instance scale).
    @location(2) @interpolate(flat) took: f32,
    // `.x` the density, `.y` the signed sideways lean per flame height (along the camera's right), `.z` the cosine and `.w` the sine of the
    // camera's elevation over the source.
    @location(3) @interpolate(flat) shape: vec4<f32>,
    @location(4) @interpolate(flat) fade: vec4<f32>,
    @location(5) @interpolate(flat) fade_margin: f32,
}

// ---- The depth fade, as `flame.wesl` has it: a fire whose source cannot be seen sheds no smoke -----------------------------

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
fn vertex(vertex_in: Vertex) -> SmokeVertexOutput {
    var out: SmokeVertexOutput;
    let vertex = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
    let origin = world_from_local[3].xyz;
    let took = clamp(length(world_from_local[0].xyz), 0.0, 1.0);
    let right = normalize(vec3<f32>(view.world_from_view[0].x, 0.0, view.world_from_view[0].z));
    let up = vec3<f32>(0.0, 1.0, 0.0);
    // The tag is the flame's (`pack_flame_tag`): the height in the high 16 bits, the clock phase in the next byte, and in the low byte the
    // density, as a share of the most density a site can ask for (1).
    let raw = mesh_functions::get_tag(vertex_in.instance_index);
    let tag = unpack2x16unorm(raw);
    let size = tag.y * material.params.y * pow(max(took, 0.0001), CATCH_SIZE_EXPONENT);
    let centre = origin + up * (size * SOURCE_HEIGHT);
    let to_eye = view.world_from_view[3].xyz - centre;
    let sin_e = clamp(to_eye.y / max(length(to_eye), 1e-4), 0.0, 1.0);
    let cos_e = sqrt(max(1.0 - sin_e * sin_e, 0.0));
    // The wind's lean, as the flame reads it: `1 + sin θ · direction` on the instance scale's other two lengths.
    let wind_world = vec2<f32>(length(world_from_local[1].xyz), length(world_from_local[2].xyz)) - vec2<f32>(1.0);
    let lean = dot(wind_world, vec2<f32>(right.x, right.z));
    let reach = lean * LEAN_TOP;
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
    out.shape = vec4<f32>(f32(raw & 0xffu) / 255.0, lean, cos_e, sin_e);
    out.fade = fade_probe(origin, right, size);
    out.fade_margin = size * FADE_MARGIN;
    return out;
}

fn pcg(v: u32) -> u32 {
    let s = v * 747796405u + 2891336453u;
    let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
    return (w >> 22u) ^ w;
}

// A reproducible number in 0..1 at an integer lattice point.
fn lattice(p: vec2<i32>) -> f32 {
    return f32(pcg((u32(p.x) * 1597334677u) ^ (u32(p.y) * 3812015801u))) / 4294967295.0;
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = vec2<i32>(floor(p));
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = lattice(i);
    let b = lattice(i + vec2<i32>(1, 0));
    let c = lattice(i + vec2<i32>(0, 1));
    let d = lattice(i + vec2<i32>(1, 1));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Three octaves, each turned and doubled, in 0..1. `detail` in 0..1 fades the octaves out toward their mean, so a column too far away for its
// wisps to be resolved is a soft haze and not a crawl of noise.
fn wisps(p: vec2<f32>, detail: f32) -> f32 {
    var sum = 0.0;
    var amp = 0.5;
    var q = p;
    for (var octave = 0; octave < 3; octave += 1) {
        sum += amp * mix(0.5, value_noise(q), detail);
        q = vec2<f32>(q.x * 0.8 - q.y * 0.6, q.x * 0.6 + q.y * 0.8) * 2.03 + vec2<f32>(17.0, 5.0);
        amp *= 0.5;
    }
    return sum / 0.875;
}

// The light a smoke column gives at `uv` (flame heights across and up the screen), `clock` seconds, `phase` radians, a `lean` of the flame
// along the camera's right, and the camera's elevation cosine. `pixel` is flame heights per pixel.
fn smoke_light(uv: vec2<f32>, clock: f32, phase: f32, lean: f32, cos_e: f32, pixel: f32) -> vec3<f32> {
    // The column's own height above the foot: the screen's `up` is `rise * cos_e`, so a camera above the fire sees the column foreshortened.
    let rise = uv.y / max(cos_e, 0.35);
    if rise < 0.5 || rise > RISE_MAX {
        return vec3<f32>(0.0);
    }
    let t = clock + phase * 3.1;
    let above = max(rise - 0.7, 0.0);
    // The centreline leaves the flame's tip and bends downwind as a bent-over plume (`x ∝ z^1.5`), and a slow meander grows with height so a
    // calm column wanders. `slope` is how far sideways it goes per unit of rise, and `stretch` the length of the path per unit of rise.
    let reach = sqrt(rise);
    let meander = 0.16 * above * sin(0.8 * t + 1.9 * rise + phase);
    let centre = LEAN_BEND * lean * rise * reach + meander;
    let slope = 1.5 * LEAN_BEND * lean * reach;
    let stretch = sqrt(1.0 + slope * slope);
    // A turbulent plume spreads linearly from its virtual origin, the flame's middle (Morton, Taylor and Turner: `b = 1.2 α z`, α ≈ 0.13), in
    // the plume's own frame: a bent column is cut obliquely by the screen's horizontal, so its half-width across is `stretch` times that. It is
    // never drawn thinner than a pixel and a quarter: a thinner column keeps its energy in a wider, dimmer one instead of crawling.
    let true_width = (0.05 + 0.156 * max(rise - SOURCE, 0.0)) * stretch;
    let half_width = max(true_width, 1.25 * pixel);
    let across = uv.x - centre;
    if abs(across) > 2.4 * half_width {
        return vec3<f32>(0.0);
    }
    // Wisps, carried along the column and warped sideways by a curl that grows with height; the column's edge is ragged by the same curl. The
    // noise lives in the plume's own frame (across it and along its path), so a bent column is bent all the way through.
    let along = rise * stretch;
    let detail = 1.0 - smoothstep(0.02, 0.06, pixel);
    var p = vec2<f32>(across / stretch * 3.0, along * 2.2 - t * 0.7);
    let curl = value_noise(p * 0.55 + vec2<f32>(3.1, t * 0.15)) - 0.5;
    p.x += (0.5 + 0.9 * above) * curl;
    let lateral = (across + 0.45 * half_width * curl) / half_width;
    let envelope = exp(-lateral * lateral * 1.6);
    let body = 0.3 + 0.7 * smoothstep(0.2, 0.8, wisps(p + vec2<f32>(phase * 5.0, 0.0), detail));
    let born = smoothstep(0.55, 1.15, rise);
    let gone = 1.0 - smoothstep(1.4, RISE_MAX, rise);
    // Lit by the flame: an inverse square from its middle, softened so it does not blow up at the source. The distance is the straight one in
    // the plane the column lives in, not the path's: a bent column is as close to its fire as the wind lets it be.
    let away = length(vec2<f32>(uv.x, rise - SOURCE)) / 0.7;
    let lit = 1.0 / (1.0 + away * away);
    // The sparks' orange going to their dull red as the column cools, pulled toward its own luminance so it reads as warm grey smoke and not as
    // a second flame.
    var colour = mix(material.warm.rgb, material.cool.rgb, smoothstep(0.9, 2.4, rise));
    colour = mix(colour, vec3<f32>(dot(colour, vec3<f32>(0.2126, 0.7152, 0.0722))), 0.35);
    return colour * (SMOKE_GAIN * envelope * body * born * gone * lit * (true_width / half_width));
}

@fragment
fn fragment(in: SmokeVertexOutput) -> @location(0) vec4<f32> {
    // Flame heights per pixel, taken before any branch (a derivative needs uniform control flow).
    let pixel = max(max(abs(dpdx(in.uv.x)), abs(dpdy(in.uv.x))), max(abs(dpdx(in.uv.y)), abs(dpdy(in.uv.y))));
    let density = in.shape.x;
    if density < 0.004 || in.took < 0.05 {
        return vec4<f32>(0.0);
    }
    let light = smoke_light(in.uv, material.params.x, in.phase, in.shape.y, in.shape.z, pixel);
    if max(light.r, max(light.g, light.b)) <= 0.0 {
        return vec4<f32>(0.0);
    }
    let seen = source_visibility(in.fade, in.fade_margin) * in.took * in.took;
    return vec4<f32>(light * (density * material.params.w * seen), 0.0);
}
