use ash::{
    extensions::khr,
    vk::{self, Handle, InstanceCreateFlags, SemaphoreCreateInfo},
};
use bevy_xr::presentation::XrGraphicsContext;
use hal::{Adapter, Device, InstanceFlags};
use openxr as xr;
use std::{
    error::Error,
    ffi::{c_void, CStr},
    os::raw::c_char,
    sync::Arc,
};
use wgpu::Limits;
use wgpu_hal as hal;
use xr::sys::platform::VkInstanceCreateInfo;

#[derive(Clone)]
pub enum GraphicsContextHandles {
    Vulkan {
        instance: ash::Instance,
        physical_device: vk::PhysicalDevice,
        device: ash::Device,
        queue_family_index: u32,
        queue_index: u32,
    },
}

#[derive(Debug, thiserror::Error)]
#[error("Error creating HAL adapter")]
pub struct AdapterError;

pub fn create_graphics_context(
    instance: &xr::Instance,
    system: xr::SystemId,
) -> Result<(GraphicsContextHandles, XrGraphicsContext), Box<dyn Error>> {
    let mut device_descriptor = wgpu::DeviceDescriptor::default();
    // device_descriptor.limits = Limits::downlevel_defaults();

    if instance.exts().khr_vulkan_enable2.is_some() {
        let vk_entry = unsafe { ash::Entry::load().unwrap() };

        // Vulkan 1.0 constrained by Oculus Go support.
        // NOTE: multiview support will require Vulkan 1.1 or specific extensions
        let vk_version = vk::make_api_version(0, 1, 1, 0);

        // todo: check requirements
        let _requirements = instance
            .graphics_requirements::<xr::Vulkan>(system)
            .unwrap();

        let vk_app_info = vk::ApplicationInfo::builder()
            .application_version(0)
            .engine_version(0)
            .api_version(vk_version);

        let mut flags = hal::InstanceFlags::empty();
        if cfg!(debug_assertions) {
            flags |= hal::InstanceFlags::VALIDATION;
            flags |= hal::InstanceFlags::DEBUG;
        }

        let mut instance_extensions =
            <hal::api::Vulkan as hal::Api>::Instance::required_extensions(&vk_entry, flags)
                .map_err(Box::new)
                .unwrap();
        instance_extensions.retain(|ext| ext != &vk::KhrGetPhysicalDeviceProperties2Fn::name());

        dbg!(&instance_extensions);
        let instance_extensions_ptrs = instance_extensions
            .iter()
            .map(|x| x.as_ptr())
            .collect::<Vec<_>>();

        let layers = [CStr::from_bytes_with_nul(b"VK_LAYER_KHRONOS_validation\0")
            .unwrap()
            .as_ptr()];
        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&vk_app_info)
            .enabled_layer_names(&layers)
            // .enabled_extension_names(&instance_extensions_ptrs)
            // .enabled_layer_names(&layers_names_raw)

            // .flags(InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR)
            ;

        let vk_instance = unsafe {
            let vk_instance = instance
                .create_vulkan_instance(
                    system,
                    std::mem::transmute(vk_entry.static_fn().get_instance_proc_addr),
                    &*create_info as *const vk::InstanceCreateInfo as *const VkInstanceCreateInfo,
                )
                .map_err(Box::new)
                .unwrap()
                .map_err(|e| Box::new(vk::Result::from_raw(e)))
                .unwrap();

            ash::Instance::load(
                vk_entry.static_fn(),
                vk::Instance::from_raw(vk_instance as _),
            )
        };
        let hal_instance = unsafe {
            <hal::api::Vulkan as hal::Api>::Instance::from_raw(
                vk_entry.clone(),
                vk_instance.clone(),
                vk_version,
                26, //TODO: is this correct?
                instance_extensions,
                flags,
                false, //TODO: is this correct?
                Some(Box::new(instance.clone())),
            )
            .map_err(Box::new)
            .unwrap()
        };

        let vk_physical_device = vk::PhysicalDevice::from_raw(
            instance
                .vulkan_graphics_device(system, vk_instance.handle().as_raw() as _)
                .map_err(Box::new)
                .unwrap() as _,
        );
        let hal_exposed_adapter = hal_instance
            .expose_adapter(vk_physical_device)
            .ok_or_else(|| Box::new(AdapterError))
            .unwrap();

        let queue_family_index = unsafe {
            vk_instance
                .get_physical_device_queue_family_properties(vk_physical_device)
                .into_iter()
                .enumerate()
                .find_map(|(queue_family_index, info)| {
                    if info.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                        Some(queue_family_index as u32)
                    } else {
                        None
                    }
                })
                .unwrap()
        };
        let queue_index = 0;

        let device_extensions = hal_exposed_adapter
            .adapter
            .required_device_extensions(device_descriptor.features)
            .into_iter()
            .filter(|ext| {
                hal_exposed_adapter
                    .adapter
                    .physical_device_capabilities()
                    .supports_extension(ext)
            })
            // .filter(|ext| ext != &khr::TimelineSemaphore::name())
            .collect::<Vec<_>>();
        dbg!(&device_extensions);
        let device_extensions_ptrs = device_extensions
            .iter()
            .map(|x| x.as_ptr())
            .collect::<Vec<_>>();

        //  TODO: how do we get limits from actual device?
        let uab_types = hal::UpdateAfterBindTypes::from_limits(
            &Default::default(),
            &hal_exposed_adapter
                .adapter
                .physical_device_capabilities()
                .properties()
                .limits,
        );
        let mut physical_features = hal_exposed_adapter.adapter.physical_device_features(
            &device_extensions,
            device_descriptor.features,
            uab_types,
        );

        dbg!(&physical_features);

        let family_info = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .queue_priorities(&[1.0])
            .build();
        let family_infos = [family_info];

        let mut multiview = vk::PhysicalDeviceMultiviewFeatures {
            multiview: vk::TRUE,
            ..Default::default()
        };
        let vk_device = {
            let info = vk::DeviceCreateInfo::builder()
                .queue_create_infos(&family_infos)
                .enabled_extension_names(&device_extensions_ptrs)
                // .enabled_layer_names(&layers)
                .push_next(&mut multiview);
            let info = physical_features.add_to_device_create_builder(info);

            unsafe {
                let vk_device = instance
                    .create_vulkan_device(
                        system,
                        std::mem::transmute(vk_entry.static_fn().get_instance_proc_addr),
                        vk_physical_device.as_raw() as _,
                        &info as *const _ as *const _,
                    )
                    .map_err(Box::new)
                    .unwrap()
                    .map_err(|e| Box::new(vk::Result::from_raw(e)))
                    .unwrap();

                ash::Device::load(vk_instance.fp_v1_0(), vk::Device::from_raw(vk_device as _))
            }
        };

        // let mut sem_type_info =
        //     vk::SemaphoreTypeCreateInfo::builder().semaphore_type(vk::SemaphoreType::TIMELINE);
        // let vk_info = vk::SemaphoreCreateInfo::builder().push_next(&mut sem_type_info);
        // dbg!("manual create sema");
        // unsafe { vk_device.create_semaphore(&vk_info, None) }.unwrap();
        // dbg!("after manual create sema");
        let hal_device = unsafe {
            hal_exposed_adapter
                .adapter
                .device_from_raw(
                    vk_device.clone(),
                    true, //    TODO: is this right?
                    &device_extensions,
                    device_descriptor.features,
                    uab_types,
                    queue_family_index,
                    queue_index,
                )
                .map_err(Box::new)
                .unwrap()
        };

        let wgpu_instance = unsafe { wgpu::Instance::from_hal::<hal::api::Vulkan>(hal_instance) };
        let wgpu_adapter = unsafe { wgpu_instance.create_adapter_from_hal(hal_exposed_adapter) };
        let (wgpu_device, wgpu_queue) = unsafe {
            wgpu_adapter
                .create_device_from_hal(hal_device, &device_descriptor, None)
                .map_err(Box::new)
                .unwrap()
        };

        Ok((
            GraphicsContextHandles::Vulkan {
                instance: vk_instance,
                physical_device: vk_physical_device,
                device: vk_device,
                queue_family_index,
                queue_index,
            },
            XrGraphicsContext {
                instance: wgpu_instance,
                device: Arc::new(wgpu_device),
                queue: Arc::new(wgpu_queue),
                adapter_info: wgpu_adapter.get_info(),
            },
        ))
    } else {
        Err(Box::new(xr::sys::Result::ERROR_EXTENSION_NOT_PRESENT))
    }
}
