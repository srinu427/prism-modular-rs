use std::sync::Arc;
use vk_context::helpers::PWImage;
use vk_context::{khr, vk, VkContext};

#[derive(thiserror::Error, Debug)]
pub enum PresentManagerError {

}

pub struct PresentManager {
  vk_context: Arc<VkContext>,
  swapchain_device: khr::swapchain::Device,
  surface: vk::SurfaceKHR,
  cmd_pool: vk::CommandPool,
  cmd_buffers: Vec<vk::CommandBuffer>,
  acquire_image_semaphores: Vec<vk::Semaphore>,
  image_blit_semaphores: Vec<vk::Semaphore>,
  images: Vec<PWImage>,
  swapchain: vk::SwapchainKHR,
  resolution: vk::Extent2D,
  image_being_presented: u32,
  images_init_done: [bool; 3],
}

impl PresentManager {
  fn select_surface_format(formats: Vec<vk::SurfaceFormatKHR>) -> vk::SurfaceFormatKHR {
    for format in &formats {
      if format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR {
        return *format;
      }
    }
    formats[0]
  }

  fn refresh_swapchain(&mut self, new_size: vk::Extent2D) -> Result<(), String> {
    let (surface_format, surface_caps, present_mode) = unsafe {
      let surface_formats = self
        .vk_context
        .vk_loaders
        .surface_driver
        .get_physical_device_surface_formats(self.vk_context.gpu, self.surface)
        .map_err(|e| format!("can't get surface formats: {e}"))?;
      let surface_format = Self::select_surface_format(surface_formats);
      let surface_caps = self
        .vk_context
        .vk_loaders
        .surface_driver
        .get_physical_device_surface_capabilities(self.vk_context.gpu, self.surface)
        .map_err(|_| "Error getting surface capabilities")?;
      let present_modes = self
        .vk_context
        .vk_loaders
        .surface_driver
        .get_physical_device_surface_present_modes(self.vk_context.gpu, self.surface)
        .map_err(|_| "Error getting present modes")?;
      let present_mode = present_modes
        .iter()
        .find(|mode| **mode == vk::PresentModeKHR::MAILBOX)
        .cloned()
        .unwrap_or(vk::PresentModeKHR::FIFO);
      (surface_format, surface_caps, present_mode)
    };

    let mut desired_image_count = surface_caps.min_image_count + 1;
    if surface_caps.max_image_count > 0 && desired_image_count > surface_caps.max_image_count {
      desired_image_count = surface_caps.max_image_count;
    }

    let new_resolution = match surface_caps.current_extent.width {
      u32::MAX => vk::Extent2D::default().width(new_size.width).height(new_size.height),
      _ => surface_caps.current_extent,
    };
    let pre_transform =
      if surface_caps.supported_transforms.contains(vk::SurfaceTransformFlagsKHR::IDENTITY) {
        vk::SurfaceTransformFlagsKHR::IDENTITY
      } else {
        surface_caps.current_transform
      };

    let swapchain_info = vk::SwapchainCreateInfoKHR::default()
      .surface(self.surface)
      .old_swapchain(self.swapchain)
      .min_image_count(desired_image_count)
      .image_color_space(surface_format.color_space)
      .image_format(surface_format.format)
      .image_extent(new_resolution)
      .image_usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::COLOR_ATTACHMENT)
      .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
      .pre_transform(pre_transform)
      .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
      .present_mode(present_mode)
      .clipped(true)
      .image_array_layers(1);

