// Factor-only KHR_materials_sheen for the admitted pouch. Charlie distribution
// and visibility, with the specification's view-only albedo scaling option.
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings::{lights, view},
    mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT,
    mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT,
    shadows::fetch_directional_shadow,
};

struct Sheen {
    color_roughness: vec4<f32>,
    albedo: array<vec4<f32>, 16>,
}
@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> sheen: Sheen;

fn fit(x: f32, alpha: f32) -> f32 {
    let w = (1.0 - alpha) * (1.0 - alpha);
    return mix(21.5473, 25.3245, w) / (1.0 + mix(3.82987, 3.32435, w) * pow(x, mix(0.19823, 0.16801, w)))
        + mix(-1.97760, -1.27393, w) * x + mix(-4.32054, -4.85967, w);
}
fn lambda(cosine: f32, alpha: f32) -> f32 {
    if (cosine < 0.5) { return exp(fit(cosine, alpha)); }
    return exp(2.0 * fit(0.5, alpha) - fit(1.0 - cosine, alpha));
}
fn directional_albedo(nv: f32) -> f32 {
    let position = clamp(nv * 64.0 - 0.5, 0.0, 63.0);
    let lo = u32(position);
    let hi = min(lo + 1u, 63u);
    return mix(sheen.albedo[lo / 4u][lo % 4u], sheen.albedo[hi / 4u][hi % 4u], fract(position));
}

@fragment
fn fragment(vertex: VertexOutput, @builtin(front_facing) front: bool) -> FragmentOutput {
    var input = pbr_input_from_standard_material(vertex, front);
    input.material.base_color = alpha_discard(input.material.flags, input.material.alpha_cutoff, input.material.base_color);
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(input);
    let nv = clamp(dot(input.N, input.V), 0.0001, 1.0);
    let alpha = max(sheen.color_roughness.a * sheen.color_roughness.a, 0.001);
    let color = sheen.color_roughness.rgb;
    let albedo = directional_albedo(nv);
    var reflected = lights.ambient_color.rgb * albedo * input.diffuse_occlusion;
    let view_z = (view.view_from_world * input.world_position).z;
    for (var i = 0u; i < lights.n_directional_lights; i += 1u) {
        let light = lights.directional_lights[i];
        let nl = max(dot(input.N, light.direction_to_light), 0.0);
        if (nl > 0.0) {
            let h = normalize(input.V + light.direction_to_light);
            let nh = clamp(dot(input.N, h), 0.0, 1.0);
            let distribution = (2.0 + 1.0 / alpha) * pow(max(1.0 - nh * nh, 0.0), 0.5 / alpha) / 6.28318530718;
            let visibility = 1.0 / (4.0 * nv * nl * (1.0 + lambda(nv, alpha) + lambda(nl, alpha)));
            var shadow = input.directional_shadow_factor;
            if ((input.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u && (light.flags & DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
                shadow *= fetch_directional_shadow(i, input.world_position, input.world_normal, view_z, input.frag_coord.xy);
            }
            reflected += light.color.rgb * nl * distribution * visibility * shadow;
        }
    }
    let base_scale = 1.0 - max(color.r, max(color.g, color.b)) * albedo;
    out.color = vec4(out.color.rgb * base_scale + reflected * color * view.exposure, out.color.a);
    out.color = main_pass_post_lighting_processing(input, out.color);
    return out;
}
