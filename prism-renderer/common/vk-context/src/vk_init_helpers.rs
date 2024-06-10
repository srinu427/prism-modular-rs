use ash::{khr, vk};
use std::cmp::min;
use std::collections::HashMap;
use std::ffi::c_char;

pub unsafe fn make_instance(
  driver: &ash::Entry,
  needed_layers: Vec<*const c_char>,
  needed_extensions: Vec<*const c_char>,
) -> Result<ash::Instance, String> {
  let app_info = vk::ApplicationInfo::default()
    .application_name(c"Prism VK App")
    .application_version(0)
    .engine_name(c"Prism Engine")
    .engine_version(0)
    .api_version(vk::API_VERSION_1_0);

  let instance_create_info = vk::InstanceCreateInfo::default()
    .application_info(&app_info)
    .enabled_extension_names(&needed_extensions[..])
    .enabled_layer_names(&needed_layers[..]);

  #[cfg(target_os = "macos")]
  let instance_create_info = instance_create_info
    .flags(vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR);

  driver
    .create_instance(&instance_create_info, None)
    .map_err(|e| format!("at instance create: {e}"))
}

pub fn select_g_queue(gpu_queue_props: &Vec<vk::QueueFamilyProperties>) -> Option<u32> {
  let mut selected_queue = None;
  let mut selected_queue_count = 0;
  for (queue_idx, queue_props) in gpu_queue_props.iter().enumerate() {
    let g_support = queue_props.queue_flags.contains(vk::QueueFlags::GRAPHICS);
    if g_support && selected_queue_count < queue_props.queue_count {
      selected_queue = Some(queue_idx as u32);
      selected_queue_count = queue_props.queue_count;
    }
  }
  selected_queue
}

pub fn select_c_queue(gpu_queue_props: &Vec<vk::QueueFamilyProperties>) -> Option<u32> {
  let mut selected_queue = None;
  let mut selected_weight = 0;
  let mut selected_queue_count = 0;
  for (queue_idx, queue_props) in gpu_queue_props.iter().enumerate() {
    let g_support = queue_props.queue_flags.contains(vk::QueueFlags::GRAPHICS);
    let c_support = queue_props.queue_flags.contains(vk::QueueFlags::COMPUTE);

    if c_support {
      let mut weight = 0;
      if !g_support {
        weight += 1
      }
      if selected_weight < weight {
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
  }
  selected_queue
}

pub fn select_t_queue(gpu_queue_props: &Vec<vk::QueueFamilyProperties>) -> Option<u32> {
  let mut selected_queue = None;
  let mut selected_weight = 0;
  let mut selected_queue_count = 0;
  for (queue_idx, queue_props) in gpu_queue_props.iter().enumerate() {
    let g_support = queue_props.queue_flags.contains(vk::QueueFlags::GRAPHICS);
    let t_support = queue_props.queue_flags.contains(vk::QueueFlags::TRANSFER);
    let c_support = queue_props.queue_flags.contains(vk::QueueFlags::COMPUTE);

    if t_support {
      let mut weight = 0;
      if !g_support {
        weight += 2
      }
      if !c_support {
        weight += 1;
      }
      if selected_weight < weight {
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
  }
  selected_queue
}

pub fn select_p_queue(
  gpu_queue_props: &Vec<vk::QueueFamilyProperties>,
  surface_driver: &khr::surface::Instance,
  surface: vk::SurfaceKHR,
  gpu: vk::PhysicalDevice,
) -> Option<u32> {
  let mut selected_queue = None;
  let mut selected_queue_count = 0;
  unsafe {
    for (queue_idx, queue_props) in gpu_queue_props.iter().enumerate() {
      if let Ok(p_support) =
        surface_driver.get_physical_device_surface_support(gpu, queue_idx as u32, surface)
      {
        if p_support && selected_queue_count < queue_props.queue_count {
          selected_queue = Some(queue_idx as u32);
          selected_queue_count = queue_props.queue_count;
        }
      }
    }
  }

  selected_queue
}

pub fn select_g_t_p_c_queue_ids(
  gpu_queue_props: &Vec<vk::QueueFamilyProperties>,
  surface_driver: &khr::surface::Instance,
  surface: vk::SurfaceKHR,
  gpu: vk::PhysicalDevice,
) -> Option<[u32; 4]> {
  if let Some(graphics_q_idx) = select_g_queue(&gpu_queue_props) {
    if let Some(transfer_q_idx) = select_t_queue(&gpu_queue_props) {
      if let Some(present_q_idx) = select_p_queue(&gpu_queue_props, surface_driver, surface, gpu) {
        if let Some(compute_q_idx) = select_c_queue(&gpu_queue_props) {
          return Some([graphics_q_idx, transfer_q_idx, present_q_idx, compute_q_idx]);
        }
      }
    }
  }
  None
}

pub unsafe fn create_device_and_queues(
  instance: &ash::Instance,
  gpu: vk::PhysicalDevice,
  needed_extensions: Vec<*const c_char>,
  features: vk::PhysicalDeviceFeatures,
  queue_indices: [u32; 4],
) -> Result<(ash::Device, [vk::Queue; 4]), String> {
  let queue_priorities: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
  let gpu_queue_props = instance.get_physical_device_queue_family_properties(gpu);

  let mut q_idx_map = HashMap::<u32, u32>::with_capacity(4);
  for x in queue_indices {
    if x >= gpu_queue_props.len() as u32 {
      return Err("invalid queue ids requested".to_string());
    }
    if let Some(q_count) = q_idx_map.get_mut(&x) {
      *q_count = min(*q_count + 1, gpu_queue_props[x as usize].queue_count);
    } else {
      q_idx_map.insert(x, 1);
    }
  }

  let queue_create_infos = q_idx_map
    .iter()
    .map(|(k, v)| {
      vk::DeviceQueueCreateInfo::default()
        .queue_family_index(*k)
        .queue_priorities(&queue_priorities[0..(*v as usize)])
    })
    .collect::<Vec<_>>();
  let device_create_info = vk::DeviceCreateInfo::default()
    .queue_create_infos(queue_create_infos.as_slice())
    .enabled_extension_names(&needed_extensions[..])
    .enabled_features(&features);

  let device = instance
    .create_device(gpu, &device_create_info, None)
    .map_err(|e| format!("at logic device init: {e}"))?;

  let mut queues = Vec::with_capacity(4);
  for x in queue_indices {
    let cur_q_idx = q_idx_map.get_mut(&x).ok_or("invalid queue".to_string())?;
    queues.push(device.get_device_queue(x, *cur_q_idx - 1));
    if *cur_q_idx != 1 {
      *cur_q_idx -= 1;
    }
  }

  Ok((device, [queues[0], queues[1], queues[2], queues[3]]))
}
