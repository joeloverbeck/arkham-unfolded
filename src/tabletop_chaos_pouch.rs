//! The admitted chaos pouch's glTF scene and factor-only velvet sheen.
//!
//! This adapter is deliberately limited to the hash-pinned pouch and the table's
//! directional/ambient lighting. Textured sheen and environment maps are not
//! supported. Other imported PBR properties remain Bevy's responsibility.

use bevy::{
    asset::LoadContext,
    gltf::{
        GltfMaterial,
        extensions::{ErasedGltfExtensionHandler, GltfExtensionHandler, GltfExtensionHandlers},
        gltf,
    },
    pbr::{
        ExtendedMaterial, MaterialExtension, MaterialPlugin,
        gltf::standard_material_from_gltf_material,
    },
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};

mod textures;

pub(crate) const PATH: &str = "models/chaos-pouch.glb";
pub(crate) const SCENE: &str = "models/chaos-pouch.glb#Scene0";
pub(crate) const VELVET: &str = "models/chaos-pouch.glb#Material3/pouch_sheen";
pub(crate) type VelvetMaterial = ExtendedMaterial<StandardMaterial, VelvetSheen>;

pub(crate) struct ChaosPouchPlugin;

impl Plugin for ChaosPouchPlugin {
    fn build(&self, app: &mut App) {
        bevy::shader::load_shader_library!(app, "tabletop_chaos_pouch.wesl");
        app.add_plugins(MaterialPlugin::<VelvetMaterial>::default());
        // Prepare before the first render extraction, so material bindings never
        // retain an earlier GPU texture view containing only the base level.
        app.add_systems(
            PostUpdate,
            textures::prepare.after(bevy::asset::AssetEventSystems),
        );
        app.world_mut()
            .resource_mut::<GltfExtensionHandlers>()
            .0
            .write_blocking()
            .push(Box::new(PouchMaterials));
    }
}

pub(crate) fn material_and_textures(server: &AssetServer) -> Vec<bevy::asset::UntypedHandle> {
    let mut handles = vec![server.load::<VelvetMaterial>(VELVET).untyped()];
    handles.extend((0..9).map(|index| {
        server
            .load::<Image>(format!("{PATH}#Texture{index}"))
            .untyped()
    }));
    handles
}

pub(crate) fn texture_ready(image: &Image) -> bool {
    image.texture_descriptor.mip_level_count > 1
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub(crate) struct VelvetSheen {
    #[uniform(100)]
    color_roughness: Vec4,
    // Directional hemispherical albedo, packed to obey uniform alignment.
    #[uniform(100)]
    albedo: [Vec4; 16],
}

impl MaterialExtension for VelvetSheen {
    fn fragment_shader() -> ShaderRef {
        "embedded://arkham_unfolded_experiment/tabletop_chaos_pouch.wesl".into()
    }
}

#[derive(Clone)]
struct PouchMaterials;

impl GltfExtensionHandler for PouchMaterials {
    fn dyn_clone(&self) -> Box<dyn ErasedGltfExtensionHandler> {
        Box::new(self.clone())
    }

    fn on_material(
        &mut self,
        context: &mut LoadContext<'_>,
        source: &gltf::Material,
        _handle: Handle<GltfMaterial>,
        material: &GltfMaterial,
        label: &str,
    ) {
        if context.path().path() != std::path::Path::new(PATH) || source.index() != Some(3) {
            return;
        }
        // Missing/unsupported factors deliberately leave the required custom
        // label absent. Startup then refuses play instead of showing flat velvet.
        let Some(extension) = source
            .extension_value("KHR_materials_sheen")
            .and_then(VelvetSheen::from_gltf)
        else {
            return;
        };
        context.add_labeled_asset(
            format!("{label}/pouch_sheen"),
            VelvetMaterial {
                base: standard_material_from_gltf_material(material),
                extension,
            },
        );
    }

    fn on_spawn_mesh_and_material(
        &mut self,
        context: &mut LoadContext<'_>,
        _primitive: &gltf::Primitive,
        _mesh: &gltf::Mesh,
        source: &gltf::Material,
        entity: &mut EntityWorldMut,
        label: &str,
    ) {
        if context.path().path() != std::path::Path::new(PATH) || source.index() != Some(3) {
            return;
        }
        let handle = context.get_label_handle::<VelvetMaterial>(format!("{label}/pouch_sheen"));
        entity
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert(MeshMaterial3d(handle));
    }
}

impl VelvetSheen {
    #[cfg(test)]
    pub(crate) fn disable_for_test(&mut self) {
        self.color_roughness = Vec4::new(0.0, 0.0, 0.0, self.color_roughness.w);
    }

    fn from_gltf(value: &serde_json::Value) -> Option<Self> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct Factors {
            sheen_color_factor: [f32; 3],
            sheen_roughness_factor: f32,
        }
        let factors: Factors = serde_json::from_value(value.clone()).ok()?;
        if !factors
            .sheen_color_factor
            .iter()
            .all(|v| (0.0..=1.0).contains(v))
            || !(0.0..=1.0).contains(&factors.sheen_roughness_factor)
        {
            return None;
        }
        let roughness = factors.sheen_roughness_factor;
        let mut albedo = [Vec4::ZERO; 16];
        for i in 0..64 {
            albedo[i / 4][i % 4] = sheen_albedo((i as f64 + 0.5) / 64.0, roughness as f64);
        }
        Some(Self {
            color_roughness: Vec3::from_array(factors.sheen_color_factor).extend(roughness),
            albedo,
        })
    }
}

