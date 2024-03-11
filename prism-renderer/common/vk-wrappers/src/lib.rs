pub mod structs;
mod gpu_debug_helpers;

use ash::extensions::ext::DebugUtils;
use ash::extensions::khr::{Surface, Swapchain};
use ash::vk;
use std::cmp::min;
use std::collections::HashMap;
use std::ffi::{c_char, CString};
use std::fs;
use std::path::PathBuf;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use gpu_allocator::vulkan::*;
use gpu_allocator::MemoryLocation;
use structs::*;

pub trait GraphicsPassGenerator {
    fn make_gpu_render_pass(
        vk_manager: &VKManager,
        image_format: vk::Format,
    ) -> Result<GraphicsPass, String>;

    fn create_per_frame_resources(
        vk_manager: &VKManager,
        graphics_pass: &mut GraphicsPass,
        allocator: &mut Allocator,
        resolution: vk::Extent2D,
    ) -> Result<(), String>;

    fn add_init_per_frame_resources_commands(
        vk_manager: &VKManager,
        graphics_pass: &GraphicsPass,
        command_buffer: vk::CommandBuffer,
    ) -> Result<(), String>;
}

#[derive(Debug)]
pub enum VKManagerError {
    VulkanDriverNotFound,
    DebugDriverNotFound,
    CStringInitFailed,
    GetWindowExtensionsFailed,
    InstanceCreationFailed,
    DebugMessengerCreationFailed,
    SurfaceCreationFailed,
    GraphicsQueueNotSupported,
    ComputeQueueNotSupported,
    TransferQueueNotSupported,
    NoSupportedGPU,
    InvalidQueueRequested,
    DeviceCreationFailed,
    QueueCreationFailed,
    MemoryAllocatorCreationFailed,
    MemoryAllocationFailed,
    MemoryFreeFailed,
    VKBufferCreationFailed,
    BufferMemoryBindFailed,
    BufferDeletionFailed,
    VKImageCreationFailed,
    ImageMemoryBindFailed,
    ImageDeletionFailed,
    FileNotFound,
    ShaderFileOpenError,
    ShaderFileParseError,
    ShaderModuleCreationFailed,
}

#[cfg(debug_assertions)]
const ENABLE_GPU_DEBUG: bool = true;

#[cfg(not(debug_assertions))]
const ENABLE_GPU_DEBUG: bool = false;

pub struct VKManager {
    _driver: ash::Entry,
    instance: ash::Instance,
    dbg_utils_driver: Option<DebugUtils>,
    dbg_messenger: Option<vk::DebugUtilsMessengerEXT>,
    pub surface_driver: Surface,
    pub surface: vk::SurfaceKHR,
    pub gpu: vk::PhysicalDevice,
    pub g_q_idx: u32,
    c_q_idx: u32,
    pub t_q_idx: u32,
    pub device: ash::Device,
    pub g_queue: vk::Queue,
    c_queue: vk::Queue,
    pub t_queue: vk::Queue,
    pub swapchain_driver: Swapchain,
}

impl VKManager {
    unsafe fn create_instance(
        driver: &ash::Entry,
        window: &(impl HasRawWindowHandle + HasRawDisplayHandle),
    ) -> Result<ash::Instance, VKManagerError> {
        let app_name = CString::new("Prism VK App")
            .ok()
            .ok_or(VKManagerError::CStringInitFailed)?;
        let engine_name = CString::new("Prism Engine")
            .ok()
            .ok_or(VKManagerError::CStringInitFailed)?;
        let app_info = vk::ApplicationInfo {
            p_application_name: app_name.as_ptr(),
            application_version: 0,
            p_engine_name: engine_name.as_ptr(),
            engine_version: 0,
            api_version: vk::API_VERSION_1_2,
            ..Default::default()
        };
        let mut needed_layers = vec![];
        // Replace with actual ash_window fn
        let mut needed_instance_extensions =
            ash_window::enumerate_required_extensions(window.raw_display_handle())
                .ok()
                .ok_or(VKManagerError::GetWindowExtensionsFailed)?
                .to_vec();
        if ENABLE_GPU_DEBUG {
            needed_layers.push(
                CString::new("VK_LAYER_KHRONOS_validation")
                    .ok()
                    .ok_or(VKManagerError::CStringInitFailed)?,
            );
            needed_instance_extensions.push(DebugUtils::name().as_ptr());
        }
        let c_needed_layers: Vec<*const c_char> =
            needed_layers.iter().map(|x| x.as_ptr()).collect();

        let instance_create_info = vk::InstanceCreateInfo {
            p_application_info: &app_info,
            enabled_extension_count: needed_instance_extensions.len() as u32,
            pp_enabled_extension_names: needed_instance_extensions.as_ptr(),
            enabled_layer_count: c_needed_layers.len() as u32,
            pp_enabled_layer_names: c_needed_layers.as_ptr(),
            ..Default::default()
        };

        Ok(driver
            .create_instance(&instance_create_info, None)
            .ok()
            .ok_or(VKManagerError::InstanceCreationFailed)?)
    }

