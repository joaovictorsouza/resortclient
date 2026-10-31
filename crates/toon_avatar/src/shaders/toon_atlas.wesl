// Atlas variant of toon.wesl (keep the two in sync): several materials share one draw call.
// The parameters of each piece come from `atlas.items[id]`, where `id` is stored per vertex in
// the vertex colour (x); the textures are atlases and the UVs are remapped when meshes merge.
//
// Toon character shader, ported from Unity's ShaderGraph_CharacterToon:
//   BaseColor = (ColorCustomization + Rimlight) * ToonShading
//
// ColorCustomization: the RGB mask picks three colour regions (R -> A, G -> B, B -> C),
//   each a gradient between two colours (x1, x2) driven by the remapped mask value.
// ToonShading: colour ramp indexed by (N.L + 1) / 2, scaled by (light + ambient) / 1.7.
// FakeOutline: near the silhouette the colour fades to albedo * outline_color, weight
//   1 - smoothstep(0, outline_thickness, dot(view, normal)). Disabled when thickness is 0.
// Rimlight: rim_color * (1 - smoothstep(0, thickness, dot(view, normal))).

import bevy_pbr::render::forward_io::VertexOutput;
import bevy_pbr::render::mesh_view_bindings::{view, lights};

struct ToonParams {
    a1: vec4<f32>,
    a2: vec4<f32>,
    b1: vec4<f32>,
    b2: vec4<f32>,
    c1: vec4<f32>,
    c2: vec4<f32>,
    flat_color: vec4<f32>,
    rim_color: vec4<f32>,
    outline_color: vec4<f32>,
    mask_remap: vec2<f32>,
    mask_factor: f32,
    thickness: f32,
    normal_adjust: f32,
    alpha_cutoff: f32,
    flags: u32,
    outline_thickness: f32,
};

const FLAG_FLAT: u32 = 1u;
const FLAG_HAS_NORMAL: u32 = 2u;
const FLAG_HAS_MASK: u32 = 4u;

struct AtlasParams {
    items: array<ToonParams, 8>,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> atlas: AtlasParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var mask_tex: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var mask_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var normal_tex: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var normal_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(5) var ramp_tex: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(6) var ramp_sampler: sampler;

fn remap01(toon: ToonParams, v: f32) -> f32 {
    return (v - toon.mask_remap.x) / (toon.mask_remap.y - toon.mask_remap.x);
}

fn customize(toon: ToonParams, m: vec3<f32>) -> vec3<f32> {
    // Unity's Smoothstep node takes (Edge1, Edge2, In); the graph wires Mask Factor to Edge2.
    let f = toon.mask_factor;
    let col_a = mix(toon.a1.rgb, mix(toon.a1.rgb, toon.a2.rgb, remap01(toon, m.r)), smoothstep(0.0, f, m.r));
    let grad_b = mix(toon.b1.rgb, toon.b2.rgb, remap01(toon, m.g));
    let ab = mix(col_a, grad_b, smoothstep(0.0, f, m.g));
    let grad_c = mix(toon.c1.rgb, toon.c2.rgb, remap01(toon, m.b));
    return mix(ab, grad_c, smoothstep(0.0, f, m.b));
}

// Screen-space tangent frame, so the meshes do not need vertex tangents.
fn perturb_normal(n: vec3<f32>, p: vec3<f32>, uv: vec2<f32>, map: vec3<f32>) -> vec3<f32> {
    let dp1 = dpdx(p);
    let dp2 = dpdy(p);
    let duv1 = dpdx(uv);
    let duv2 = dpdy(uv);
    let dp2perp = cross(dp2, n);
    let dp1perp = cross(n, dp1);
    let t = dp2perp * duv1.x + dp1perp * duv2.x;
    // glTF UVs have v pointing down, hence the flipped bitangent.
    let b = -(dp2perp * duv1.y + dp1perp * duv2.y);
    let invmax = inverseSqrt(max(max(dot(t, t), dot(b, b)), 1e-12));
    return normalize(t * invmax * map.x + b * invmax * map.y + n * map.z);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    let toon = atlas.items[u32(in.color.x + 0.5)];
    let mask = textureSample(mask_tex, mask_sampler, in.uv);
    let has_mask = (toon.flags & FLAG_HAS_MASK) != 0u;

    var albedo: vec3<f32>;
    if ((toon.flags & FLAG_FLAT) != 0u || !has_mask) {
        albedo = toon.flat_color.rgb;
        if (has_mask) {
            albedo = albedo * mask.rgb;
        }
    } else {
        albedo = customize(toon, mask.rgb);
    }

    if (toon.alpha_cutoff > 0.0 && mask.a < toon.alpha_cutoff) {
        discard;
    }

    var n = normalize(in.world_normal);
    if (!is_front) {
        n = -n;
    }
    if ((toon.flags & FLAG_HAS_NORMAL) != 0u) {
        let tn = textureSample(normal_tex, normal_sampler, in.uv).xyz * 2.0 - 1.0;
        n = perturb_normal(n, in.world_position.xyz, in.uv, tn);
    }
    n = normalize(mix(n, vec3<f32>(0.0, -0.5, 1.0), toon.normal_adjust));

    // Light: direction only. Bevy's colours are in photometric units, so use a
    // normalised white light plus a fixed ambient, as the Unity result is ~1.0.
    var to_light = vec3<f32>(0.0, 1.0, 0.0);
    if (lights.n_directional_lights > 0u) {
        to_light = lights.directional_lights[0].direction_to_light;
    }
    let ndotl = dot(n, to_light);
    let ramp = textureSample(ramp_tex, ramp_sampler, vec2<f32>((ndotl + 1.0) * 0.5, 0.5)).rgb;
    let light_term = (vec3<f32>(1.0) + vec3<f32>(0.7)) / 1.7;

    let view_dir = normalize(view.world_position.xyz - in.world_position.xyz);
    let rim = toon.rim_color.rgb * (1.0 - smoothstep(0.0, toon.thickness, dot(view_dir, n)));

    var color = (albedo + rim) * ramp * light_term;
    if (toon.outline_thickness > 0.0) {
        let edge = 1.0 - smoothstep(0.0, toon.outline_thickness, dot(view_dir, n));
        color = mix(color, albedo * toon.outline_color.rgb, edge);
    }
    return vec4<f32>(color, 1.0);
}
