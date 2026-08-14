use ash::{vk, Entry};
use jni::{objects::{JClass, JString}, JNIEnv};
use std::ffi::{CStr, CString};
use std::fmt::Write as _;

#[no_mangle]
pub extern "system" fn Java_com_example_waydroidvulkan_MainActivity_getVulkanReport(
    mut env: JNIEnv,
    _class: JClass,
) -> JString<'static> {
    let report = match inspect_vulkan() {
        Ok(report) => report,
        Err(error) => format!(
            "WAYDROID VULKAN ANALYZER\n========================\n\nERROR: {error}\n"
        ),
    };

    let value = env.new_string(report).expect("failed to create Java string");
    unsafe { JString::from_raw(value.into_raw()) }
}

fn inspect_vulkan() -> Result<String, String> {
    let mut out = String::new();
    writeln!(out, "╔══════════════════════════════════════════════╗").unwrap();
    writeln!(out, "║       WAYDROID RUST VULKAN ANALYZER         ║").unwrap();
    writeln!(out, "╚══════════════════════════════════════════════╝\n").unwrap();

    let entry = unsafe { Entry::load() }
        .map_err(|e| format!("cannot load libvulkan.so: {e}"))?;

    let loader_version = unsafe {
        entry
            .try_enumerate_instance_version()
            .map_err(|e| format!("vkEnumerateInstanceVersion failed: {e:?}"))?
            .unwrap_or(vk::API_VERSION_1_0)
    };

    writeln!(out, "VULKAN LOADER").unwrap();
    writeln!(out, "--------------").unwrap();
    writeln!(out, "libvulkan.so             : FOUND").unwrap();
    writeln!(out, "Loader API               : {}", vk_version(loader_version)).unwrap();

    let instance_extensions = unsafe {
        entry
            .enumerate_instance_extension_properties(None)
            .map_err(|e| format!("instance extension enumeration failed: {e:?}"))?
    };
    writeln!(out, "Instance extensions      : {}", instance_extensions.len()).unwrap();

    let app_name = CString::new("Waydroid Vulkan Analyzer").unwrap();
    let engine_name = CString::new("Waydroid").unwrap();
    let app_info = vk::ApplicationInfo::default()
        .application_name(&app_name)
        .application_version(vk::make_api_version(0, 1, 0, 0))
        .engine_name(&engine_name)
        .engine_version(vk::make_api_version(0, 1, 0, 0))
        .api_version(loader_version);
    let instance_info = vk::InstanceCreateInfo::default().application_info(&app_info);

    let instance = unsafe { entry.create_instance(&instance_info, None) }
        .map_err(|e| format!("vkCreateInstance failed: {e:?}"))?;

    let devices = unsafe { instance.enumerate_physical_devices() }
        .map_err(|e| format!("vkEnumeratePhysicalDevices failed: {e:?}"))?;

    writeln!(out, "Physical devices         : {}\n", devices.len()).unwrap();

    if devices.is_empty() {
        unsafe { instance.destroy_instance(None); }
        return Err("Vulkan loader is present, but no physical device was exposed.".into());
    }

    for (index, device) in devices.iter().enumerate() {
        inspect_device(&instance, *device, index, &mut out)?;
    }

    unsafe { instance.destroy_instance(None); }
    Ok(out)
}