    fn select_g_queue(
        gpu_queue_props: &Vec<vk::QueueFamilyProperties>,
    ) -> Result<u32, VKManagerError> {
        let mut selected_queue = None;
        let mut selected_weight = 4;
        let mut selected_queue_count = 0;
        for (queue_idx, queue_props) in gpu_queue_props.iter().enumerate() {
            let g_support = queue_props.queue_flags.contains(vk::QueueFlags::GRAPHICS);
            let mut weight = 4;
            if g_support {
                weight = 3
            }
            if selected_weight > weight {
                selected_queue = Some(queue_idx as u32);
                selected_weight = weight;
                selected_queue_count = queue_props.queue_count;
            }
            if selected_weight == weight && selected_queue_count < queue_props.queue_count {
                selected_queue = Some(queue_idx as u32);
                selected_weight = weight;
                selected_queue_count = queue_props.queue_count;
            }
        }
        selected_queue.ok_or(VKManagerError::GraphicsQueueNotSupported)
    }

    fn select_c_queue(
        gpu_queue_props: &Vec<vk::QueueFamilyProperties>,
    ) -> Result<u32, VKManagerError> {
        let mut selected_queue = None;
        let mut selected_weight = 4;
        let mut selected_queue_count = 0;
        for (queue_idx, queue_props) in gpu_queue_props.iter().enumerate() {
            let g_support = queue_props.queue_flags.contains(vk::QueueFlags::GRAPHICS);
            let c_support = queue_props.queue_flags.contains(vk::QueueFlags::COMPUTE);
            let mut weight = 4;
            if c_support {
                weight = 3;
                if !g_support {
                    weight = 2
                }
            }
            if selected_weight > weight {
                selected_queue = Some(queue_idx as u32);
                selected_weight = weight;
                selected_queue_count = queue_props.queue_count;
            }
            if selected_weight == weight && selected_queue_count < queue_props.queue_count {
                selected_queue = Some(queue_idx as u32);
                selected_weight = weight;
                selected_queue_count = queue_props.queue_count;
            }
        }
        selected_queue.ok_or(VKManagerError::ComputeQueueNotSupported)
    }

    fn select_t_queue(
        gpu_queue_props: &Vec<vk::QueueFamilyProperties>,
    ) -> Result<u32, VKManagerError> {
        let mut selected_queue = None;
        let mut selected_weight = 4;
        let mut selected_queue_count = 0;
        for (queue_idx, queue_props) in gpu_queue_props.iter().enumerate() {
            let g_support = queue_props.queue_flags.contains(vk::QueueFlags::GRAPHICS);
            let t_support = queue_props.queue_flags.contains(vk::QueueFlags::TRANSFER);
            let c_support = queue_props.queue_flags.contains(vk::QueueFlags::COMPUTE);
            let mut weight = 4;
            if t_support {
                weight = 3;
                if !g_support {
                    weight = 2
                }
                if !g_support && !c_support {
                    weight = 1
                }
            };
            if selected_weight > weight {
                selected_queue = Some(queue_idx as u32);
                selected_weight = weight;
                selected_queue_count = queue_props.queue_count;
            }
            if selected_weight == weight && selected_queue_count < queue_props.queue_count {
                selected_queue = Some(queue_idx as u32);
                selected_weight = weight;
                selected_queue_count = queue_props.queue_count;
            }
        }
        selected_queue.ok_or(VKManagerError::TransferQueueNotSupported)
    }

