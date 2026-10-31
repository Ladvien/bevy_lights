// The coals under a fire (`src/bed.rs`, `FuelBed`): a disc lying flat at the site's foot on which a field of coal lumps glows, dark ash with
// orange and yellow shards in it, hottest under the flame's axis and ragged at the rim. Drawn as one additive mesh instance per bed, as a
// function of the shared clock and the instance's tag (its heat and where in the tile its window starts): no particle state, no per-bed
// material, so every bed in the scene shares one material and one pipeline.
//
// Additive blending, no depth write, no prepass: the same material contract as the flame. The coals add their light over what lies behind
// them and darken nothing (there is no black plate on the ground), and the opaque logs and stones above the bed hide it through the depth
// test. `docs/research/fuel-bed-heat.md` has the sources and the model.
//
// The colour of a coal is a table lookup (`material.ramp`, `bed_ramp` in `src/model/bed.rs`): the char field of the lump on one axis and the
// bed's heat on the other, both blended linearly. The ramp's three stops are temperatures, and the heat moves every one of them down the
// blackbody law, so a cooling bed goes through a deepening red to nothing. `RAMP_FIELD`, `RAMP_HEAT` and `RAMP_LEN` here are the model's
// `BED_RAMP_FIELD`, `BED_RAMP_HEAT` and their product: a mismatch is a uniform buffer of the wrong size, which wgpu refuses loudly.

import bevy_pbr::render::{
    forward_io::{Vertex, decompress_vertex},
    mesh_functions,
    view_transformations::position_world_to_clip,
};

const RAMP_FIELD: u32 = 32u;
const RAMP_HEAT: u32 = 24u;
const RAMP_LEN: u32 = 768u;

struct Bed {
    // x: seconds (wrapped hourly), y: the tile's width in metres, z: the strength (the scene's fire strength).
    params: vec4<f32>,
    // The colour of the coals: `RAMP_HEAT` rows of `RAMP_FIELD` columns, row-major.
    ramp: array<vec4<f32>, RAMP_LEN>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> material: Bed;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var field_texture: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var field_sampler: sampler;

// How much of the heap's colour the shimmer moves, and how fast: the lumps of a bed brighten and dim slowly and each at its own time, which is
// what makes a bed of coals crawl. Its scale is a coal's, six to the tile.
const SHIMMER: f32 = 0.24;
const SHIMMER_LUMPS: f32 = 6.0;
const SHIMMER_SPEED: f32 = 0.35;
// How much cooler the rim of the bed is than its middle, in the field: the plume is axisymmetric, so the coals under its axis are the
// hottest (`CORE_COOL` in `src/model/char.rs`, which this is the disc's own form of).
const RIM_COOL: f32 = 0.38;

struct BedVertexOutput {
    @builtin(position) position: vec4<f32>,
    // The place on the disc's square, `0..1` on both axes: the middle is `0.5`.
    @location(0) uv: vec2<f32>,
    // The heat the bed is drawn at, `0..1`.
    @location(1) @interpolate(flat) heat: f32,
    // Where in the tile this bed's window starts, as a share of the tile.
    @location(2) @interpolate(flat) offset: vec2<f32>,
    // The disc's width in metres (its instance scale).
    @location(3) @interpolate(flat) width: f32,
}

@vertex
fn vertex(vertex_in: Vertex) -> BedVertexOutput {
    var out: BedVertexOutput;
    let vertex = decompress_vertex(vertex_in, vertex_in.instance_index);
    let world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
    let world = world_from_local * vec4<f32>(vertex.position, 1.0);
    out.position = position_world_to_clip(world.xyz);
    // The mesh is a unit square in the ground plane, centred: its place in `0..1` is its `x` and `z` plus a half.
    out.uv = vertex.position.xz + vec2<f32>(0.5);
    // The tag is the bed's (`pack_bed_tag`): the heat in the low sixteen bits, the tile offset's two bytes above.
    let raw = mesh_functions::get_tag(vertex_in.instance_index);
    out.heat = f32(raw & 0xffffu) / 65535.0;
    out.offset = vec2<f32>(f32((raw >> 16u) & 0xffu), f32((raw >> 24u) & 0xffu)) / 255.0;
    out.width = length(world_from_local[0].xyz);
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

// The colour of a coal of char field `field` in a bed of `heat`, both `0..1`: the table's four nearest entries, blended.
fn ramp_at(heat: f32, field: f32) -> vec3<f32> {
    let h = clamp(heat, 0.0, 1.0) * f32(RAMP_HEAT - 1u);
    let f = clamp(field, 0.0, 1.0) * f32(RAMP_FIELD - 1u);
    let row = min(u32(floor(h)), RAMP_HEAT - 2u);
    let column = min(u32(floor(f)), RAMP_FIELD - 2u);
    let t_heat = h - f32(row);
    let t_field = f - f32(column);
    let at = row * RAMP_FIELD + column;
    let a = material.ramp[at].rgb;
    let b = material.ramp[at + 1u].rgb;
    let c = material.ramp[at + RAMP_FIELD].rgb;
    let d = material.ramp[at + RAMP_FIELD + 1u].rgb;
    return mix(mix(a, b, t_field), mix(c, d, t_field), t_heat);
}

@fragment
fn fragment(in: BedVertexOutput) -> @location(0) vec4<f32> {
    let centred = in.uv - vec2<f32>(0.5);
    // `0` at the disc's middle and `1` at its rim (the disc is inscribed in the square).
    let rim = length(centred) * 2.0;
    // The lumps' place in the repeating tile, laid at the coals' own size: a wider bed has more coals, not larger ones.
    let p = centred * (in.width / material.params.y) + in.offset;
    // Sampled before any branch: a derivative (the mip level) needs uniform control flow.
    let base = textureSample(field_texture, field_sampler, p).r;
    let t = material.params.x;
    // Each lump brightens and dims at its own pace.
    let crawl = value_noise(p * SHIMMER_LUMPS + vec2<f32>(t * SHIMMER_SPEED, -0.6 * t * SHIMMER_SPEED));
    var field = base * (1.0 - 0.5 * SHIMMER + SHIMMER * crawl);
    // The bed cools from its middle to its rim.
    field = field * (1.0 - RIM_COOL * smoothstep(0.0, 1.0, pow(rim, 1.4)));
    // The edge is ragged ash: a bright lump holds on further out than a dull one, and everything is gone at the rim.
    let rim_reach = rim + 0.2 * (0.5 - base);
    let edge = (1.0 - smoothstep(0.5, 0.92, rim_reach)) * (1.0 - smoothstep(0.82, 1.0, rim));
    let colour = ramp_at(in.heat, field) * (material.params.z * edge);
    return vec4<f32>(colour, 0.0);
}