// Numerical hemisphere integration of the Khronos KHR_materials_sheen Charlie
// BRDF. Computed once on load for the imported roughness, not every frame.
// https://github.com/KhronosGroup/glTF/tree/main/extensions/2.0/Khronos/KHR_materials_sheen
fn sheen_albedo(nv: f64, roughness: f64) -> f32 {
    let alpha = (roughness * roughness).max(0.001);
    let lambda = |cos: f64| {
        let fit = |x: f64| {
            let weight = (1.0 - alpha).powi(2);
            let mix = |a: f64, b: f64| a + (b - a) * weight;
            mix(21.5473, 25.3245) / (1.0 + mix(3.82987, 3.32435) * x.powf(mix(0.19823, 0.16801)))
                + mix(-1.97760, -1.27393) * x
                + mix(-4.32054, -4.85967)
        };
        if cos < 0.5 {
            fit(cos).exp()
        } else {
            (2.0 * fit(0.5) - fit(1.0 - cos)).exp()
        }
    };
    let mut integral = 0.0;
    for elevation in 0..64 {
        let nl = (elevation as f64 + 0.5) / 64.0;
        for azimuth in 0..128 {
            let phi = std::f64::consts::TAU * (azimuth as f64 + 0.5) / 128.0;
            let vl = nv * nl + (1.0 - nv * nv).sqrt() * (1.0 - nl * nl).sqrt() * phi.cos();
            let nh = (nv + nl) / (2.0 + 2.0 * vl).sqrt();
            let distribution = (2.0 + 1.0 / alpha) * (1.0 - nh * nh).max(0.0).powf(0.5 / alpha)
                / std::f64::consts::TAU;
            integral += distribution / (4.0 * nv * (1.0 + lambda(nv) + lambda(nl)));
        }
    }
    (integral * std::f64::consts::TAU / (64.0 * 128.0)) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sheen_requires_supported_linear_factors_and_refuses_textured_variants() {
        let value = serde_json::json!({"sheenColorFactor": [0.08, 0.14, 0.15], "sheenRoughnessFactor": 0.75});
        let material = VelvetSheen::from_gltf(&value).unwrap();
        assert_eq!(material.color_roughness, Vec4::new(0.08, 0.14, 0.15, 0.75));
        assert!(
            material
                .albedo
                .iter()
                .all(|v| v.is_finite() && v.min_element() >= 0.0 && v.max_element() <= 1.0)
        );
        assert!(
            material.albedo[0].x > material.albedo[15].w,
            "velvet reflects more at grazing angles"
        );
        for field in ["sheenColorTexture", "sheenRoughnessTexture"] {
            let mut unsupported = value.clone();
            unsupported[field] = serde_json::json!({"index": 0});
            assert!(VelvetSheen::from_gltf(&unsupported).is_none());
        }
        for roughness in [-0.1, 1.1] {
            let mut invalid = value.clone();
            invalid["sheenRoughnessFactor"] = serde_json::json!(roughness);
            assert!(VelvetSheen::from_gltf(&invalid).is_none());
        }
        assert!(VelvetSheen::from_gltf(&serde_json::json!({})).is_none());
    }
}
