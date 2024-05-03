mod gpu_debug_helpers;
pub mod structs;

use ash::ext;
pub use ash::khr;
pub use ash::vk;
pub use vk_mem;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::cmp::min;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use structs::*;
use vk_mem::{Allocator, AllocatorCreateInfo};

pub trait GraphicsPassGenerator {
  fn make_gpu_render_pass(
    vk_manager: &VKManager,
    image_format: vk::Format,
  ) -> Result<SDRenderPass, String>;

  fn create_per_frame_resources(
    vk_manager: &VKManager,
    graphics_pass: &mut SDRenderPass,
    allocator: Arc<Mutex<Allocator>>,
    resolution: vk::Extent2D,
    descriptor_pool: &SDDescriptorPool,
  ) -> Result<(), String>;

  fn add_init_per_frame_resources_commands(
    vk_manager: &VKManager,
    graphics_pass: &SDRenderPass,
    command_buffer: vk::CommandBuffer,
  ) -> Result<(), String>;
}

#[derive(Debug)]
pub enum VKManagerError {
  VulkanDriverNotFound,
  UnsupportedWindow,
  DebugDriverNotFound,
  RawDisplayHandleNotFound,
  RawWindowHandleNotFound,
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
  VKBufferCreationFailed,
  VKImageCreationFailed,
  FileNotFound,
  ShaderFileOpenError,
  ShaderFileParseError,
  ShaderModuleCreationFailed,
}

pub struct VKManager {
  _driver: ash::Entry,
  instance: ash::Instance,
  #[cfg(debug_assertions)]
  dbg_utils_driver: ext::debug_utils::Instance,
  #[cfg(debug_assertions)]
  dbg_messenger: vk::DebugUtilsMessengerEXT,
  pub surface_driver: khr::surface::Instance,
  pub surface: vk::SurfaceKHR,
  pub gpu: vk::PhysicalDevice,
  pub g_q_idx: u32,
  c_q_idx: u32,
  pub t_q_idx: u32,
  pub device: Arc<ash::Device>,
  pub g_queue: vk::Queue,
  c_queue: vk::Queue,
  pub t_queue: vk::Queue,
  pub swapchain_driver: Arc<khr::swapchain::Device>,
}

impl VKManager {
  unsafe fn create_instance(
    driver: &ash::Entry,
    window: &(impl HasWindowHandle + HasDisplayHandle),
  ) -> Result<ash::Instance, VKManagerError> {
    let app_name = c"Prism VK App";
    let engine_name = c"Prism Engine";
    let app_info = vk::ApplicationInfo::default()
      .application_name(app_name)
      .application_version(0)
      .engine_name(&engine_name)
      .engine_version(0)
      .api_version(vk::API_VERSION_1_2);

    #[cfg(debug_assertions)]
    let needed_layers = vec![c"VK_LAYER_KHRONOS_validation".as_ptr()];
    #[cfg(not(debug_assertions))]
    let needed_layers = vec![];
    // Replace with actual ash_window fn
    let mut needed_instance_extensions = ash_window::enumerate_required_extensions(
      window
        .display_handle()
        .map_err(|_| VKManagerError::UnsupportedWindow)?
        .as_raw(),
    )
    .map_err(|_| VKManagerError::GetWindowExtensionsFailed)?
    .to_vec();
    #[cfg(debug_assertions)]
    needed_instance_extensions.push(ext::debug_utils::NAME.as_ptr());

    #[cfg(target_os = "macos")]
    needed_instance_extensions.push(vk::KhrPortabilityEnumerationFn::name().as_ptr());

    #[cfg(not(target_os = "macos"))]
    let instance_create_info = vk::InstanceCreateInfo::default()
      .application_info(&app_info)
      .enabled_extension_names(needed_instance_extensions.as_slice())
      .enabled_layer_names(needed_layers.as_slice());

    #[cfg(target_os = "macos")]
    let instance_create_info = vk::InstanceCreateInfo {
      p_application_info: &app_info,
      enabled_extension_count: needed_instance_extensions.len() as u32,
      pp_enabled_extension_names: needed_instance_extensions.as_ptr(),
      enabled_layer_count: c_needed_layers.len() as u32,
      pp_enabled_layer_names: c_needed_layers.as_ptr(),
      flags: vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR,
      ..Default::default()
    };

    Ok(
      driver
        .create_instance(&instance_create_info, None)
        .map_err(|_| VKManagerError::InstanceCreationFailed)?,
    )
  }

