//! GPU-specific render workarounds.
//!
//! Skinned meshes (our avatars) are corrupted on Android/Vulkan/Adreno GPUs when Bevy skins through a
//! storage buffer (<https://github.com/bevyengine/bevy/issues/24926>, open upstream). Bevy falls back to
//! uniform-buffer skinning when the device reports no storage buffers, so on the Adreno models reported
//! in that issue we cap the device limits to make it take that path. Remove this module once the
//! upstream bug is fixed.

use bevy::render::settings::{WgpuLimits, WgpuSettings};

/// Adreno models reported as affected in bevyengine/bevy#24926 (Adreno 730 and 650, Bevy 0.19.x).
/// No other Adreno model is known to be affected or known to work, so only these are gated; add a
/// model here when the issue reports it.
const AFFECTED_ADRENO_MODELS: [u32; 2] = [650, 730];

/// Adreno model number from a wgpu adapter name such as `Adreno (TM) 730` (suffixes like the `L` of
/// `Adreno (TM) 642L` are ignored). `None` for any other GPU.
pub fn adreno_model(adapter_name: &str) -> Option<u32> {
    let digits: String = adapter_name
        .strip_prefix("Adreno (TM) ")?
        .trim()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// True when the adapter is one of the [`AFFECTED_ADRENO_MODELS`].
pub fn is_affected_adreno(adapter_name: &str) -> bool {
    adreno_model(adapter_name).is_some_and(|model| AFFECTED_ADRENO_MODELS.contains(&model))
}

/// Upper bounds that make Bevy see a device without storage buffers or compute, like its WebGL2 mode.
///
/// Applied with `or_worse_values_from`: `max` limits become the minimum of the adapter and these
/// values, `min` limits the maximum. Everything not being forced is therefore set to a neutral value
/// so the adapter's real capabilities still apply.
pub fn uniform_skinning_limits() -> WgpuLimits {
    WgpuLimits {
        // Forced: no storage buffers => skins (and morph targets) use uniform buffers.
        max_storage_buffers_per_shader_stage: 0,
        max_storage_textures_per_shader_stage: 0,
        // No storage buffers also rules out Bevy's compute passes; zero these so Bevy does not try them.
        max_compute_workgroup_storage_size: 0,
        max_compute_invocations_per_workgroup: 0,
        max_compute_workgroup_size_x: 0,
        max_compute_workgroup_size_y: 0,
        max_compute_workgroup_size_z: 0,
        max_compute_workgroups_per_dimension: 0,
        // Neutral: keep whatever the adapter offers.
        min_uniform_buffer_offset_alignment: 1,
        min_storage_buffer_offset_alignment: 1,
        max_immediate_size: u32::MAX,
        max_binding_array_elements_per_shader_stage: u32::MAX,
        max_binding_array_acceleration_structure_elements_per_shader_stage: u32::MAX,
        max_binding_array_sampler_elements_per_shader_stage: u32::MAX,
        ..WgpuLimits::default()
    }
}

/// Renderer settings for this device, with the Adreno skinning workaround when it applies.
pub fn render_settings() -> WgpuSettings {
    WgpuSettings {
        constrained_limits: needs_uniform_skinning().then(uniform_skinning_limits),
        ..WgpuSettings::default()
    }
}

/// Reads the Vulkan adapter names before the renderer starts (Bevy only decides the device limits once,
/// at startup, so this has to happen first).
#[cfg(target_os = "android")]
fn needs_uniform_skinning() -> bool {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::VULKAN;
    let instance = wgpu::Instance::new(descriptor);
    bevy::tasks::block_on(instance.enumerate_adapters(wgpu::Backends::VULKAN))
        .iter()
        .any(|adapter| is_affected_adreno(&adapter.get_info().name))
}

#[cfg(not(target_os = "android"))]
fn needs_uniform_skinning() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_adreno_model_from_the_adapter_name() {
        assert_eq!(adreno_model("Adreno (TM) 730"), Some(730));
        assert_eq!(adreno_model("Adreno (TM) 650"), Some(650));
        assert_eq!(adreno_model("Adreno (TM) 642L"), Some(642));
        assert_eq!(adreno_model("Samsung Xclipse 540"), None);
        assert_eq!(adreno_model("Mali-G715"), None);
        assert_eq!(adreno_model("Adreno (TM) "), None);
        assert_eq!(adreno_model(""), None);
    }

    #[test]
    fn only_the_reported_adreno_models_are_gated() {
        assert!(is_affected_adreno("Adreno (TM) 730"));
        assert!(is_affected_adreno("Adreno (TM) 650"));
        // Other Adreno models are neither reported affected nor reported working: left alone.
        assert!(!is_affected_adreno("Adreno (TM) 740"));
        assert!(!is_affected_adreno("Adreno (TM) 720"));
        assert!(!is_affected_adreno("Adreno (TM) 660"));
        assert!(!is_affected_adreno("Samsung Xclipse 540"));
    }

    #[test]
    fn limits_hide_storage_buffers_and_compute() {
        let limits = uniform_skinning_limits();
        assert_eq!(limits.max_storage_buffers_per_shader_stage, 0);
        assert_eq!(limits.max_storage_textures_per_shader_stage, 0);
        assert_eq!(limits.max_compute_workgroup_size_x, 0);
        assert_eq!(limits.max_compute_workgroup_storage_size, 0);
    }

    #[test]
    fn limits_do_not_constrain_anything_else_below_the_adapter() {
        // `or_worse_values_from` keeps the adapter value where ours is neutral.
        let adapter = WgpuLimits {
            max_texture_dimension_2d: 16384,
            max_bind_groups: 8,
            min_uniform_buffer_offset_alignment: 64,
            min_storage_buffer_offset_alignment: 64,
            max_immediate_size: 128,
            max_storage_buffers_per_shader_stage: 32,
            ..WgpuLimits::default()
        };
        let merged = adapter.or_worse_values_from(&uniform_skinning_limits());
        assert_eq!(merged.max_storage_buffers_per_shader_stage, 0);
        assert_eq!(merged.min_uniform_buffer_offset_alignment, 64);
        assert_eq!(merged.min_storage_buffer_offset_alignment, 64);
        assert_eq!(merged.max_immediate_size, 128);
    }

    #[test]
    fn desktop_settings_keep_unconstrained_limits() {
        assert!(render_settings().constrained_limits.is_none());
    }
}
