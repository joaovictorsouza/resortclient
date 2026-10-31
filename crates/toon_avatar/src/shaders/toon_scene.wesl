// Toon look for the scene (floors, walls, furniture): the colour (and texture) of a StandardMaterial,
// shaded with the same ramp and light model as the characters (see toon.wesl), plus a soft dark edge.

import bevy_pbr::render::forward_io::VertexOutput;
import bevy_pbr::render::mesh_view_bindings::{view, lights};

struct SceneParams {
    color: vec4<f32>,
    alpha_cutoff: f32,
    edge: f32,
    flags: u32,
    pad: u32,
};

const FLAG_HAS_TEXTURE: u32 = 1u;
const FLAG_MASK: u32 = 2u;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> scene: SceneParams;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var base_tex: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var base_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var ramp_tex: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(4) var ramp_sampler: sampler;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    var albedo = scene.color;
    if ((scene.flags & FLAG_HAS_TEXTURE) != 0u) {
        albedo = albedo * textureSample(base_tex, base_sampler, in.uv);
    }
    if ((scene.flags & FLAG_MASK) != 0u && albedo.a < scene.alpha_cutoff) {
        discard;
    }

    var n = normalize(in.world_normal);
    if (!is_front) {
        n = -n;
    }
    var to_light = vec3<f32>(0.0, 1.0, 0.0);
    if (lights.n_directional_lights > 0u) {
        to_light = lights.directional_lights[0].direction_to_light;
    }
    let ndotl = dot(n, to_light);
    let ramp = textureSample(ramp_tex, ramp_sampler, vec2<f32>((ndotl + 1.0) * 0.5, 0.5)).rgb;
    let light_term = (vec3<f32>(1.0) + vec3<f32>(0.7)) / 1.7;

    var rgb = albedo.rgb * ramp * light_term;
    if (scene.edge > 0.0) {
        let view_dir = normalize(view.world_position.xyz - in.world_position.xyz);
        let rim = 1.0 - smoothstep(0.0, scene.edge, dot(view_dir, n));
        rgb = mix(rgb, albedo.rgb * 0.35, rim);
    }
    return vec4<f32>(rgb, albedo.a);
}