  fn select_g_queue(
    gpu_queue_props: &Vec<vk::QueueFamilyProperties>,
  ) -> Result<u32, VKManagerError> {
    let mut selected_queue = None;
    let mut selected_weight = 4;
    let mut selected_queue_count = 0;
    for (queue_idx, queue_props) in gpu_queue_props.iter().enumerate() {
      let g_support = queue_props.queue_flags.contains(vk::QueueFlags::GRAPHICS);
      let weight = if g_support { 3 } else { 4 };
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
      let weight = if c_support {
        if g_support {
          3
        } else {
          2
        }
      } else {
        4
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
      let weight = if t_support {
        if g_support {
          3
        } else {
          if c_support {
            2
          } else {
            1
          }
        }
      } else {
        4
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
    surface_driver: &khr::surface::Instance,
    surface: vk::SurfaceKHR,
  ) -> Result<(vk::PhysicalDevice, u32, u32, u32), VKManagerError> {
    for gpu in instance.enumerate_physical_devices().unwrap() {
      let gpu_queue_props = instance.get_physical_device_queue_family_properties(gpu);

      let Ok(graphics_queue) = Self::select_g_queue(&gpu_queue_props) else {
        continue;
      };
      let Ok(transfer_queue) = Self::select_t_queue(&gpu_queue_props) else {
        continue;
      };
      let Ok(compute_queue) = Self::select_c_queue(&gpu_queue_props) else {
        continue;
      };
      if let Ok(surface_support) =
        surface_driver.get_physical_device_surface_support(gpu, graphics_queue, surface)
      {
        if surface_support {
          return Ok((gpu, graphics_queue, compute_queue, transfer_queue));
        }
      }
    }
    Err(VKManagerError::NoSupportedGPU)
  }

  unsafe fn create_device_and_queues(
    instance: &ash::Instance,
    gpu: vk::PhysicalDevice,
    queue_indices: [u32; 3],
  ) -> Result<(ash::Device, vk::Queue, vk::Queue, vk::Queue), VKManagerError> {
    #[cfg(not(target_os = "macos"))]
    let device_extensions = [khr::surface::NAME.as_ptr()];
    #[cfg(target_os = "macos")]
    let device_extensions = [
      Swapchain::name().as_ptr(),
      vk::KhrPortabilitySubsetFn::name().as_ptr(),
    ];
    let device_features = vk::PhysicalDeviceFeatures::default();

    let queue_priorities: [f32; 3] = [1.0, 1.0, 1.0];
    let gpu_queue_props = instance.get_physical_device_queue_family_properties(gpu);
    let mut q_idx_map = HashMap::<u32, u32>::with_capacity(4);
    for x in queue_indices {
      if let Some(q_count) = q_idx_map.get_mut(&x) {
        *q_count = min(
          *q_count + 1,
          gpu_queue_props
            .get(x as usize)
            .ok_or(VKManagerError::InvalidQueueRequested)?
            .queue_count,
        );
      } else {
        q_idx_map.insert(x, 1);
      }
    }
    let mut queue_create_infos = Vec::<vk::DeviceQueueCreateInfo>::with_capacity(q_idx_map.len());
    for (k, v) in &q_idx_map {
      queue_create_infos.push(
        vk::DeviceQueueCreateInfo::default()
          .queue_family_index(*k)
          .queue_priorities(&queue_priorities[0..(*v as usize)]),
      );
    }
    let device_create_info = vk::DeviceCreateInfo::default()
      .queue_create_infos(queue_create_infos.as_slice())
      .enabled_extension_names(device_extensions.as_slice())
      .enabled_features(&device_features);

    let device = instance
      .create_device(gpu, &device_create_info, None)
      .map_err(|_| VKManagerError::DeviceCreationFailed)?;

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
    window: Arc<(impl HasWindowHandle + HasDisplayHandle)>,
  ) -> Result<Self, VKManagerError> {
    let driver = unsafe { ash::Entry::load().map_err(|_| VKManagerError::VulkanDriverNotFound)? };
    let instance = unsafe { Self::create_instance(&driver, window.as_ref())? };

    #[cfg(debug_assertions)]
    let dbg_utils_driver = ext::debug_utils::Instance::new(&driver, &instance);
    #[cfg(debug_assertions)]
    let dbg_messenger = unsafe {
      dbg_utils_driver
        .create_debug_utils_messenger(&gpu_debug_helpers::make_debug_mgr_create_info(), None)
        .map_err(|_| VKManagerError::DebugMessengerCreationFailed)?
    };

    let surface_driver = khr::surface::Instance::new(&driver, &instance);
    let surface = unsafe {
      ash_window::create_surface(
        &driver,
        &instance,
        window
          .display_handle()
          .map_err(|_| VKManagerError::UnsupportedWindow)?
          .as_raw(),
        window
          .window_handle()
          .map_err(|_| VKManagerError::UnsupportedWindow)?
          .as_raw(),
        None,
      )
      .map_err(|_| VKManagerError::SurfaceCreationFailed)?
    };

    let (gpu, g_q_idx, c_q_idx, t_q_idx) =
      unsafe { Self::select_gpu(&instance, &surface_driver, surface)? };
    let (device, g_queue, c_queue, t_queue) =
      unsafe { Self::create_device_and_queues(&instance, gpu, [g_q_idx, c_q_idx, t_q_idx])? };

    let swapchain_driver = khr::swapchain::Device::new(&instance, &device);

    Ok(Self {
      _driver: driver,
      instance,
      #[cfg(debug_assertions)]
      dbg_utils_driver,
      #[cfg(debug_assertions)]
      dbg_messenger,
      surface_driver,
      surface,
      gpu,
      g_q_idx,
      c_q_idx,
      t_q_idx,
      device: Arc::new(device),
      g_queue,
      c_queue,
      t_queue,
      swapchain_driver: Arc::new(swapchain_driver),
    })
  }

  pub fn make_mem_allocator(&self) -> Result<Arc<Mutex<Allocator>>, VKManagerError> {
    Ok(Arc::new(Mutex::new(
      Allocator::new(AllocatorCreateInfo::new(
        &self.instance,
        &self.device,
        self.gpu,
      ))
      .map_err(|_| VKManagerError::MemoryAllocatorCreationFailed)?,
    )))
  }

  pub fn create_buffer(
    &self,
    allocator: Arc<Mutex<Allocator>>,
    size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    mem_props: vk::MemoryPropertyFlags,
  ) -> Result<SDBuffer, VKManagerError> {
    let buffer_create_info = vk::BufferCreateInfo::default()
      .usage(usage)
      .size(size)
      .sharing_mode(vk::SharingMode::EXCLUSIVE);

    SDBuffer::new(buffer_create_info, allocator, mem_props)
      .map_err(|_| VKManagerError::VKBufferCreationFailed)
  }

  pub fn create_2d_image(
    &self,
    allocator: Arc<Mutex<Allocator>>,
    resolution: vk::Extent2D,
    format: vk::Format,
    usage: vk::ImageUsageFlags,
    mem_props: vk::MemoryPropertyFlags,
  ) -> Result<SDImage, VKManagerError> {
    let image_create_info = vk::ImageCreateInfo::default()
      .image_type(vk::ImageType::TYPE_2D)
      .format(format)
      .extent(vk::Extent3D::from(resolution).depth(1))
      .mip_levels(1)
      .array_layers(1)
      .samples(vk::SampleCountFlags::TYPE_1)
      .tiling(vk::ImageTiling::OPTIMAL)
      .usage(usage)
      .sharing_mode(vk::SharingMode::EXCLUSIVE)
      .initial_layout(vk::ImageLayout::UNDEFINED);

    SDImage::new(image_create_info, allocator, mem_props)
      .map_err(|_| VKManagerError::VKImageCreationFailed)
  }

  pub fn make_shader_from_spv(
    &self,
    shader_path: PathBuf,
  ) -> Result<vk::ShaderModule, VKManagerError> {
    if shader_path.is_file() {
      let mut shader_file_bytes =
        fs::File::open(shader_path).map_err(|_| VKManagerError::ShaderFileOpenError)?;
      let shader_data = ash::util::read_spv(&mut shader_file_bytes)
        .map_err(|_| VKManagerError::ShaderFileParseError)?;
      Ok(unsafe {
        self
          .device
          .create_shader_module(
            &vk::ShaderModuleCreateInfo::default().code(shader_data.as_slice()),
            None,
          )
          .map_err(|_| VKManagerError::ShaderModuleCreationFailed)?
      })
    } else {
      Err(VKManagerError::FileNotFound)
    }
  }
}

impl Drop for VKManager {
  fn drop(&mut self) {
    unsafe {
      self.device.destroy_device(None);
      self.surface_driver.destroy_surface(self.surface, None);
      #[cfg(debug_assertions)]
      self
        .dbg_utils_driver
        .destroy_debug_utils_messenger(self.dbg_messenger, None);
      self.instance.destroy_instance(None);
    }
  }
}
