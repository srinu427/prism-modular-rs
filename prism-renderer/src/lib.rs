mod presentation;

use presentation::PresentManager;
use presentation::PresentManagerError;
use vk_context::auto_drop_wrappers::AdAllocatedImage;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use transfer_manager::TransferManager;
use vert_mesh_pbr::structs::PbrMaterial;
use vk_context::ash::vk;
use vk_context::auto_drop_wrappers::{AdCommandBuffer, AdCommandPool, ADRenderPass};
use vk_context::gpu_allocator::vulkan::Allocator;
use vk_context::VkLoaders;
use vk_context::{HasDisplayHandle, HasWindowHandle};
use vk_context::gpu_allocator::MemoryLocation;

pub struct Renderer {
  mesh_render_pass: ADRenderPass,
  material: PbrMaterial,
  attachment_image: AdAllocatedImage,
  attachment_image_view: vk::ImageView,
  mesh_frame_buffer: vk::Framebuffer,
  render_cmd_buffer: AdCommandBuffer,
  render_cmd_pool: AdCommandPool,
  image: AdAllocatedImage,
  allocator: Arc<Mutex<Allocator>>,
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

    let allocator = Arc::new(Mutex::new(vk_context.create_allocator()?));

    let image_path = PathBuf::from("./tile_tex.png");
    let image =
      transfer_manager.load_image_from_file(Arc::clone(&allocator), &image_path, "display_img")?;

    let material = PbrMaterial::new(
      &transfer_manager,
      Arc::clone(&allocator),
      "tile_material",
      "./tile_tex.png".as_ref()
    )?;

    let mesh_render_pass = vk_context
      .create_ad_render_pass_builder(vk::RenderPassCreateFlags::default())
      .add_attachment(
        vk::AttachmentDescription::default()
          .format(vk::Format::R8G8B8A8_UNORM)
          .samples(vk::SampleCountFlags::TYPE_1)
          .initial_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
          .final_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL),
      )
      .add_sub_pass(
        vk::SubpassDescription::default()
          .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
          .color_attachments(&[vk::AttachmentReference::default()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)]),
      )
      .add_sub_pass_dependency(
        vk::SubpassDependency::default()
          .src_subpass(vk::SUBPASS_EXTERNAL)
          .dst_subpass(0)
          .src_stage_mask(vk::PipelineStageFlags::TRANSFER)
          .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
          .src_access_mask(vk::AccessFlags::TRANSFER_READ)
          .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE),
      )
      .add_sub_pass_dependency(
        vk::SubpassDependency::default()
          .src_subpass(0)
          .dst_subpass(vk::SUBPASS_EXTERNAL)
          .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
          .dst_stage_mask(vk::PipelineStageFlags::TRANSFER)
          .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
          .dst_access_mask(vk::AccessFlags::TRANSFER_READ),
      )
      .build()?;

    let attachment_image = AdAllocatedImage::new(
      Arc::clone(&vk_context.device),
      Arc::clone(&allocator),
      "attachment_image_allocation",
      vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(vk::Format::R8G8B8A8_UNORM)
        .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .mip_levels(1)
        .array_layers(1)
        .extent(
          vk::Extent3D::default()
            .width(1280)
            .height(720)
            .depth(1),
        ),
        MemoryLocation::GpuOnly
    )?;

    let attachment_image_view = unsafe {
      vk_context
        .device
        .create_image_view(
          &vk::ImageViewCreateInfo::default()
            .format(attachment_image.format)
            .image(attachment_image.inner)
            .view_type(vk::ImageViewType::TYPE_2D)
            .components(vk::ComponentMapping::default())
            .subresource_range(
              vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .level_count(1)
                .layer_count(1)
                .base_mip_level(0)
                .base_array_layer(0)
            ),
          None
        )
        .map_err(|e| format!("at attachment image view create: {e}"))?
    };

    let mesh_frame_buffer = unsafe {
      vk_context
        .device
        .create_framebuffer(
          &vk::FramebufferCreateInfo::default()
            .render_pass(mesh_render_pass.inner)
            .attachments(&[attachment_image_view])
            .width(1280)
            .height(720)
            .layers(1),
          None
        )
        .map_err(|e| format!("at frame buffer create: {e}"))?
    };

    let render_cmd_pool = vk_context
      .create_ad_command_pool(
        vk::CommandPoolCreateInfo::default()
          .queue_family_index(vk_context.graphics_q_idx)
          .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER),
      )?;

    let render_cmd_buffer = render_cmd_pool
      .allocate_command_buffers(vk::CommandBufferLevel::PRIMARY, 1)?
      .swap_remove(0);

    Ok(Self {
      mesh_render_pass,
      material,
      vk_context,
      present_manager,
      transfer_manager,
      image,
      allocator,
      attachment_image,
      attachment_image_view,
      mesh_frame_buffer,
      render_cmd_pool,
      render_cmd_buffer,
    })
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

  pub fn draw(&mut self) -> Result<bool, String> {
    self.render_cmd_buffer.begin(vk::CommandBufferBeginInfo::default())?;
    self.render_cmd_buffer.begin_render_pass(
      vk::RenderPassBeginInfo::default()
        .render_pass(self.mesh_render_pass.inner)
        .framebuffer(self.mesh_frame_buffer)
        .render_area(
          vk::Rect2D::default()
            .extent(
              vk::Extent2D::default()
                .width(self.attachment_image.resolution.width)
                .height(self.attachment_image.resolution.height)
            )
        ),
      vk::SubpassContents::INLINE
    );
    self.render_cmd_buffer.end_render_pass();
    self.render_cmd_buffer.end()?;

    match self.present_manager.present_image_content(
      &self.image,
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
        PresentManagerError::RefreshNeeded => return Ok(true),
        PresentManagerError::PresentError(_) => {}
      },
    };
    Ok(false)
  }
}

impl Drop for Renderer {
  fn drop(&mut self) {
    unsafe {
      self.present_manager.wait_for_present();
      self.vk_context.device.destroy_framebuffer(self.mesh_frame_buffer, None);
      self.vk_context.device.destroy_image_view(self.attachment_image_view, None);
    }
  }
}
