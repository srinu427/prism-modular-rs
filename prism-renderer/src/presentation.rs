use std::sync::Arc;
use vk_context::auto_drop_wrappers::{AdFence, AdSemaphore};
use vk_context::helpers::PWImage;
use vk_context::{ash::khr, ash::vk, VkContext};

#[derive(thiserror::Error, Debug)]
pub enum PresentManagerError {
  #[error("Error initializing PresentManager: {0}")]
  InitError(String),
  #[error("Error refreshing PresentManager: {0}")]
  RefreshError(String),
  #[error("Surface not in sync with swapchain call refresh_swapchain to sync it")]
  RefreshNeeded,
  #[error("Error in Blit-ting and Presenting the image: {0}")]
  PresentError(String),
}

pub struct PresentManager {
  vk_context: Arc<VkContext>,
  swapchain_device: khr::swapchain::Device,
  surface: vk::SurfaceKHR,
  cmd_pool: vk::CommandPool,
  cmd_buffers: Vec<vk::CommandBuffer>,
  acquire_image_sem_list: Vec<AdSemaphore>,
  image_blit_sem_list: Vec<AdSemaphore>,
  image_blit_fences: Vec<AdFence>,
  images: Vec<PWImage>,
  swapchain: vk::SwapchainKHR,
  resolution: vk::Extent2D,
  presenting_image: Option<u32>,
  images_init_done: [bool; 3],
  cmd_buffer_init_done: [bool; 3],
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