    unsafe fn select_gpu(
        instance: &ash::Instance,
        surface_driver: &Surface,
        surface: vk::SurfaceKHR,
    ) -> Result<(vk::PhysicalDevice, u32, u32, u32), VKManagerError> {
        for gpu in instance.enumerate_physical_devices().unwrap() {
            let gpu_queue_props = instance.get_physical_device_queue_family_properties(gpu);

            let graphics_queue = match Self::select_g_queue(&gpu_queue_props) {
                Ok(x) => x,
                Err(_) => continue,
            };
            let transfer_queue = match Self::select_t_queue(&gpu_queue_props) {
                Ok(x) => x,
                Err(_) => continue,
            };
            let compute_queue = match Self::select_c_queue(&gpu_queue_props) {
                Ok(x) => x,
                Err(_) => continue,
            };

            match surface_driver.get_physical_device_surface_support(gpu, graphics_queue, surface) {
                Ok(_) => {}
                Err(_) => continue,
            };

            return Ok((gpu, graphics_queue, compute_queue, transfer_queue));
        }
        Err(VKManagerError::NoSupportedGPU)
    }

    unsafe fn create_device_and_queues(
        instance: &ash::Instance,
        gpu: vk::PhysicalDevice,
        queue_indices: [u32; 3],
    ) -> Result<(ash::Device, vk::Queue, vk::Queue, vk::Queue), VKManagerError> {
        let device_extensions = [Swapchain::name().as_ptr()];
        let device_features = vk::PhysicalDeviceFeatures {
            ..Default::default()
        };

        let queue_priorities: [f32; 3] = [1.0, 1.0, 1.0];
        let gpu_queue_props = instance.get_physical_device_queue_family_properties(gpu);
        let mut q_idx_map = HashMap::<u32, u32>::with_capacity(4);
        for x in queue_indices {
            match q_idx_map.get_mut(&x) {
                None => {
                    q_idx_map.insert(x, 1);
                }
                Some(qc) => {
                    *qc = min(
                        *qc + 1,
                        gpu_queue_props
                            .get(x as usize)
                            .ok_or(VKManagerError::InvalidQueueRequested)?
                            .queue_count,
                    );
                }
            }
        }
        // println!("{:#?}", q_idx_map);
        let mut queue_create_infos =
            Vec::<vk::DeviceQueueCreateInfo>::with_capacity(q_idx_map.len());
        for (k, v) in q_idx_map.iter() {
            queue_create_infos.push(vk::DeviceQueueCreateInfo {
                queue_family_index: *k,
                queue_count: *v,
                p_queue_priorities: queue_priorities.as_ptr(),
                ..Default::default()
            });
        }
        let device_create_info = vk::DeviceCreateInfo {
            queue_create_info_count: queue_create_infos.len() as u32,
            p_queue_create_infos: queue_create_infos.as_ptr(),
            enabled_extension_count: device_extensions.len() as u32,
            pp_enabled_extension_names: device_extensions.as_ptr(),
            p_enabled_features: &device_features,
            ..Default::default()
        };
        let device = instance
            .create_device(gpu, &device_create_info, None)
            .ok()
            .ok_or(VKManagerError::DeviceCreationFailed)?;

        let mut queues = Vec::<vk::Queue>::with_capacity(4);
        for x in queue_indices {
            let cur_q_idx = q_idx_map
                .get_mut(&x)
                .ok_or(VKManagerError::InvalidQueueRequested)?;
            queues.push(device.get_device_queue(x, *cur_q_idx - 1));
            if *cur_q_idx != 1 {
                *cur_q_idx -= 1;
            }
        }
        // println!("{:#?}", queues);
        Ok((
            device,
            *queues.get(0).ok_or(VKManagerError::QueueCreationFailed)?,
            *queues.get(1).ok_or(VKManagerError::QueueCreationFailed)?,
            *queues.get(2).ok_or(VKManagerError::QueueCreationFailed)?,
        ))
    }

