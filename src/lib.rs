use ash::{vk, Entry};
use jni::{
    objects::{JClass, JString},
    JNIEnv,
};
use std::{
    ffi::{CStr, CString},
    fmt::Write as _,
};

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_waydroidvulkan_MainActivity_getVulkanReport<'local>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> JString<'local> {
    let report = match inspect_vulkan() {
        Ok(report) => report,
        Err(error) => format!(
            "╔══════════════════════════════════════════════╗\n\
             ║       WAYDROID RUST VULKAN ANALYZER       ║\n\
             ╚══════════════════════════════════════════════╝\n\n\
             FATAL ERROR\n\
             -----------\n\
             {error}\n"
        ),
    };

    env.new_string(report)
        .expect("failed to create Java string")
}

fn inspect_vulkan() -> Result<String, String> {
    let mut out = String::new();

    writeln!(
        out,
        "╔══════════════════════════════════════════════╗"
    )
    .unwrap();
    writeln!(
        out,
        "║       WAYDROID RUST VULKAN ANALYZER         ║"
    )
    .unwrap();
    writeln!(
        out,
        "╚══════════════════════════════════════════════╝\n"
    )
    .unwrap();

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
    writeln!(
        out,
        "Loader API               : {}",
        vk_version(loader_version)
    )
    .unwrap();

    let instance_extensions = unsafe {
        entry
            .enumerate_instance_extension_properties(None)
            .map_err(|e| format!("instance extension enumeration failed: {e:?}"))?
    };

    writeln!(
        out,
        "Instance extensions      : {}",
        instance_extensions.len()
    )
    .unwrap();

    let app_name = CString::new("Waydroid Vulkan Analyzer").unwrap();
    let engine_name = CString::new("Waydroid").unwrap();

    /*
     * Do not request a Vulkan version newer than what the loader advertises.
     */
    let app_info = vk::ApplicationInfo::default()
        .application_name(&app_name)
        .application_version(vk::make_api_version(0, 1, 0, 0))
        .engine_name(&engine_name)
        .engine_version(vk::make_api_version(0, 1, 0, 0))
        .api_version(loader_version);

    let instance_info = vk::InstanceCreateInfo::default()
        .application_info(&app_info);

    let instance = unsafe { entry.create_instance(&instance_info, None) }
        .map_err(|e| format!("vkCreateInstance failed: {e:?}"))?;

    let devices = unsafe { instance.enumerate_physical_devices() }
        .map_err(|e| format!("vkEnumeratePhysicalDevices failed: {e:?}"))?;

    writeln!(out, "Physical devices         : {}\n", devices.len()).unwrap();

    if devices.is_empty() {
        unsafe {
            instance.destroy_instance(None);
        }

        return Err(
            "Vulkan loader is present, but no physical device was exposed."
                .to_string(),
        );
    }

    for (index, device) in devices.iter().enumerate() {
        inspect_device(&instance, *device, index, &mut out)?;
    }

    unsafe {
        instance.destroy_instance(None);
    }

    Ok(out)
}

