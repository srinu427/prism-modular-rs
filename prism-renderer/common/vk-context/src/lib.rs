#[cfg(debug_assertions)]
mod debug_helpers;
pub mod helpers;
mod vk_init_helpers;

use std::sync::Arc;

#[cfg(debug_assertions)]
pub use ash::ext;
pub use ash::khr;
pub use ash::vk;
pub use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

pub struct VkLoaders {
  pub surface_driver: khr::surface::Instance,
  #[cfg(debug_assertions)]
  pub dbg_messenger: vk::DebugUtilsMessengerEXT,
  #[cfg(debug_assertions)]
  pub dbg_utils_driver: ext::debug_utils::Instance,
  pub vk_driver: ash::Instance,
  _loader: ash::Entry,
}

impl VkLoaders {
  pub fn new() -> Result<Self, String> {
    let layers = vec![
      #[cfg(debug_assertions)]
      c"VK_LAYER_KHRONOS_validation".as_ptr(),
    ];
    let instance_extensions = vec![
      #[cfg(debug_assertions)]
      ext::debug_utils::NAME.as_ptr(),
    ];
    unsafe {
      let loader = ash::Entry::load().map_err(|e| format!("vulkan load failed: {e}"))?;
      let vk_driver = vk_init_helpers::make_instance(&loader, layers, instance_extensions)?;

      #[cfg(debug_assertions)]
      let dbg_utils_driver = ext::debug_utils::Instance::new(&loader, &vk_driver);
      #[cfg(debug_assertions)]
      let dbg_messenger = dbg_utils_driver
        .create_debug_utils_messenger(&debug_helpers::make_debug_mgr_create_info(), None)
        .map_err(|e| format!("debug messenger init failed: {e}"))?;

      let surface_driver = khr::surface::Instance::new(&loader, &vk_driver);
      Ok(Self { surface_driver, dbg_messenger, dbg_utils_driver, vk_driver, _loader: loader })
    }
  }

  pub fn make_surface(
    &self,
    window: &(impl HasWindowHandle + HasDisplayHandle),
  ) -> Result<vk::SurfaceKHR, String> {
    unsafe {
      ash_window::create_surface(
        &self._loader,
        &self.vk_driver,
        window.display_handle().map_err(|_| "unsupported window".to_string())?.as_raw(),
        window.window_handle().map_err(|_| "unsupported window".to_string())?.as_raw(),
        None,
      )
      .map_err(|e| format!("surface create error: {e}"))
    }
  }
}

impl Drop for VkLoaders {
  fn drop(&mut self) {
    unsafe {
      self.vk_driver.destroy_instance(None);
    }
  }
}

pub struct VkContext {
  pub device: ash::Device,
  pub graphics_q: vk::Queue,
  pub transfer_q: vk::Queue,
  pub present_q: vk::Queue,
  pub compute_q: vk::Queue,
  pub gpu: vk::PhysicalDevice,
  pub graphics_q_idx: u32,
  pub transfer_q_idx: u32,
  pub present_q_idx: u32,
  pub compute_q_idx: u32,
  pub vk_loaders: Arc<VkLoaders>,
}

impl VkContext {
  unsafe fn select_gpu(
    vk_driver: &ash::Instance,
    surface_driver: &khr::surface::Instance,
    surface: vk::SurfaceKHR,
    preferred_gpu: Option<(u32, u32)>,
  ) -> Result<(vk::PhysicalDevice, [u32; 4]), String> {
    let gpu_list =
      vk_driver.enumerate_physical_devices().map_err(|e| format!("can't get GPU list: {e}"))?;
    let gpu_infos = gpu_list
      .into_iter()
      .filter_map(|gpu| {
        let gpu_info = vk_driver.get_physical_device_properties(gpu);
        let gpu_queue_info = vk_driver.get_physical_device_queue_family_properties(gpu);
        vk_init_helpers::select_g_t_p_c_queue_ids(&gpu_queue_info, &surface_driver, surface, gpu)
          .map(|gpu_queue_ids| (gpu, (gpu_info.vendor_id, gpu_info.device_id), gpu_queue_ids))
      })
      .collect::<Vec<_>>();

    match preferred_gpu {
      None => gpu_infos
        .iter()
        .next()
        .cloned()
        .ok_or("no supported GPU".to_string())
        .map(|selected_gpu_info| (selected_gpu_info.0, selected_gpu_info.2)),
      Some(preferred_gpu_ids) => {
        gpu_infos.iter().find(|(_, gpu_ids, _)| *gpu_ids == preferred_gpu_ids).cloned().map_or(
          gpu_infos
            .iter()
            .next()
            .cloned()
            .ok_or("no supported GPU".to_string())
            .map(|selected_gpu_info| (selected_gpu_info.0, selected_gpu_info.2)),
          |selected_gpu_info| Ok((selected_gpu_info.0, selected_gpu_info.2)),
        )
      }
    }
  }

  pub fn new(
    vk_loaders: Arc<VkLoaders>,
    surface: vk::SurfaceKHR,
    preferred_gpu: Option<(u32, u32)>,
  ) -> Result<Self, String> {
    let device_extensions = vec![khr::swapchain::NAME.as_ptr()];

    unsafe {
      let (gpu, queue_ids) = Self::select_gpu(
        &vk_loaders.vk_driver,
        &vk_loaders.surface_driver,
        surface,
        preferred_gpu,
      )?;
      let (device, queues) = vk_init_helpers::create_device_and_queues(
        &vk_loaders.vk_driver,
        gpu,
        device_extensions,
        vk::PhysicalDeviceFeatures::default(),
        queue_ids,
      )?;
      Ok(Self {
        device,
        graphics_q: queues[0],
        transfer_q: queues[1],
        present_q: queues[2],
        compute_q: queues[3],
        gpu,
        graphics_q_idx: queue_ids[0],
        transfer_q_idx: queue_ids[1],
        present_q_idx: queue_ids[2],
        compute_q_idx: queue_ids[3],
        vk_loaders,
      })
    }
  }
}

impl Drop for VkContext {
  fn drop(&mut self) {
    unsafe {
      self.device.destroy_device(None);
      #[cfg(debug_assertions)]
      self
        .vk_loaders
        .dbg_utils_driver
        .destroy_debug_utils_messenger(self.vk_loaders.dbg_messenger, None);
      self.vk_loaders.vk_driver.destroy_instance(None);
    }
  }
}
