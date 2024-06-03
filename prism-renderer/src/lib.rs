mod presentation;
mod transfer;

use crate::presentation::PresentManagerError;
use crate::transfer::TransferManager;
use presentation::PresentManager;
use std::path::PathBuf;
use std::sync::Arc;
use vk_context::gpu_allocator::vulkan::{Allocation, Allocator};
use vk_context::helpers::PWImage;
use vk_context::vk;
use vk_context::VkLoaders;
use vk_context::{HasDisplayHandle, HasWindowHandle};

pub struct Renderer {
  image: PWImage,
  allocation: Allocation,
  allocator: Allocator,
  transfer_manager: TransferManager,
  present_manager: PresentManager,
  vk_context: Arc<vk_context::VkContext>,
}

impl Renderer {
  pub fn new(
    window: &(impl HasWindowHandle + HasDisplayHandle),
    resolution_x: u32,
    resolution_y: u32,
  ) -> Result<Self, String> {
    let vk_loaders = Arc::new(VkLoaders::new()?);
    let surface = vk_loaders.make_surface(&window)?;
    let vk_context = Arc::new(vk_context::VkContext::new(vk_loaders, surface, None)?);
    let present_manager = PresentManager::new(
      Arc::clone(&vk_context),
      surface,
      vk::Extent2D { width: resolution_x, height: resolution_y },
    )
    .map_err(|e| format!("{e}"))?;

    let transfer_manager = TransferManager::new(Arc::clone(&vk_context))?;

    let mut allocator = vk_context.create_allocator()?;

    let image_path = PathBuf::from("./tile_tex.png");
    let (image, allocation) =
      transfer_manager.load_image_from_file(&mut allocator, &image_path, "display_img")?;
    Ok(Self { vk_context, present_manager, transfer_manager, image, allocator, allocation })
  }

  pub fn refresh_surface(
    &mut self,
    window: &(impl HasWindowHandle + HasDisplayHandle),
    resolution_x: u32,
    resolution_y: u32,
  ) -> Result<(), String> {
    let surface = self.vk_context.vk_loaders.make_surface(&window)?;
    let surface_support = unsafe {
      self
        .vk_context
        .vk_loaders
        .surface_driver
        .get_physical_device_surface_support(
          self.vk_context.gpu,
          self.vk_context.present_q_idx,
          surface,
        )
        .map_err(|e| format!("{e}"))?
    };
    if surface_support {
      self.present_manager = PresentManager::new(
        Arc::clone(&self.vk_context),
        surface,
        vk::Extent2D { width: resolution_x, height: resolution_y },
      )
      .map_err(|e| format!("{e}"))?;
      Ok(())
    } else {
      Err("New surface unsupported by renderer, please restart app".to_string())
    }
  }

  pub fn resize_swapchain(&mut self, resolution_x: u32, resolution_y: u32) -> Result<(), String> {
    self
      .present_manager
      .refresh_swapchain(vk::Extent2D { width: resolution_x, height: resolution_y })
      .map_err(|e| format!("{e}"))
  }

  pub fn draw(&mut self) -> bool {
    match self.present_manager.present_image_content(
      self.image,
      vk::ImageSubresourceLayers::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .mip_level(0)
        .base_array_layer(0)
        .layer_count(1),
      [
        vk::Offset3D { x: 0, y: 0, z: 0 },
        vk::Offset3D {
          x: self.image.resolution.width as i32,
          y: self.image.resolution.height as i32,
          z: self.image.resolution.depth as i32,
        },
      ],
      vk::Filter::NEAREST,
      vec![],
    ) {
      Ok(_) => {}
      Err(e) => match e {
        PresentManagerError::InitError(_) => {}
        PresentManagerError::RefreshError(_) => {}
        PresentManagerError::RefreshNeeded => return true,
        PresentManagerError::PresentError(_) => {}
      },
    };
    false
  }
}

impl Drop for Renderer {
  fn drop(&mut self) {
    unsafe {
      self.present_manager.wait_for_present();
      self.vk_context.device.destroy_image(self.image.inner, None);
    }
  }
}