fn inspect_device(
    instance: &ash::Instance,
    device: vk::PhysicalDevice,
    index: usize,
    out: &mut String,
) -> Result<(), String> {
    let properties = unsafe {
        instance.get_physical_device_properties(device)
    };

    let features = unsafe {
        instance.get_physical_device_features(device)
    };

    let memory = unsafe {
        instance.get_physical_device_memory_properties(device)
    };

    let queues = unsafe {
        instance.get_physical_device_queue_family_properties(device)
    };

    let extensions = unsafe {
        instance
            .enumerate_device_extension_properties(device)
            .map_err(|e| {
                format!(
                    "device extension enumeration failed: {e:?}"
                )
            })?
    };

    let name = cstr(properties.device_name.as_ptr());

    writeln!(
        out,
        "══════════════════════════════════════════════"
    )
    .unwrap();

    writeln!(out, "GPU #{index}").unwrap();

    writeln!(
        out,
        "══════════════════════════════════════════════"
    )
    .unwrap();

    writeln!(out, "Name                     : {name}").unwrap();

    writeln!(
        out,
        "Vendor ID                : 0x{:04x}",
        properties.vendor_id
    )
    .unwrap();

    writeln!(
        out,
        "Device ID                : 0x{:04x}",
        properties.device_id
    )
    .unwrap();

    writeln!(
        out,
        "Device Type              : {:?}",
        properties.device_type
    )
    .unwrap();

    writeln!(
        out,
        "API Version              : {}",
        vk_version(properties.api_version)
    )
    .unwrap();

    writeln!(
        out,
        "Driver Version           : {}",
        properties.driver_version
    )
    .unwrap();

    /*
     * ------------------------------------------------------------
     * DRIVER / EXTENDED PROPERTIES
     * ------------------------------------------------------------
     */

    let has_driver_properties =
        has_extension(
            &extensions,
            "VK_KHR_driver_properties",
        );

    writeln!(out, "\nDRIVER PROPERTIES").unwrap();
    writeln!(out, "-----------------").unwrap();

    if has_driver_properties {
        let mut driver_properties =
            vk::PhysicalDeviceDriverProperties::default();

        let mut properties2 =
            vk::PhysicalDeviceProperties2::default()
                .push_next(&mut driver_properties);

        unsafe {
            instance.get_physical_device_properties2(
                device,
                &mut properties2,
            );
        }

        writeln!(
            out,
            "Driver ID                : {:?}",
            driver_properties.driver_id
        )
        .unwrap();

        writeln!(
            out,
            "Driver Name              : {}",
            cstr(driver_properties.driver_name.as_ptr())
        )
        .unwrap();

        writeln!(
            out,
            "Driver Info              : {}",
            cstr(driver_properties.driver_info.as_ptr())
        )
        .unwrap();

        writeln!(
            out,
            "Conformance              : {}.{}.{}",
            driver_properties.conformance_version.major,
            driver_properties.conformance_version.minor,
            driver_properties.conformance_version.subminor
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "VK_KHR_driver_properties  : NOT AVAILABLE"
        )
        .unwrap();
    }

    /*
     * ------------------------------------------------------------
     * GAMING LIMITS
     * ------------------------------------------------------------
     */

    let l = properties.limits;

    writeln!(out, "\nGAMING LIMITS").unwrap();
    writeln!(out, "-------------").unwrap();

    writeln!(
        out,
        "maxImageDimension2D      : {}",
        l.max_image_dimension2_d
    )
    .unwrap();

    writeln!(
        out,
        "maxImageDimension3D      : {}",
        l.max_image_dimension3_d
    )
    .unwrap();

    writeln!(
        out,
        "maxImageArrayLayers      : {}",
        l.max_image_array_layers
    )
    .unwrap();

    writeln!(
        out,
        "maxUniformBufferRange    : {}",
        l.max_uniform_buffer_range
    )
    .unwrap();

    writeln!(
        out,
        "maxStorageBufferRange    : {}",
        l.max_storage_buffer_range
    )
    .unwrap();

    writeln!(
        out,
        "maxPushConstantsSize     : {}",
        l.max_push_constants_size
    )
    .unwrap();

    writeln!(
        out,
        "maxBoundDescriptorSets   : {}",
        l.max_bound_descriptor_sets
    )
    .unwrap();

    writeln!(
        out,
        "maxColorAttachments      : {}",
        l.max_color_attachments
    )
    .unwrap();

    writeln!(
        out,
        "maxComputeWorkGroupCount : {}, {}, {}",
        l.max_compute_work_group_count[0],
        l.max_compute_work_group_count[1],
        l.max_compute_work_group_count[2]
    )
    .unwrap();

    writeln!(
        out,
        "maxComputeWorkGroupSize  : {}, {}, {}",
        l.max_compute_work_group_size[0],
        l.max_compute_work_group_size[1],
        l.max_compute_work_group_size[2]
    )
    .unwrap();

    writeln!(
        out,
        "maxComputeInvocations    : {}",
        l.max_compute_work_group_invocations
    )
    .unwrap();

    writeln!(
        out,
        "maxFramebufferWidth      : {}",
        l.max_framebuffer_width
    )
    .unwrap();

    writeln!(
        out,
        "maxFramebufferHeight     : {}",
        l.max_framebuffer_height
    )
    .unwrap();

    writeln!(
        out,
        "timestampPeriod          : {}",
        l.timestamp_period
    )
    .unwrap();

    /*
     * ------------------------------------------------------------
     * CORE FEATURES
     * ------------------------------------------------------------
     */

    writeln!(out, "\nCORE FEATURES").unwrap();
    writeln!(out, "-------------").unwrap();

    feature(
        out,
        "geometryShader",
        features.geometry_shader,
    );

    feature(
        out,
        "tessellationShader",
        features.tessellation_shader,
    );

    feature(
        out,
        "multiDrawIndirect",
        features.multi_draw_indirect,
    );

    feature(
        out,
        "wideLines",
        features.wide_lines,
    );

    feature(
        out,
        "largePoints",
        features.large_points,
    );

    feature(
        out,
        "samplerAnisotropy",
        features.sampler_anisotropy,
    );

    feature(
        out,
        "textureCompressionETC2",
        features.texture_compression_etc2,
    );

    feature(
        out,
        "textureCompressionASTC_LDR",
        features.texture_compression_astc_ldr,
    );

    feature(
        out,
        "textureCompressionBC",
        features.texture_compression_bc,
    );

    feature(
        out,
        "vertexPipelineStoresAndAtomics",
        features.vertex_pipeline_stores_and_atomics,
    );

    feature(
        out,
        "fragmentStoresAndAtomics",
        features.fragment_stores_and_atomics,
    );

    feature(
        out,
        "shaderInt64",
        features.shader_int64,
    );

    feature(
        out,
        "shaderFloat64",
        features.shader_float64,
    );

    feature(
        out,
        "shaderInt16",
        features.shader_int16,
    );

    /*
     * ------------------------------------------------------------
     * VULKAN 1.1 / 1.2 / 1.3 / 1.4 FEATURES
     * ------------------------------------------------------------
     */

    let api_major =
        vk::api_version_major(properties.api_version);

    let api_minor =
        vk::api_version_minor(properties.api_version);

    writeln!(out, "\nVULKAN VERSION FEATURES").unwrap();
    writeln!(out, "-----------------------").unwrap();

    writeln!(
        out,
        "Vulkan 1.1 support       : {}",
        yes(api_major > 1 || (api_major == 1 && api_minor >= 1))
    )
    .unwrap();

    writeln!(
        out,
        "Vulkan 1.2 support       : {}",
        yes(api_major > 1 || (api_major == 1 && api_minor >= 2))
    )
    .unwrap();

    writeln!(
        out,
        "Vulkan 1.3 support       : {}",
        yes(api_major > 1 || (api_major == 1 && api_minor >= 3))
    )
    .unwrap();

    /*
     * ash 0.38.0 is generated against Vulkan 1.3.
     *
     * The loader/device can still report Vulkan 1.4, but this
     * particular ash release does not expose
     * PhysicalDeviceVulkan14Features.
     *
     * Therefore Vulkan 1.4 capability is determined from the
     * device API version and the corresponding extension set,
     * while the Vulkan 1.4 feature section below remains present.
     */

    let vulkan14_api =
        api_major > 1
            || (api_major == 1 && api_minor >= 4);

    let vulkan14_global_priority_query =
        has_extension(
            &extensions,
            "VK_KHR_global_priority",
        )
        && has_extension(
            &extensions,
            "VK_EXT_global_priority_query",
        );

    let vulkan14_shader_subgroup_rotate =
        has_extension(
            &extensions,
            "VK_KHR_shader_subgroup_rotate",
        );

    let vulkan14_shader_float_controls2 =
        has_extension(
            &extensions,
            "VK_KHR_shader_float_controls2",
        );

    let vulkan14_shader_expect_assume =
        has_extension(
            &extensions,
            "VK_KHR_shader_expect_assume",
        );

    let vulkan14_dynamic_rendering_local_read =
        has_extension(
            &extensions,
            "VK_KHR_dynamic_rendering_local_read",
        );

    let vulkan14_maintenance5 =
        has_extension(
            &extensions,
            "VK_KHR_maintenance5",
        );

    let vulkan14_maintenance6 =
        has_extension(
            &extensions,
            "VK_KHR_maintenance6",
        );

    let vulkan14_pipeline_protected_access =
        has_extension(
            &extensions,
            "VK_EXT_pipeline_protected_access",
        );

    let vulkan14_pipeline_robustness =
        has_extension(
            &extensions,
            "VK_EXT_pipeline_robustness",
        );

    let vulkan14_host_image_copy =
        has_extension(
            &extensions,
            "VK_EXT_host_image_copy",
        );

    let vulkan14_push_descriptor =
        has_extension(
            &extensions,
            "VK_KHR_push_descriptor",
        );

    writeln!(
        out,
        "Vulkan 1.4 support       : {}",
        yes(vulkan14_api)
    )
    .unwrap();

    /*
     * ------------------------------------------------------------
     * QUERY PROMOTED VULKAN 1.1 / 1.2 / 1.3 FEATURES
     * ------------------------------------------------------------
     */

    let mut vulkan11 =
        vk::PhysicalDeviceVulkan11Features::default();

    let mut vulkan12 =
        vk::PhysicalDeviceVulkan12Features::default();

    let mut vulkan13 =
        vk::PhysicalDeviceVulkan13Features::default();

    let mut features2 =
        vk::PhysicalDeviceFeatures2::default()
            .push_next(&mut vulkan11)
            .push_next(&mut vulkan12)
            .push_next(&mut vulkan13);

    unsafe {
        instance.get_physical_device_features2(
            device,
            &mut features2,
        );
    }

    /*
     * ------------------------------------------------------------
     * VULKAN 1.1
     * ------------------------------------------------------------
     */

    writeln!(out, "\nVULKAN 1.1 FEATURES").unwrap();
    writeln!(out, "-------------------").unwrap();

    feature(
        out,
        "storageBuffer16BitAccess",
        vulkan11.storage_buffer16_bit_access,
    );

    feature(
        out,
        "uniformAndStorageBuffer16BitAccess",
        vulkan11.uniform_and_storage_buffer16_bit_access,
    );

    feature(
        out,
        "multiview",
        vulkan11.multiview,
    );

    feature(
        out,
        "variablePointersStorageBuffer",
        vulkan11.variable_pointers_storage_buffer,
    );

    feature(
        out,
        "protectedMemory",
        vulkan11.protected_memory,
    );

    feature(
        out,
        "samplerYcbcrConversion",
        vulkan11.sampler_ycbcr_conversion,
    );

    /*
     * ------------------------------------------------------------
     * VULKAN 1.2
     * ------------------------------------------------------------
     */

    writeln!(out, "\nVULKAN 1.2 FEATURES").unwrap();
    writeln!(out, "-------------------").unwrap();

    feature(
        out,
        "samplerMirrorClampToEdge",
        vulkan12.sampler_mirror_clamp_to_edge,
    );

    feature(
        out,
        "drawIndirectCount",
        vulkan12.draw_indirect_count,
    );

    feature(
        out,
        "storageBuffer8BitAccess",
        vulkan12.storage_buffer8_bit_access,
    );

    feature(
        out,
        "uniformAndStorageBuffer8BitAccess",
        vulkan12.uniform_and_storage_buffer8_bit_access,
    );

    feature(
        out,
        "shaderFloat16",
        vulkan12.shader_float16,
    );

    feature(
        out,
        "shaderInt8",
        vulkan12.shader_int8,
    );

    feature(
        out,
        "descriptorIndexing",
        vulkan12.descriptor_indexing,
    );

    feature(
        out,
        "runtimeDescriptorArray",
        vulkan12.runtime_descriptor_array,
    );

    feature(
        out,
        "descriptorBindingPartiallyBound",
        vulkan12.descriptor_binding_partially_bound,
    );

    feature(
        out,
        "descriptorBindingVariableDescriptorCount",
        vulkan12.descriptor_binding_variable_descriptor_count,
    );

    feature(
        out,
        "bufferDeviceAddress",
        vulkan12.buffer_device_address,
    );

    feature(
        out,
        "bufferDeviceAddressCaptureReplay",
        vulkan12.buffer_device_address_capture_replay,
    );

    feature(
        out,
        "timelineSemaphore",
        vulkan12.timeline_semaphore,
    );

    feature(
        out,
        "vulkanMemoryModel",
        vulkan12.vulkan_memory_model,
    );

    feature(
        out,
        "shaderOutputViewportIndex",
        vulkan12.shader_output_viewport_index,
    );

    feature(
        out,
        "shaderOutputLayer",
        vulkan12.shader_output_layer,
    );

    /*
     * ------------------------------------------------------------
     * VULKAN 1.3
     * ------------------------------------------------------------
     */

    writeln!(out, "\nVULKAN 1.3 FEATURES").unwrap();
    writeln!(out, "-------------------").unwrap();

    feature(
        out,
        "robustImageAccess",
        vulkan13.robust_image_access,
    );

    feature(
        out,
        "inlineUniformBlock",
        vulkan13.inline_uniform_block,
    );

    /*
     * FIX:
     *
     * ash 0.38 calls this field:
     *
     * descriptor_binding_inline_uniform_block_update_after_bind
     *
     * The old name descriptor_binding_inline_uniform_block
     * does not exist.
     */

    feature(
        out,
        "descriptorBindingInlineUniformBlockUpdateAfterBind",
        vulkan13
            .descriptor_binding_inline_uniform_block_update_after_bind,
    );

    feature(
        out,
        "pipelineCreationCacheControl",
        vulkan13.pipeline_creation_cache_control,
    );

    feature(
        out,
        "privateData",
        vulkan13.private_data,
    );

    feature(
        out,
        "shaderDemoteToHelperInvocation",
        vulkan13.shader_demote_to_helper_invocation,
    );

    feature(
        out,
        "shaderTerminateInvocation",
        vulkan13.shader_terminate_invocation,
    );

    feature(
        out,
        "subgroupSizeControl",
        vulkan13.subgroup_size_control,
    );

    feature(
        out,
        "computeFullSubgroups",
        vulkan13.compute_full_subgroups,
    );

    feature(
        out,
        "synchronization2",
        vulkan13.synchronization2,
    );

    feature(
        out,
        "textureCompressionASTC_HDR",
        vulkan13.texture_compression_astc_hdr,
    );

    feature(
        out,
        "dynamicRendering",
        vulkan13.dynamic_rendering,
    );

    feature(
        out,
        "maintenance4",
        vulkan13.maintenance4,
    );

    /*
     * ------------------------------------------------------------
     * VULKAN 1.4
     * ------------------------------------------------------------
     *
     * ash 0.38 does not contain PhysicalDeviceVulkan14Features.
     * Keep the complete report section, but use the promoted
     * extension names to determine whether the corresponding
     * Vulkan 1.4 capability is exposed.
     */

    writeln!(out, "\nVULKAN 1.4 FEATURES").unwrap();
    writeln!(out, "-------------------").unwrap();

    feature(
        out,
        "globalPriorityQuery",
        if vulkan14_api {
            vulkan14_global_priority_query
                || has_extension(
                    &extensions,
                    "VK_KHR_global_priority",
                )
        } else {
            false
        } as vk::Bool32,
    );

    feature(
        out,
        "shaderSubgroupRotate",
        vulkan14_shader_subgroup_rotate as vk::Bool32,
    );

    feature(
        out,
        "shaderSubgroupRotateClustered",
        vulkan14_shader_subgroup_rotate as vk::Bool32,
    );

    feature(
        out,
        "shaderFloatControls2",
        vulkan14_shader_float_controls2 as vk::Bool32,
    );

    feature(
        out,
        "shaderExpectAssume",
        vulkan14_shader_expect_assume as vk::Bool32,
    );

    feature(
        out,
        "rectangularLines",
        has_extension(
            &extensions,
            "VK_KHR_line_rasterization",
        ) as vk::Bool32,
    );

    feature(
        out,
        "bresenhamLines",
        has_extension(
            &extensions,
            "VK_KHR_line_rasterization",
        ) as vk::Bool32,
    );

    feature(
        out,
        "smoothLines",
        has_extension(
            &extensions,
            "VK_KHR_line_rasterization",
        ) as vk::Bool32,
    );

    feature(
        out,
        "stippledRectangularLines",
        has_extension(
            &extensions,
            "VK_KHR_line_rasterization",
        ) as vk::Bool32,
    );

    feature(
        out,
        "stippledBresenhamLines",
        has_extension(
            &extensions,
            "VK_KHR_line_rasterization",
        ) as vk::Bool32,
    );

    feature(
        out,
        "stippledSmoothLines",
        has_extension(
            &extensions,
            "VK_KHR_line_rasterization",
        ) as vk::Bool32,
    );

    feature(
        out,
        "vertexAttributeInstanceRateDivisor",
        has_extension(
            &extensions,
            "VK_KHR_vertex_attribute_divisor",
        ) as vk::Bool32,
    );

    feature(
        out,
        "vertexAttributeInstanceRateZeroDivisor",
        has_extension(
            &extensions,
            "VK_KHR_vertex_attribute_divisor",
        ) as vk::Bool32,
    );

    feature(
        out,
        "indexTypeUint8",
        has_extension(
            &extensions,
            "VK_KHR_index_type_uint8",
        ) as vk::Bool32,
    );

    feature(
        out,
        "dynamicRenderingLocalRead",
        vulkan14_dynamic_rendering_local_read as vk::Bool32,
    );

    feature(
        out,
        "maintenance5",
        vulkan14_maintenance5 as vk::Bool32,
    );

    feature(
        out,
        "maintenance6",
        vulkan14_maintenance6 as vk::Bool32,
    );

    feature(
        out,
        "pipelineProtectedAccess",
        vulkan14_pipeline_protected_access as vk::Bool32,
    );

    feature(
        out,
        "pipelineRobustness",
        vulkan14_pipeline_robustness as vk::Bool32,
    );

    feature(
        out,
        "hostImageCopy",
        vulkan14_host_image_copy as vk::Bool32,
    );

    feature(
        out,
        "pushDescriptor",
        vulkan14_push_descriptor as vk::Bool32,
    );

    /*
     * ------------------------------------------------------------
     * MEMORY
     * ------------------------------------------------------------
     */

    writeln!(out, "\nMEMORY HEAPS").unwrap();
    writeln!(out, "------------").unwrap();

    let mut device_local_mb = 0u64;

    for i in 0..memory.memory_heap_count {
        let heap = memory.memory_heaps[i as usize];

        let size_mb =
            heap.size / 1024 / 1024;

        if heap
            .flags
            .contains(vk::MemoryHeapFlags::DEVICE_LOCAL)
        {
            device_local_mb += size_mb;
        }

        writeln!(
            out,
            "Heap #{i}                  : {} MB flags=0x{:x}",
            size_mb,
            heap.flags.as_raw()
        )
        .unwrap();
    }

    writeln!(
        out,
        "Device-local total        : {device_local_mb} MB"
    )
    .unwrap();

    /*
     * Memory budget.
     */

    if has_extension(
        &extensions,
        "VK_EXT_memory_budget",
    ) {
        let mut budget =
            vk::PhysicalDeviceMemoryBudgetPropertiesEXT::default();

        let mut memory2 =
            vk::PhysicalDeviceMemoryProperties2::default()
                .push_next(&mut budget);

        unsafe {
            instance.get_physical_device_memory_properties2(
                device,
                &mut memory2,
            );
        }

        writeln!(
            out,
            "\nMEMORY BUDGET (VK_EXT_memory_budget)"
        )
        .unwrap();

        writeln!(
            out,
            "-------------------------------------"
        )
        .unwrap();

        for i in 0..memory2
            .memory_properties
            .memory_heap_count
        {
            let budget_mb =
                budget.heap_budget[i as usize]
                    / 1024
                    / 1024;

            let usage_mb =
                budget.heap_usage[i as usize]
                    / 1024
                    / 1024;

            writeln!(
                out,
                "Heap #{i:<20}: budget={} MB usage={} MB",
                budget_mb,
                usage_mb
            )
            .unwrap();
        }
    } else {
        writeln!(
            out,
            "\nMEMORY BUDGET               : NOT AVAILABLE"
        )
        .unwrap();
    }

    /*
     * ------------------------------------------------------------
     * QUEUE FAMILIES
     * ------------------------------------------------------------
     */

    writeln!(out, "\nQUEUE FAMILIES").unwrap();
    writeln!(out, "--------------").unwrap();

    for (i, q) in queues.iter().enumerate() {
        writeln!(
            out,
            "Queue #{i}                  : count={} flags=0x{:x}",
            q.queue_count,
            q.queue_flags.as_raw()
        )
        .unwrap();

        writeln!(
            out,
            "  Graphics                : {}",
            yes(
                q.queue_flags
                    .contains(vk::QueueFlags::GRAPHICS)
            )
        )
        .unwrap();

        writeln!(
            out,
            "  Compute                 : {}",
            yes(
                q.queue_flags
                    .contains(vk::QueueFlags::COMPUTE)
            )
        )
        .unwrap();

        writeln!(
            out,
            "  Transfer                : {}",
            yes(
                q.queue_flags
                    .contains(vk::QueueFlags::TRANSFER)
            )
        )
        .unwrap();

        writeln!(
            out,
            "  Sparse                  : {}",
            yes(
                q.queue_flags
                    .contains(vk::QueueFlags::SPARSE_BINDING)
            )
        )
        .unwrap();
    }

    /*
     * ------------------------------------------------------------
     * SUBGROUP INFORMATION
     * ------------------------------------------------------------
     */

    if has_extension(
        &extensions,
        "VK_EXT_subgroup_size_control",
    ) || api_major > 1
        || (api_major == 1 && api_minor >= 3)
    {
        let mut subgroup =
            vk::PhysicalDeviceSubgroupProperties::default();

        let mut properties2 =
            vk::PhysicalDeviceProperties2::default()
                .push_next(&mut subgroup);

        unsafe {
            instance.get_physical_device_properties2(
                device,
                &mut properties2,
            );
        }

        writeln!(out, "\nSUBGROUP").unwrap();
        writeln!(out, "--------").unwrap();

        writeln!(
            out,
            "Size                     : {}",
            subgroup.subgroup_size
        )
        .unwrap();

        writeln!(
            out,
            "Stages                   : 0x{:x}",
            subgroup.supported_stages.as_raw()
        )
        .unwrap();

        writeln!(
            out,
            "Operations               : 0x{:x}",
            subgroup.supported_operations.as_raw()
        )
        .unwrap();

        writeln!(
            out,
            "Quad operations          : {}",
            yes(subgroup.quad_operations_in_all_stages != 0)
        )
        .unwrap();
    }

    /*
     * ------------------------------------------------------------
     * DEVICE EXTENSIONS
     * ------------------------------------------------------------
     */

    writeln!(
        out,
        "\nDEVICE EXTENSIONS ({})",
        extensions.len()
    )
    .unwrap();

    for e in &extensions {
        let name =
            cstr(e.extension_name.as_ptr());

        writeln!(out, "  {name}").unwrap();
    }

    /*
     * ------------------------------------------------------------
     * GAMING CHECKS
     * ------------------------------------------------------------
     */

    writeln!(out, "\nGAMING CHECKS").unwrap();
    writeln!(out, "-------------").unwrap();

    check(
        out,
        "Graphics queue",
        queues.iter().any(|q| {
            q.queue_flags
                .contains(vk::QueueFlags::GRAPHICS)
        }),
    );

    check(
        out,
        "Compute queue",
        queues.iter().any(|q| {
            q.queue_flags
                .contains(vk::QueueFlags::COMPUTE)
        }),
    );

    check(
        out,
        "Transfer queue",
        queues.iter().any(|q| {
            q.queue_flags
                .contains(vk::QueueFlags::TRANSFER)
        }),
    );

    check(
        out,
        "Anisotropic filtering",
        features.sampler_anisotropy != 0,
    );

    check(
        out,
        "BC texture compression",
        features.texture_compression_bc != 0,
    );

    check(
        out,
        "ETC2 texture compression",
        features.texture_compression_etc2 != 0,
    );

    check(
        out,
        "ASTC LDR texture compression",
        features.texture_compression_astc_ldr != 0,
    );

    check(
        out,
        "VK_KHR_swapchain",
        has_extension(
            &extensions,
            "VK_KHR_swapchain",
        ),
    );

    check(
        out,
        "VK_EXT_memory_budget",
        has_extension(
            &extensions,
            "VK_EXT_memory_budget",
        ),
    );

    check(
        out,
        "VK_KHR_driver_properties",
        has_extension(
            &extensions,
            "VK_KHR_driver_properties",
        ),
    );

    check(
        out,
        "VK_KHR_dynamic_rendering",
        has_extension(
            &extensions,
            "VK_KHR_dynamic_rendering",
        ),
    );

    check(
        out,
        "VK_KHR_synchronization2",
        has_extension(
            &extensions,
            "VK_KHR_synchronization2",
        ),
    );

    check(
        out,
        "VK_KHR_timeline_semaphore",
        has_extension(
            &extensions,
            "VK_KHR_timeline_semaphore",
        ),
    );

    check(
        out,
        "VK_KHR_buffer_device_address",
        has_extension(
            &extensions,
            "VK_KHR_buffer_device_address",
        ),
    );

    check(
        out,
        "VK_EXT_descriptor_indexing",
        has_extension(
            &extensions,
            "VK_EXT_descriptor_indexing",
        ),
    );

    check(
        out,
        "VK_EXT_graphics_pipeline_library",
        has_extension(
            &extensions,
            "VK_EXT_graphics_pipeline_library",
        ),
    );

    check(
        out,
        "VK_EXT_mesh_shader",
        has_extension(
            &extensions,
            "VK_EXT_mesh_shader",
        ),
    );

    check(
        out,
        "VK_KHR_ray_tracing_pipeline",
        has_extension(
            &extensions,
            "VK_KHR_ray_tracing_pipeline",
        ),
    );

    /*
     * ------------------------------------------------------------
     * ANDROID / PUBG-RELEVANT CHECKS
     * ------------------------------------------------------------
     */

    writeln!(
        out,
        "\nANDROID GAMING COMPATIBILITY"
    )
    .unwrap();

    writeln!(
        out,
        "---------------------------"
    )
    .unwrap();

    let swapchain =
        has_extension(
            &extensions,
            "VK_KHR_swapchain",
        );

    let astc =
        features.texture_compression_astc_ldr != 0;

    let etc2 =
        features.texture_compression_etc2 != 0;

    let anisotropy =
        features.sampler_anisotropy != 0;

    let graphics =
        queues.iter().any(|q| {
            q.queue_flags
                .contains(vk::QueueFlags::GRAPHICS)
        });

    let compute =
        queues.iter().any(|q| {
            q.queue_flags
                .contains(vk::QueueFlags::COMPUTE)
        });

    let dynamic_rendering =
        vulkan13.dynamic_rendering != 0
            || has_extension(
                &extensions,
                "VK_KHR_dynamic_rendering",
            );

    let synchronization2 =
        vulkan13.synchronization2 != 0
            || has_extension(
                &extensions,
                "VK_KHR_synchronization2",
            );

    let timeline =
        vulkan12.timeline_semaphore != 0
            || has_extension(
                &extensions,
                "VK_KHR_timeline_semaphore",
            );

    let descriptor_indexing =
        vulkan12.descriptor_indexing != 0
            || has_extension(
                &extensions,
                "VK_EXT_descriptor_indexing",
            );

    let buffer_device_address =
        vulkan12.buffer_device_address != 0
            || has_extension(
                &extensions,
                "VK_KHR_buffer_device_address",
            );

    check(out, "Graphics", graphics);
    check(out, "Compute", compute);
    check(out, "Swapchain", swapchain);
    check(out, "ASTC LDR", astc);
    check(out, "ETC2", etc2);
    check(
        out,
        "Anisotropic filtering",
        anisotropy,
    );
    check(
        out,
        "Dynamic rendering",
        dynamic_rendering,
    );
    check(
        out,
        "Synchronization2",
        synchronization2,
    );
    check(
        out,
        "Timeline semaphore",
        timeline,
    );
    check(
        out,
        "Descriptor indexing",
        descriptor_indexing,
    );
    check(
        out,
        "Buffer device address",
        buffer_device_address,
    );

    /*
     * ------------------------------------------------------------
     * PUBG ANALYSIS
     * ------------------------------------------------------------
     *
     * This is intentionally a capability analysis.
     *
     * We must NOT claim that PUBG will enable Vulkan solely from
     * these properties. The game may additionally use its own
     * device/driver whitelist or Android-side checks.
     */

    writeln!(out, "\nPUBG VULKAN ANALYSIS").unwrap();
    writeln!(out, "--------------------").unwrap();

    let core_ready =
        graphics
            && compute
            && swapchain
            && astc
            && etc2;

    if core_ready {
        writeln!(
            out,
            "Core Vulkan capability    : READY ✓"
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "Core Vulkan capability    : INCOMPLETE ✗"
        )
        .unwrap();
    }

    writeln!(
        out,
        "Modern Vulkan features    : {}",
        if dynamic_rendering
            && synchronization2
            && timeline
            && descriptor_indexing
            && buffer_device_address
        {
            "READY ✓"
        } else {
            "PARTIAL"
        }
    )
    .unwrap();

    writeln!(
        out,
        "Texture formats            : {}",
        if astc && etc2 {
            "ANDROID READY ✓"
        } else {
            "LIMITED"
        }
    )
    .unwrap();

    writeln!(
        out,
        "Ray tracing                : {}",
        yes(
            has_extension(
                &extensions,
                "VK_KHR_ray_tracing_pipeline",
            )
        )
    )
    .unwrap();

    writeln!(
        out,
        "Mesh shader                : {}",
        yes(
            has_extension(
                &extensions,
                "VK_EXT_mesh_shader",
            )
        )
    )
    .unwrap();

    writeln!(out, "\nPUBG RESULT").unwrap();
    writeln!(out, "-----------").unwrap();

    if core_ready {
        writeln!(
            out,
            "Vulkan hardware capability: PASS ✓"
        )
        .unwrap();

        writeln!(
            out,
            "PUBG Vulkan support       : NOT DETERMINED"
        )
        .unwrap();

        writeln!(
            out,
            "Reason                    : GPU/driver capability \
             is available, but PUBG may apply its own Android \
             device/driver whitelist or runtime checks."
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "Vulkan hardware capability: FAIL ✗"
        )
        .unwrap();

        writeln!(
            out,
            "PUBG Vulkan support       : UNLIKELY"
        )
        .unwrap();
    }

    /*
     * ------------------------------------------------------------
     * WAYDROID / RADV DIAGNOSTICS
     * ------------------------------------------------------------
     */

    writeln!(
        out,
        "\nWAYDROID / RADV DIAGNOSTICS"
    )
    .unwrap();

    writeln!(
        out,
        "---------------------------"
    )
    .unwrap();

    if properties.vendor_id == 0x1002 {
        writeln!(
            out,
            "GPU vendor                : AMD ✓"
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "GPU vendor                : non-AMD"
        )
        .unwrap();
    }

    let driver_name =
        if has_driver_properties {
            let mut driver_properties =
                vk::PhysicalDeviceDriverProperties::default();

            let mut properties2 =
                vk::PhysicalDeviceProperties2::default()
                    .push_next(&mut driver_properties);

            unsafe {
                instance.get_physical_device_properties2(
                    device,
                    &mut properties2,
                );
            }

            cstr(
                driver_properties
                    .driver_name
                    .as_ptr(),
            )
        } else {
            "unknown".to_string()
        };

    writeln!(
        out,
        "Vulkan driver             : {driver_name}"
    )
    .unwrap();

    if driver_name
        .to_ascii_lowercase()
        .contains("radv")
        || name
            .to_ascii_lowercase()
            .contains("radv")
    {
        writeln!(
            out,
            "RADV detected             : YES ✓"
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "RADV detected             : NO / UNKNOWN"
        )
        .unwrap();
    }

    writeln!(
        out,
        "Physical device exposed   : YES ✓"
    )
    .unwrap();

    writeln!(
        out,
        "Android Vulkan loader     : WORKING ✓"
    )
    .unwrap();

    /*
     * ------------------------------------------------------------
     * FINAL
     * ------------------------------------------------------------
     */

    writeln!(out, "\nFINAL").unwrap();
    writeln!(out, "-----").unwrap();

    writeln!(
        out,
        "Vulkan API                : {}",
        vk_version(properties.api_version)
    )
    .unwrap();

    writeln!(
        out,
        "GPU                       : {name}"
    )
    .unwrap();

    writeln!(
        out,
        "Device-local memory       : {} MB",
        device_local_mb
    )
    .unwrap();

    if core_ready {
        writeln!(
            out,
            "Gaming Vulkan             : READY ✓"
        )
        .unwrap();
    } else {
        writeln!(
            out,
            "Gaming Vulkan             : LIMITED ✗"
        )
        .unwrap();
    }

    writeln!(out, "\nNOTE:").unwrap();

    writeln!(
        out,
        "Vulkan capability does not guarantee that a specific"
    )
    .unwrap();

    writeln!(
        out,
        "Android game will expose or enable its Vulkan renderer."
    )
    .unwrap();

    writeln!(
        out,
        "Games may use device/driver whitelists, Android GPU"
    )
    .unwrap();

    writeln!(
        out,
        "profiles, runtime checks, or their own compatibility logic."
    )
    .unwrap();

    Ok(out)
}