  pub fn refresh_swapchain(&mut self, new_size: vk::Extent2D) -> Result<(), PresentManagerError> {
    let (surface_format, surface_caps, present_mode) = unsafe {
      let surface_formats = self
        .vk_context
        .vk_loaders
        .surface_driver
        .get_physical_device_surface_formats(self.vk_context.gpu, self.surface)
        .map_err(|e| {
          PresentManagerError::RefreshError(format!("can't get surface formats: {e}"))
        })?;
      let surface_format = Self::select_surface_format(surface_formats);
      let surface_caps = self
        .vk_context
        .vk_loaders
        .surface_driver
        .get_physical_device_surface_capabilities(self.vk_context.gpu, self.surface)
        .map_err(|e| {
          PresentManagerError::RefreshError(format!("can't get surface capabilities: {e}"))
        })?;
      let present_modes = self
        .vk_context
        .vk_loaders
        .surface_driver
        .get_physical_device_surface_present_modes(self.vk_context.gpu, self.surface)
        .map_err(|e| PresentManagerError::RefreshError(format!("can't get present modes :{e}")))?;
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
        .map_err(|e| PresentManagerError::RefreshError(format!("at swapchain create: {e}")))?;
      let new_images = self
        .swapchain_device
        .get_swapchain_images(new_swapchain)
        .map_err(|e| {
          PresentManagerError::RefreshError(format!("at getting swapchain images: {e}"))
        })?
        .into_iter()
        .map(|x| PWImage {
          inner: x,
          format: surface_format.format,
          _type: vk::ImageType::TYPE_2D,
          resolution: vk::Extent3D::from(new_resolution).depth(1),
        })
        .collect::<Vec<_>>();
      self.swapchain_device.destroy_swapchain(self.swapchain, None);

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
  ) -> Result<Self, PresentManagerError> {
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
        .map_err(|e| PresentManagerError::InitError(format!("at command pool create: {e}")))?
    };

    let cmd_buffers = unsafe {
      vk_context
        .device
        .allocate_command_buffers(
          &vk::CommandBufferAllocateInfo::default()
            .command_pool(cmd_pool)
            .command_buffer_count(3)
            .level(vk::CommandBufferLevel::PRIMARY),
        )
        .map_err(|e| PresentManagerError::InitError(format!("at cmd buffer allocation: {e}")))?
    };

    let mut acquire_image_semaphores = Vec::with_capacity(3);
    for _ in 0..3 {
      let new_semaphore =
        vk_context.create_ad_semaphore().map_err(|e| PresentManagerError::RefreshError(e))?;
      acquire_image_semaphores.push(new_semaphore);
    }

    let mut image_blit_semaphores = Vec::with_capacity(3);
    for _ in 0..3 {
      let new_semaphore =
        vk_context.create_ad_semaphore().map_err(|e| PresentManagerError::RefreshError(e))?;
      image_blit_semaphores.push(new_semaphore);
    }

    let mut image_blit_fences = Vec::with_capacity(3);
    for _ in 0..3 {
      let new_fence =
        vk_context.create_ad_fence().map_err(|e| PresentManagerError::RefreshError(e))?;
      image_blit_fences.push(new_fence);
    }

    let mut out_data = Self {
      vk_context,
      swapchain_device,
      surface,
      cmd_pool,
      cmd_buffers,
      acquire_image_sem_list: acquire_image_semaphores,
      image_blit_sem_list: image_blit_semaphores,
      image_blit_fences,
      images: vec![],
      swapchain: vk::SwapchainKHR::null(),
      resolution: vk::Extent2D::default(),
      presenting_image: None,
      images_init_done: [false; 3],
      cmd_buffer_init_done: [false; 3],
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
  ) -> Result<(), PresentManagerError> {
    let image_idx = unsafe {
      match self.swapchain_device.acquire_next_image(
        self.swapchain,
        999999999,
        self.acquire_image_sem_list[self.presenting_image.unwrap_or(0) as usize].inner,
        vk::Fence::null(),
      ) {
        Ok(x) => x.0 as usize,
        Err(e) => {
          return if e == vk::Result::SUBOPTIMAL_KHR || e == vk::Result::ERROR_OUT_OF_DATE_KHR {
            Err(PresentManagerError::RefreshNeeded)
          } else {
            Err(PresentManagerError::PresentError(format!("at acquire image to present: {e}")))
          };
        }
      }
    };
    let barrier_before_blit = vk::ImageMemoryBarrier::default()
      .image(self.images[image_idx].inner)
      .src_queue_family_index(self.vk_context.graphics_q_idx)
      .dst_queue_family_index(self.vk_context.graphics_q_idx)
      .src_access_mask(vk::AccessFlags::MEMORY_READ)
      .dst_access_mask(vk::AccessFlags::MEMORY_WRITE)
      .old_layout(if !self.images_init_done[image_idx] {
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
      if self.cmd_buffer_init_done[image_idx] {
        self
          .vk_context
          .device
          .wait_for_fences(&[self.image_blit_fences[image_idx].inner], true, 15000000)
          .map_err(|e| PresentManagerError::PresentError(format!("at fence wait: {e}")))?;
        self
          .vk_context
          .device
          .reset_fences(&[self.image_blit_fences[image_idx].inner])
          .map_err(|e| PresentManagerError::PresentError(format!("at fence reset: {e}")))?;
      }
      self
        .vk_context
        .device
        .begin_command_buffer(self.cmd_buffers[image_idx], &vk::CommandBufferBeginInfo::default())
        .map_err(|e| {
          PresentManagerError::PresentError(format!("at cmd buffer record begin: {e}"))
        })?;
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

      wait_for.push(self.acquire_image_sem_list[self.presenting_image.unwrap_or(0) as usize].inner);

      self
        .vk_context
        .device
        .end_command_buffer(self.cmd_buffers[image_idx])
        .map_err(|e| PresentManagerError::PresentError(format!("at ending cmd buffer: {e}")))?;
      self
        .vk_context
        .device
        .queue_submit(
          self.vk_context.graphics_q,
          &[vk::SubmitInfo::default()
            .command_buffers(&[self.cmd_buffers[image_idx]])
            .wait_semaphores(&wait_for[..])
            .signal_semaphores(&[self.image_blit_sem_list[image_idx].inner])
            .wait_dst_stage_mask(
              &vec![vk::PipelineStageFlags::BOTTOM_OF_PIPE; wait_for.len()][..],
            )],
          self.image_blit_fences[image_idx].inner,
        )
        .map_err(|e| PresentManagerError::PresentError(format!("at blit cmd submit: {e}")))?;
      self
        .swapchain_device
        .queue_present(
          self.vk_context.present_q,
          &vk::PresentInfoKHR::default()
            .wait_semaphores(&[self.image_blit_sem_list[image_idx].inner])
            .swapchains(&[self.swapchain])
            .image_indices(&[image_idx as u32]),
        )
        .map_err(|e| PresentManagerError::PresentError(format!("at present: {e}")))?;
    };
    self.presenting_image = Some(image_idx as u32);
    self.images_init_done[image_idx] = true;
    self.cmd_buffer_init_done[image_idx] = true;
    Ok(())
  }

  pub fn wait_for_present(&mut self) {
    unsafe {
      if let Some(presenting_idx) = self.presenting_image {
        let _ = self.vk_context.device.wait_for_fences(
          &[self.image_blit_fences[presenting_idx as usize].inner],
          true,
          15000000,
        );
        let _ = self
          .vk_context
          .device
          .reset_fences(&[self.image_blit_fences[presenting_idx as usize].inner]);
      }
    }
    self.presenting_image = None;
  }
}

impl Drop for PresentManager {
  fn drop(&mut self) {
    self.wait_for_present();
    unsafe {
      self.vk_context.device.destroy_command_pool(self.cmd_pool, None);
      self.swapchain_device.destroy_swapchain(self.swapchain, None);
      self.vk_context.vk_loaders.surface_driver.destroy_surface(self.surface, None);
    }
  }
}