    unsafe {
      let new_swapchain = self
        .swapchain_device
        .create_swapchain(&swapchain_info, None)
        .map_err(|e| format!("error creating swapchain: {e}"))?;
      let new_images = self
        .swapchain_device
        .get_swapchain_images(new_swapchain)
        .map_err(|e| format!("error getting swapchain images: {e}"))?
        .into_iter()
        .map(|x| PWImage {
          inner: x,
          format: surface_format.format,
          _type: vk::ImageType::TYPE_2D,
          resolution: vk::Extent3D::from(new_resolution).depth(1),
        })
        .collect::<Vec<_>>();

      self.vk_context.device.free_command_buffers(self.cmd_pool, &self.cmd_buffers);
      let new_cmd_buffers = self
        .vk_context
        .device
        .allocate_command_buffers(
          &vk::CommandBufferAllocateInfo::default()
            .command_pool(self.cmd_pool)
            .command_buffer_count(new_images.len() as u32)
            .level(vk::CommandBufferLevel::PRIMARY),
        )
        .map_err(|e| format!("can't allocate command buffers: {e}"))?;
      self.cmd_buffers = new_cmd_buffers;

      for sem in self.acquire_image_semaphores.drain(..) {
        self.vk_context.device.destroy_semaphore(sem, None);
      }
      self.acquire_image_semaphores.reserve(new_images.len());
      for i in 0..new_images.len() {
        let new_semaphore = self
          .vk_context
          .device
          .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
          .map_err(|e| format!("semaphore create error: {e}"))?;
        self.acquire_image_semaphores[i] = new_semaphore;
      }

      for sem in self.image_blit_semaphores.drain(..) {
        self.vk_context.device.destroy_semaphore(sem, None);
      }
      self.image_blit_semaphores.reserve(new_images.len());
      for i in 0..new_images.len() {
        let new_semaphore = self
          .vk_context
          .device
          .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
          .map_err(|e| format!("semaphore create error: {e}"))?;
        self.image_blit_semaphores[i] = new_semaphore;
      }

      self.resolution = new_resolution;
      self.swapchain = new_swapchain;
      self.images = new_images;
      self.images_init_done = [false; 3];
    }
    Ok(())
  }

  pub fn new(
    vk_context: Arc<VkContext>,
    surface: vk::SurfaceKHR,
    size: vk::Extent2D,
  ) -> Result<Self, String> {
    let swapchain_device =
      khr::swapchain::Device::new(&vk_context.vk_loaders.vk_driver, &vk_context.device);

    let cmd_pool = unsafe {
      vk_context
        .device
        .create_command_pool(
          &vk::CommandPoolCreateInfo::default()
            .queue_family_index(vk_context.graphics_q_idx)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
          None,
        )
        .map_err(|e| format!("command pool create error: {e}"))?
    };
    let mut out_data = Self {
      vk_context,
      swapchain_device,
      surface,
      cmd_pool,
      cmd_buffers: vec![],
      acquire_image_semaphores: vec![],
      image_blit_semaphores: vec![],
      images: vec![],
      swapchain: vk::SwapchainKHR::null(),
      resolution: vk::Extent2D::default(),
      image_being_presented: 0,
      images_init_done: [false; 3],
    };
    out_data.refresh_swapchain(size)?;
    Ok(out_data)
  }

  /*
  Blit the source region specified to the entirety of the next present Image and present.
  Uses its own command buffer for the blit.
  So use only once per frame, unless swapchain needs refresh.
  Make sure Image in is Transfer Src Optimal layout before calling.
   */
  pub fn present_image_content(
    &mut self,
    src_image: PWImage,
    src_subresource: vk::ImageSubresourceLayers,
    src_image_range: [vk::Offset3D; 2],
    filter: vk::Filter,
    mut wait_for: Vec<vk::Semaphore>,
  ) -> Result<(), String> {
    let image_idx = unsafe {
      match self.swapchain_device.acquire_next_image(
        self.swapchain,
        15000,
        self.acquire_image_semaphores[self.image_being_presented as usize],
        vk::Fence::null(),
      ) {
        Ok(x) => x.0 as usize,
        Err(e) => {
          return if e == vk::Result::SUBOPTIMAL_KHR || e == vk::Result::ERROR_OUT_OF_DATE_KHR {
            self.refresh_swapchain()?;
            self
              .present_image_content(src_image, src_subresource, src_image_range, filter, wait_for)
              .map_err(|e| "failed acquiring image twice")?;
            Ok(())
          } else {
            Err(format!("error acquiring image to present: {e}"))
          }
        }
      }
    };
    let barrier_before_blit = vk::ImageMemoryBarrier::default()
      .image(self.images[image_idx].inner)
      .src_queue_family_index(self.vk_context.graphics_q_idx)
      .dst_queue_family_index(self.vk_context.graphics_q_idx)
      .src_access_mask(vk::AccessFlags::MEMORY_READ)
      .dst_access_mask(vk::AccessFlags::MEMORY_WRITE)
      .old_layout(if self.images_init_done[image_idx] {
        vk::ImageLayout::UNDEFINED
      } else {
        vk::ImageLayout::PRESENT_SRC_KHR
      })
      .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
      .subresource_range(
        vk::ImageSubresourceRange::default()
          .aspect_mask(vk::ImageAspectFlags::COLOR)
          .layer_count(1)
          .base_array_layer(0)
          .level_count(1)
          .base_mip_level(0),
      );
    let blit_region = vk::ImageBlit::default()
      .src_subresource(src_subresource)
      .src_offsets(src_image_range)
      .dst_subresource(
        vk::ImageSubresourceLayers::default()
          .aspect_mask(vk::ImageAspectFlags::COLOR)
          .mip_level(0)
          .base_array_layer(0)
          .layer_count(1),
      )
      .dst_offsets([
        vk::Offset3D::default(),
        vk::Offset3D::default()
          .x(self.images[image_idx].resolution.width as i32)
          .y(self.images[image_idx].resolution.height as i32)
          .z(1),
      ]);
    let barrier_after_blit = vk::ImageMemoryBarrier::default()
      .image(self.images[image_idx].inner)
      .src_queue_family_index(self.vk_context.graphics_q_idx)
      .dst_queue_family_index(self.vk_context.graphics_q_idx)
      .src_access_mask(vk::AccessFlags::MEMORY_WRITE)
      .dst_access_mask(vk::AccessFlags::MEMORY_READ)
      .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
      .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
      .subresource_range(
        vk::ImageSubresourceRange::default()
          .aspect_mask(vk::ImageAspectFlags::COLOR)
          .layer_count(1)
          .base_array_layer(0)
          .level_count(1)
          .base_mip_level(0),
      );
    unsafe {
      self
        .vk_context
        .device
        .begin_command_buffer(self.cmd_buffers[image_idx], &vk::CommandBufferBeginInfo::default())
        .map_err(|e| format!("error beginning cmd buffer record: {e}"))?;
      self.vk_context.device.cmd_pipeline_barrier(
        self.cmd_buffers[image_idx],
        vk::PipelineStageFlags::BOTTOM_OF_PIPE,
        vk::PipelineStageFlags::TRANSFER,
        vk::DependencyFlags::BY_REGION,
        &[],
        &[],
        &[barrier_before_blit],
      );
      self.vk_context.device.cmd_blit_image(
        self.cmd_buffers[image_idx],
        src_image.inner,
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        self.images[image_idx].inner,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        &[blit_region],
        filter,
      );
      self.vk_context.device.cmd_pipeline_barrier(
        self.cmd_buffers[image_idx],
        vk::PipelineStageFlags::TRANSFER,
        vk::PipelineStageFlags::BOTTOM_OF_PIPE,
        vk::DependencyFlags::BY_REGION,
        &[],
        &[],
        &[barrier_after_blit],
      );
      wait_for.push(self.acquire_image_semaphores[self.image_being_presented as usize]);
      self
        .vk_context
        .device
        .queue_submit(
          self.vk_context.graphics_q,
          &[vk::SubmitInfo::default()
            .command_buffers(&[self.cmd_buffers[image_idx]])
            .wait_semaphores(&wait_for[..])
            .signal_semaphores(&[self.image_blit_semaphores[image_idx]])],
          vk::Fence::null(),
        )
        .map_err(|e| format!("error submitting blit cmds: {e}"))?;
      self
        .swapchain_device
        .queue_present(
          self.vk_context.present_q,
          &vk::PresentInfoKHR::default()
            .wait_semaphores(&wait_for[..])
            .swapchains(&[self.swapchain])
            .image_indices(&[image_idx as u32]),
        )
        .map_err(|e| format!("present error: {e}"))?;
    };
    Ok(())
  }
}

impl Drop for PresentManager {
  fn drop(&mut self) {
    unsafe {
      self.swapchain_device.destroy_swapchain(self.swapchain, None);
    }
  }
}