    pub fn new(
        window: &(impl HasRawWindowHandle + HasRawDisplayHandle)
    ) -> Result<Self, VKManagerError> {
        let driver = unsafe {
            ash::Entry::load()
                .ok()
                .ok_or(VKManagerError::VulkanDriverNotFound)?
        };
        let instance = unsafe { Self::create_instance(&driver, window)? };

        let dbg_utils_driver = {
            if ENABLE_GPU_DEBUG {
                Some(DebugUtils::new(&driver, &instance))
            } else {
                None
            }
        };
        let dbg_messenger = unsafe {
            if ENABLE_GPU_DEBUG {
                Some(
                    dbg_utils_driver
                        .as_ref()
                        .ok_or(VKManagerError::DebugDriverNotFound)?
                        .create_debug_utils_messenger(
                            &gpu_debug_helpers::make_debug_mgr_create_info(),
                            None,
                        )
                        .ok()
                        .ok_or(VKManagerError::DebugMessengerCreationFailed)?,
                )
            } else {
                None
            }
        };

        let surface_driver = Surface::new(&driver, &instance);
        let surface = unsafe {
            ash_window::create_surface(
                &driver,
                &instance,
                window.raw_display_handle(),
                window.raw_window_handle(),
                None,
            )
                .ok()
                .ok_or(VKManagerError::SurfaceCreationFailed)?
        };

        let (gpu, g_q_idx, c_q_idx, t_q_idx) =
            unsafe { Self::select_gpu(&instance, &surface_driver, surface)? };
        let (device, g_queue, c_queue, t_queue) =
            unsafe { Self::create_device_and_queues(&instance, gpu, [g_q_idx, c_q_idx, t_q_idx])? };

        let swapchain_driver = Swapchain::new(&instance, &device);

        Ok(Self {
            _driver: driver,
            instance,
            dbg_utils_driver,
            dbg_messenger,
            surface_driver,
            surface,
            gpu,
            g_q_idx,
            c_q_idx,
            t_q_idx,
            device,
            g_queue,
            c_queue,
            t_queue,
            swapchain_driver,
        })
    }

    pub fn make_mem_allocator(&self) -> Result<Allocator, VKManagerError> {
        Allocator::new(&AllocatorCreateDesc {
            instance: self.instance.clone(),
            device: self.device.clone(),
            physical_device: self.gpu.clone(),
            debug_settings: Default::default(),
            buffer_device_address: false,
            allocation_sizes: Default::default(),
        })
            .ok()
            .ok_or(VKManagerError::MemoryAllocatorCreationFailed)
    }