/*
 * ------------------------------------------------------------
 * HELPERS
 * ------------------------------------------------------------
 */

fn cstr(ptr: *const i8) -> String {
    if ptr.is_null() {
        return "unknown".to_string();
    }

    unsafe {
        CStr::from_ptr(ptr)
    }
    .to_string_lossy()
    .into_owned()
}

fn has_extension(
    extensions: &[vk::ExtensionProperties],
    needle: &str,
) -> bool {
    extensions.iter().any(|e| {
        cstr(e.extension_name.as_ptr()) == needle
    })
}

fn feature(
    out: &mut String,
    name: &str,
    value: vk::Bool32,
) {
    writeln!(
        out,
        "{name:<36}: {}",
        yes(value != 0)
    )
    .unwrap();
}

fn check(
    out: &mut String,
    name: &str,
    value: bool,
) {
    writeln!(
        out,
        "{name:<36}: {}",
        yes(value)
    )
    .unwrap();
}

fn yes(value: bool) -> &'static str {
    if value {
        "YES ✓"
    } else {
        "NO ✗"
    }
}

fn vk_version(version: u32) -> String {
    format!(
        "{}.{}.{}",
        vk::api_version_major(version),
        vk::api_version_minor(version),
        vk::api_version_patch(version)
    )
}