fn inspect_device(
    instance: &ash::Instance,
    device: vk::PhysicalDevice,
    index: usize,
    out: &mut String,
) -> Result<(), String> {
    let properties = unsafe { instance.get_physical_device_properties(device) };
    let features = unsafe { instance.get_physical_device_features(device) };
    let memory = unsafe { instance.get_physical_device_memory_properties(device) };
    let queues = unsafe { instance.get_physical_device_queue_family_properties(device) };
    let extensions = unsafe { instance.enumerate_device_extension_properties(device) }
        .map_err(|e| format!("device extension enumeration failed: {e:?}"))?;

    let name = unsafe { CStr::from_ptr(properties.device_name.as_ptr()) }.to_string_lossy();

    writeln!(out, "══════════════════════════════════════════════").unwrap();
    writeln!(out, "GPU #{index}").unwrap();
    writeln!(out, "══════════════════════════════════════════════").unwrap();
    writeln!(out, "Name                     : {name}").unwrap();
    writeln!(out, "Vendor ID                : 0x{:04x}", properties.vendor_id).unwrap();
    writeln!(out, "Device ID                : 0x{:04x}", properties.device_id).unwrap();
    writeln!(out, "Device Type              : {:?}", properties.device_type).unwrap();
    writeln!(out, "API Version              : {}", vk_version(properties.api_version)).unwrap();
    writeln!(out, "Driver Version           : {}", properties.driver_version).unwrap();

    let l = properties.limits;
    writeln!(out, "\nGAMING LIMITS").unwrap();
    writeln!(out, "maxImageDimension2D      : {}", l.max_image_dimension2_d).unwrap();
    writeln!(out, "maxImageDimension3D      : {}", l.max_image_dimension3_d).unwrap();
    writeln!(out, "maxImageArrayLayers      : {}", l.max_image_array_layers).unwrap();
    writeln!(out, "maxUniformBufferRange    : {}", l.max_uniform_buffer_range).unwrap();
    writeln!(out, "maxStorageBufferRange    : {}", l.max_storage_buffer_range).unwrap();
    writeln!(out, "maxPushConstantsSize     : {}", l.max_push_constants_size).unwrap();
    writeln!(out, "maxBoundDescriptorSets   : {}", l.max_bound_descriptor_sets).unwrap();
    writeln!(out, "maxColorAttachments      : {}", l.max_color_attachments).unwrap();
    writeln!(out, "maxComputeWorkGroupCount : {}, {}, {}", l.max_compute_work_group_count[0], l.max_compute_work_group_count[1], l.max_compute_work_group_count[2]).unwrap();
    writeln!(out, "maxComputeWorkGroupSize  : {}, {}, {}", l.max_compute_work_group_size[0], l.max_compute_work_group_size[1], l.max_compute_work_group_size[2]).unwrap();
    writeln!(out, "maxComputeInvocations    : {}", l.max_compute_work_group_invocations).unwrap();
    writeln!(out, "maxFramebufferWidth      : {}", l.max_framebuffer_width).unwrap();
    writeln!(out, "maxFramebufferHeight     : {}", l.max_framebuffer_height).unwrap();
    writeln!(out, "maxColorAttachments      : {}", l.max_color_attachments).unwrap();
    writeln!(out, "timestampPeriod          : {}", l.timestamp_period).unwrap();

    writeln!(out, "\nCORE FEATURES").unwrap();
    feature(out, "geometryShader", features.geometry_shader);
    feature(out, "tessellationShader", features.tessellation_shader);
    feature(out, "multiDrawIndirect", features.multi_draw_indirect);
    feature(out, "wideLines", features.wide_lines);
    feature(out, "largePoints", features.large_points);
    feature(out, "samplerAnisotropy", features.sampler_anisotropy);
    feature(out, "textureCompressionETC2", features.texture_compression_etc2);
    feature(out, "textureCompressionASTC_LDR", features.texture_compression_astc_ldr);
    feature(out, "textureCompressionBC", features.texture_compression_bc);
    feature(out, "vertexPipelineStoresAndAtomics", features.vertex_pipeline_stores_and_atomics);
    feature(out, "fragmentStoresAndAtomics", features.fragment_stores_and_atomics);
    feature(out, "shaderInt64", features.shader_int64);
    feature(out, "shaderFloat64", features.shader_float64);
    feature(out, "shaderInt16", features.shader_int16);

    writeln!(out, "\nMEMORY HEAPS").unwrap();
    let mut device_local_mb = 0u64;
    for i in 0..memory.memory_heap_count {
        let heap = memory.memory_heaps[i as usize];
        let size_mb = heap.size / 1024 / 1024;
        if heap.flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL) { device_local_mb += size_mb; }
        writeln!(out, "Heap #{i}                  : {size_mb} MB flags=0x{:x}", heap.flags.as_raw()).unwrap();
    }
    writeln!(out, "Device-local total        : {device_local_mb} MB").unwrap();

    writeln!(out, "\nQUEUE FAMILIES").unwrap();
    for (i, q) in queues.iter().enumerate() {
        writeln!(out, "Queue #{i}                  : count={} flags=0x{:x}", q.queue_count, q.queue_flags.as_raw()).unwrap();
        writeln!(out, "  Graphics                : {}", yes(q.queue_flags.contains(vk::QueueFlags::GRAPHICS))).unwrap();
        writeln!(out, "  Compute                 : {}", yes(q.queue_flags.contains(vk::QueueFlags::COMPUTE))).unwrap();
        writeln!(out, "  Transfer                : {}", yes(q.queue_flags.contains(vk::QueueFlags::TRANSFER))).unwrap();
    }

    writeln!(out, "\nDEVICE EXTENSIONS ({})", extensions.len()).unwrap();
    for e in extensions {
        let name = unsafe { CStr::from_ptr(e.extension_name.as_ptr()) }.to_string_lossy();
        writeln!(out, "  {name}").unwrap();
    }

    let has = |needle: &str| {
        extensions.iter().any(|e| {
            unsafe { CStr::from_ptr(e.extension_name.as_ptr()) }
                .to_string_lossy() == needle
        })
    };

    writeln!(out, "\nGAMING CHECKS").unwrap();
    check(out, "Graphics queue", queues.iter().any(|q| q.queue_flags.contains(vk::QueueFlags::GRAPHICS)));
    check(out, "Compute queue", queues.iter().any(|q| q.queue_flags.contains(vk::QueueFlags::COMPUTE)));
    check(out, "Transfer queue", queues.iter().any(|q| q.queue_flags.contains(vk::QueueFlags::TRANSFER)));
    check(out, "Anisotropic filtering", features.sampler_anisotropy != 0);
    check(out, "BC texture compression", features.texture_compression_bc != 0);
    check(out, "ETC2 texture compression", features.texture_compression_etc2 != 0);
    check(out, "ASTC LDR texture compression", features.texture_compression_astc_ldr != 0);
    check(out, "VK_KHR_swapchain", has("VK_KHR_swapchain"));
    check(out, "VK_EXT_memory_budget", has("VK_EXT_memory_budget"));
    check(out, "VK_KHR_dynamic_rendering", has("VK_KHR_dynamic_rendering"));
    check(out, "VK_KHR_synchronization2", has("VK_KHR_synchronization2"));
    check(out, "VK_KHR_timeline_semaphore", has("VK_KHR_timeline_semaphore"));
    check(out, "VK_KHR_buffer_device_address", has("VK_KHR_buffer_device_address"));
    check(out, "VK_EXT_descriptor_indexing", has("VK_EXT_descriptor_indexing"));
    check(out, "VK_EXT_mesh_shader", has("VK_EXT_mesh_shader"));
    check(out, "VK_KHR_ray_tracing_pipeline", has("VK_KHR_ray_tracing_pipeline"));

    Ok(())
}

fn feature(out: &mut String, name: &str, value: vk::Bool32) {
    writeln!(out, "{name:<32}: {}", yes(value != 0)).unwrap();
}

fn check(out: &mut String, name: &str, value: bool) {
    writeln!(out, "{name:<32}: {}", yes(value)).unwrap();
}

fn yes(value: bool) -> &'static str {
    if value { "YES ✓" } else { "NO ✗" }
}

fn vk_version(version: u32) -> String {
    format!(
        "{}.{}.{}",
        vk::api_version_major(version),
        vk::api_version_minor(version),
        vk::api_version_patch(version)
    )
}