    pub fn create_buffer(
        &self,
        allocator: &mut Allocator,
        name: &str,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        memory_location: MemoryLocation,
    ) -> Result<GPUBuffer, VKManagerError> {
        let buffer_create_info = vk::BufferCreateInfo {
            usage,
            size,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        unsafe {
            let buffer = self
                .device
                .create_buffer(&buffer_create_info, None)
                .ok()
                .ok_or(VKManagerError::VKBufferCreationFailed)?;
            let malloc_requirements = self.device.get_buffer_memory_requirements(buffer);
            let malloc_info = AllocationCreateDesc {
                name,
                requirements: malloc_requirements,
                location: memory_location,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            };
            let mem_allocation = allocator
                .allocate(&malloc_info)
                .ok()
                .ok_or(VKManagerError::MemoryAllocationFailed)?;
            self.device
                .bind_buffer_memory(buffer, mem_allocation.memory(), mem_allocation.offset())
                .ok()
                .ok_or(VKManagerError::BufferMemoryBindFailed)?;
            Ok(GPUBuffer {
                buffer,
                allocation: mem_allocation,
                current_size: size,
            })
        }
    }

    pub fn destroy_buffer(
        &self,
        allocator: &mut Allocator,
        buffer: GPUBuffer,
    ) -> Result<(), VKManagerError> {
        unsafe { self.device.destroy_buffer(buffer.buffer, None) };
        allocator
            .free(buffer.allocation)
            .ok()
            .ok_or(VKManagerError::MemoryFreeFailed)?;
        Ok(())
    }

    pub fn create_2d_image(
        &self,
        allocator: &mut Allocator,
        name: &str,
        resolution: vk::Extent2D,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
    ) -> Result<GPUImage, VKManagerError> {
        let image_create_info = vk::ImageCreateInfo {
            flags: Default::default(),
            image_type: vk::ImageType::TYPE_2D,
            format,
            extent: vk::Extent3D {
                width: resolution.width,
                height: resolution.height,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            ..Default::default()
        };
        unsafe{
            let image = self.device
                .create_image(&image_create_info, None)
                .ok()
                .ok_or(VKManagerError::VKImageCreationFailed)?;
            let malloc_requirements = self.device.get_image_memory_requirements(image);
            let malloc_info = AllocationCreateDesc {
                name,
                requirements: malloc_requirements,
                location: MemoryLocation::GpuOnly,
                linear: false,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            };
            let mem_allocation = allocator
                .allocate(&malloc_info)
                .ok()
                .ok_or(VKManagerError::MemoryAllocationFailed)?;
            self.device
                .bind_image_memory(image, mem_allocation.memory(), mem_allocation.offset())
                .ok()
                .ok_or(VKManagerError::ImageMemoryBindFailed)?;
            Ok(GPUImage {
                image,
                allocation: mem_allocation,
                current_res: resolution,
            })
        }
    }

    pub fn destroy_image(
        &self,
        allocator: &mut Allocator,
        image: GPUImage,
    ) -> Result<(), VKManagerError> {
        unsafe { self.device.destroy_image(image.image, None) };
        allocator
            .free(image.allocation)
            .ok()
            .ok_or(VKManagerError::MemoryFreeFailed)?;
        Ok(())
    }

    pub fn make_shader_from_spv(
        &self, shader_path: PathBuf,
    ) -> Result<vk::ShaderModule, VKManagerError> {
        if shader_path.is_file() {
            let mut shader_file_bytes = fs::File::open(shader_path)
                .ok()
                .ok_or(VKManagerError::ShaderFileOpenError)?;
            let shader_data = ash::util::read_spv(&mut shader_file_bytes)
                .ok()
                .ok_or(VKManagerError::ShaderFileParseError)?;
            Ok(unsafe {
                self
                    .device
                    .create_shader_module(
                        &vk::ShaderModuleCreateInfo {
                            code_size: shader_data.len() * 4,
                            p_code: shader_data.as_ptr(),
                            ..Default::default()
                        },
                        None,
                    )
                    .ok()
                    .ok_or(VKManagerError::ShaderModuleCreationFailed)?
            })
        } else {
            Err(VKManagerError::FileNotFound)
        }
    }

    pub unsafe fn destroy_per_frame_render_pass_resources(
        &self,
        per_frame_render_pass_resources: PerFrameGraphicsPassResources,
        allocator: &mut Allocator,
    ) {
        self.device
            .destroy_framebuffer(per_frame_render_pass_resources.frame_buffer.clone(), None);

        for image_view in per_frame_render_pass_resources.attachment_image_views {
            self.device.destroy_image_view(image_view.clone(), None);
        }
        for image in per_frame_render_pass_resources.attachments {
            let _ = self.destroy_image(allocator, image);
        }
    }

    pub fn destroy_gpu_render_pass(
        &self,
        gpu_render_pass: GraphicsPass,
        allocator: &mut Allocator,
    ) {
        for render_pass_resources in gpu_render_pass.per_frame_resources {
            unsafe {
                self.destroy_per_frame_render_pass_resources(render_pass_resources, allocator)
            };
        }
        for pipeline in gpu_render_pass.pipelines {
            unsafe { self.device.destroy_pipeline_layout(pipeline.0, None) };
            unsafe { self.device.destroy_pipeline(pipeline.1, None) };
        }
        unsafe { self.device.destroy_render_pass(gpu_render_pass.raw, None) };
    }
}

impl Drop for VKManager {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
            self.surface_driver.destroy_surface(self.surface, None);
            match self.dbg_utils_driver.as_ref() {
                None => {}
                Some(dbg_drv) => match self.dbg_messenger {
                    None => {}
                    Some(x) => dbg_drv.destroy_debug_utils_messenger(x, None),
                },
            };
            self.instance.destroy_instance(None);
        }
    }
}
