mod swapchain_manager;
mod transfer_manager;
mod vertex_pass;

pub use camera_3d::{Camera3D, CameraTransforms};
use mesh_structs::glam;
pub use mesh_structs::{Mesh, TriangleFaceInfo, Vertex};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use swapchain_manager::SwapchainManager;
use vertex_pass::VertexPass;
use vk_wrappers::structs::*;
use vk_wrappers::vk;
use vk_wrappers::{GraphicsPassGenerator, VKManager};
use vk_wrappers::vk_mem::Allocator;
use winit::window::Window;

pub trait Drawable {
  fn draw(&self, renderer: &Renderer);
}

pub struct Camera {
  position: glam::Vec4,
}

#[derive(Debug)]
pub enum RendererError {
  VKManagerInitError,
  GraphicsPassCreateError,
  GraphicsPassNotPresent,
  GraphicsPassResourceCreateFailed,
  CommandPoolCreateError,
  CommandBuffersCreateError,
  SemaphoreCreateError,
  FenceCreateError,
  DescriptorPoolCreateError,
  CommandBufferBeginError,
  CommandBufferEndError,
  QueueSubmitError,
  SwapchainManagerCreateError,
  AllocatorCreateError,
  MemoryMapFailed,
  SamplerCreateError,
}

pub struct Renderer {
  pub materials: HashMap<String, RenderableMaterial>,
  render_fences: Vec<SDFence>,
  acquire_image_semaphores: Vec<SDSemaphore>,
  render_semaphores: Vec<SDSemaphore>,
  render_command_buffers: Vec<SDCommandBuffer>,
  render_command_pool: SDCommandPool,
  transfer_command_pool: SDCommandPool,
  render_passes: HashMap<String, SDRenderPass>,
  vertex_descriptor_pool: SDDescriptorPool,
  images: HashMap<String, SDImage>,
  image_views: HashMap<String, SDImageView>,
  flat_sampler: vk::Sampler,
  buffers: HashMap<String, SDBuffer>,
  transfer_allocator: Arc<Mutex<Allocator>>,
  vertex_allocator: Arc<Mutex<Allocator>>,
  camera: Camera3D,
  swapchain_manager: SwapchainManager,
  vk_manager: Arc<VKManager>,
  window: Arc<Window>,
}

impl Renderer {
  pub fn new(window: Arc<Window>) -> Result<Self, RendererError> {
    let vk_manager =
      Arc::new(VKManager::new(Arc::clone(&window)).map_err(|_| RendererError::VKManagerInitError)?);

    let transfer_command_pool_info = vk::CommandPoolCreateInfo {
      flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
      queue_family_index: vk_manager.t_q_idx,
      ..Default::default()
    };
    let transfer_command_pool =
      SDCommandPool::new(Arc::clone(&vk_manager.device), transfer_command_pool_info)
        .map_err(|_| RendererError::CommandPoolCreateError)?;

    let render_command_pool_info = vk::CommandPoolCreateInfo {
      flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
      queue_family_index: vk_manager.g_q_idx,
      ..Default::default()
    };
    let render_command_pool =
      SDCommandPool::new(Arc::clone(&vk_manager.device), render_command_pool_info)
        .map_err(|_| RendererError::CommandPoolCreateError)?;

    let mut vertex_graphics_pass =
      VertexPass::make_gpu_render_pass(&vk_manager, vk::Format::R8G8B8A8_UNORM)
        .map_err(|_| RendererError::GraphicsPassCreateError)?;

    let render_command_buffers = render_command_pool
      .make_sd_command_buffers(vk::CommandBufferLevel::PRIMARY, 3)
      .map_err(|_| RendererError::CommandBuffersCreateError)?;

    let transfer_allocator = vk_manager
      .make_mem_allocator()
      .map_err(|_| RendererError::AllocatorCreateError)?;

    let vertex_allocator = vk_manager
      .make_mem_allocator()
      .map_err(|_| RendererError::AllocatorCreateError)?;

    let vertex_descriptor_pool = SDDescriptorPool::new(
      Arc::clone(&vk_manager.device),
      vk::DescriptorPoolCreateInfo {
        flags: vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET,
        max_sets: 3,
        pool_size_count: 1,
        p_pool_sizes: &vk::DescriptorPoolSize {
          ty: vk::DescriptorType::UNIFORM_BUFFER,
          descriptor_count: 3,
        },
        ..Default::default()
      },
    )
    .map_err(|_| RendererError::DescriptorPoolCreateError)?;

    let mut render_semaphores = Vec::with_capacity(3);
    let mut acquire_image_semaphores = Vec::with_capacity(3);
    let mut render_fences = Vec::with_capacity(3);
    for _ in 0..3 {
      let render_semaphore = SDSemaphore::new(
        Arc::clone(&vk_manager.device),
        vk::SemaphoreCreateInfo::default(),
      )
      .map_err(|_| RendererError::SemaphoreCreateError)?;
      render_semaphores.push(render_semaphore);
      let acquire_image_semaphore = SDSemaphore::new(
        Arc::clone(&vk_manager.device),
        vk::SemaphoreCreateInfo::default(),
      )
      .map_err(|_| RendererError::SemaphoreCreateError)?;
      acquire_image_semaphores.push(acquire_image_semaphore);
      let render_fence = SDFence::new(
        Arc::clone(&vk_manager.device),
        vk::FenceCreateInfo::default(),
      )
      .map_err(|_| RendererError::FenceCreateError)?;
      render_fences.push(render_fence);
    }

    let init_cmd_buffer_begin_info = vk::CommandBufferBeginInfo::default();
    unsafe {
      vk_manager
        .device
        .begin_command_buffer(render_command_buffers[0].value, &init_cmd_buffer_begin_info)
        .map_err(|_| RendererError::CommandBufferBeginError)?;
    }
    let swapchain_manager =
      SwapchainManager::new(window.inner_size(), Arc::clone(&vk_manager), None)
        .map_err(|_| RendererError::SwapchainManagerCreateError)?;
    swapchain_manager.init_images(render_command_buffers[0].value);

    VertexPass::create_per_frame_resources(
      &vk_manager,
      &mut vertex_graphics_pass,
      Arc::clone(&vertex_allocator),
      swapchain_manager.resolution,
      &vertex_descriptor_pool,
    )
    .map_err(|_| RendererError::GraphicsPassResourceCreateFailed)?;
    let _ = VertexPass::add_init_per_frame_resources_commands(
      &vk_manager,
      &vertex_graphics_pass,
      render_command_buffers[0].value,
    );

    unsafe {
      vk_manager
        .device
        .end_command_buffer(render_command_buffers[0].value)
        .map_err(|_| RendererError::CommandBufferEndError)?;
      let queue_submit_info = vk::SubmitInfo {
        command_buffer_count: 1,
        p_command_buffers: &render_command_buffers[0].value,
        ..Default::default()
      };
      vk_manager
        .device
        .queue_submit(
          vk_manager.g_queue,
          &[queue_submit_info],
          render_fences[0].value,
        )
        .map_err(|_| RendererError::QueueSubmitError)?;
    }

    let flat_sampler = unsafe{
      vk_manager.device.create_sampler(
        &vk::SamplerCreateInfo{
          anisotropy_enable: 0,
          ..Default::default()
        },
        None
      ).map_err(|_| RendererError::SamplerCreateError)?
    };

    let tile_material = RenderableMaterial::new(
      "tile_tex",
      &vk_manager,
      Arc::clone(&vertex_allocator),
      &transfer_command_pool,
      vec![PathBuf::from("tile_tex.png")],
      vk::Format::R8G8B8A8_UNORM,
      flat_sampler,
      &vertex_descriptor_pool,
      vertex_graphics_pass.per_frame_resources
    )

    let mut render_passes = HashMap::new();
    render_passes.insert("vertex".into(), vertex_graphics_pass);

    let camera = Camera3D {
      eye: glam::Vec4::new(1f32, 1f32, 1f32, 1f32),
      dir: glam::Vec4::new(-1f32, -1f32, -1f32, 0f32),
      up: glam::Vec4::new(0f32, 1f32, 0f32, 0f32),
      info: glam::Vec4::new(
        0.1f32,
        10f32,
        120f32 * (std::f32::consts::PI / 180f32),
        16f32 / 9f32,
      ),
    };

    Ok(Self {
      window,
      vk_manager,
      swapchain_manager,
      camera,
      render_passes,
      vertex_allocator,
      transfer_allocator,
      images: HashMap::new(),
      image_views: HashMap::new(),
      flat_sampler,
      buffers: HashMap::new(),
      transfer_command_pool,
      render_command_pool,
      render_command_buffers,
      render_semaphores,
      vertex_descriptor_pool,
      acquire_image_semaphores,
      render_fences,
      materials: HashMap::new(),
    })
  }

  pub fn draw(&mut self) -> Result<(), String> {
    let (next_image_idx, next_image_res) = match self
      .swapchain_manager
      .get_next_image(self.acquire_image_semaphores[self.swapchain_manager.current_image_idx].value)
    {
      Ok(x) => (x, vk::Result::SUCCESS),
      Err(e) => {
        self.swapchain_manager = SwapchainManager::new(
          self.window.inner_size(),
          Arc::clone(&self.vk_manager),
          Some(&self.swapchain_manager),
        )?;
        let next_image_idx = self
          .swapchain_manager
          .get_next_image(
            self.acquire_image_semaphores[self.swapchain_manager.current_image_idx].value,
          )
          .map_err(|_| "Error recreating swapchain")?;
        (next_image_idx, e)
      }
    };

    let camera_transforms = CameraTransforms {
      view: self.camera.get_view_matrix(),
      proj: self.camera.get_perspective_matrix(),
    };

    let vertex_render_pass = self
      .render_passes
      .get_mut("vertex")
      .ok_or("Error getting render pass for vertex rendering")?;

    unsafe {
      vertex_render_pass.per_frame_resources[next_image_idx].uniform_buffers[0]
        .allocation
        .as_mut()
        .ok_or("Uniform buffer not allocated")?
        .mapped_slice_mut()
        .ok_or("Error mounting uniform buffer")?
        .as_mut_ptr()
        .copy_from(
          (&camera_transforms as *const CameraTransforms) as *const u8,
          std::mem::size_of::<CameraTransforms>(),
        );
      if next_image_res == vk::Result::ERROR_OUT_OF_DATE_KHR {
        vertex_render_pass.per_frame_resources.clear();
        VertexPass::create_per_frame_resources(
          &self.vk_manager,
          vertex_render_pass,
          Arc::clone(&self.vertex_allocator),
          self.swapchain_manager.resolution,
          &self.vertex_descriptor_pool,
        )?;
      }
    }

    let vertex_command_buffer = self.render_command_buffers[next_image_idx].value;

    unsafe {
      self.vk_manager.device.update_descriptor_sets(
        &[vk::WriteDescriptorSet {
          dst_set: vertex_render_pass.per_frame_resources[next_image_idx].descriptor_sets[0].value,
          dst_binding: 0,
          dst_array_element: 0,
          descriptor_count: 1,
          descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
          p_buffer_info: &vk::DescriptorBufferInfo {
            buffer: vertex_render_pass.per_frame_resources[next_image_idx].uniform_buffers[0]
              .buffer,
            offset: 0,
            range: vk::WHOLE_SIZE,
          },
          ..Default::default()
        }],
        &[],
      );
      self
        .vk_manager
        .device
        .reset_command_buffer(
          vertex_command_buffer,
          vk::CommandBufferResetFlags::default(),
        )
        .map_err(|_| "Error resetting command buffer for vertex rendering")?;

      let vertex_cmd_buffer_begin_info = vk::CommandBufferBeginInfo::default();
      self
        .vk_manager
        .device
        .begin_command_buffer(vertex_command_buffer, &vertex_cmd_buffer_begin_info)
        .map_err(|_| "Error starting recording command buffer for vertex rendering")?;

      if next_image_res == vk::Result::ERROR_OUT_OF_DATE_KHR {
        self.swapchain_manager.init_images(vertex_command_buffer);
        let _ = VertexPass::add_init_per_frame_resources_commands(
          &self.vk_manager,
          vertex_render_pass,
          vertex_command_buffer,
        );
      }

      let vertex_render_pass_begin_info = vk::RenderPassBeginInfo {
        render_pass: vertex_render_pass.render_pass,
        framebuffer: vertex_render_pass.per_frame_resources[next_image_idx]
          .frame_buffer
          .value,
        render_area: vk::Rect2D {
          offset: vk::Offset2D { x: 0, y: 0 },
          extent: self.swapchain_manager.resolution,
        },
        clear_value_count: 1,
        p_clear_values: &vk::ClearValue {
          color: vk::ClearColorValue {
            uint32: [0, 0, 0, 0],
          },
        },
        ..Default::default()
      };
      self.vk_manager.device.cmd_begin_render_pass(
        vertex_command_buffer,
        &vertex_render_pass_begin_info,
        vk::SubpassContents::INLINE,
      );

      self.vk_manager.device.cmd_bind_pipeline(
        vertex_command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        vertex_render_pass.pipeline_packs[0].pipeline,
      );

      self.vk_manager.device.cmd_set_viewport(
        vertex_command_buffer,
        0,
        &[vk::Viewport {
          x: 0f32,
          y: self.swapchain_manager.resolution.height as f32,
          width: self.swapchain_manager.resolution.width as f32,
          height: -1f32 * self.swapchain_manager.resolution.height as f32,
          min_depth: 0f32,
          max_depth: 1f32,
        }],
      );
      self.vk_manager.device.cmd_set_scissor(
        vertex_command_buffer,
        0,
        &[vk::Rect2D {
          offset: Default::default(),
          extent: self.swapchain_manager.resolution,
        }],
      );

      for (_, material) in self.materials.iter() {
        self.vk_manager.device.cmd_bind_descriptor_sets(
          vertex_command_buffer,
          vk::PipelineBindPoint::GRAPHICS,
          vertex_render_pass.pipeline_packs[0].pipeline_layout,
          0,
          &[vertex_render_pass.per_frame_resources[next_image_idx].descriptor_sets[0].value],
          &[],
        );
        for mesh in material.meshes.iter() {
          self.vk_manager.device.cmd_bind_vertex_buffers(
            vertex_command_buffer,
            0,
            &[mesh.vertex_buffers[next_image_idx].buffer],
            &[0],
          );
          self.vk_manager.device.cmd_draw(
            vertex_command_buffer,
            mesh.vertex_buffers[next_image_idx].current_size as u32,
            1,
            0,
            0,
          );
        }
      }

      self
        .vk_manager
        .device
        .cmd_end_render_pass(vertex_command_buffer);
    }

    let vertex_image = &vertex_render_pass.per_frame_resources[next_image_idx].attachments[0];

    self
      .swapchain_manager
      .blit_to_image(vertex_command_buffer, vertex_image, next_image_idx);

    unsafe {
      self
        .vk_manager
        .device
        .end_command_buffer(vertex_command_buffer)
        .map_err(|_| "Error ending recording command buffer for vertex rendering")?;
    }

    let wait_semaphores =
      vec![self.acquire_image_semaphores[self.swapchain_manager.current_image_idx].value];
    let wait_stages = vec![
      vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
      vk::PipelineStageFlags::VERTEX_INPUT,
    ];

    let queue_submit_info = vk::SubmitInfo {
      wait_semaphore_count: wait_semaphores.len() as u32,
      p_wait_semaphores: wait_semaphores.as_ptr(),
      p_wait_dst_stage_mask: wait_stages.as_ptr(),
      command_buffer_count: 1,
      p_command_buffers: &vertex_command_buffer,
      signal_semaphore_count: 1,
      p_signal_semaphores: &self.render_semaphores[next_image_idx].value,
      ..Default::default()
    };

    unsafe {
      self
        .vk_manager
        .device
        .queue_submit(
          self.vk_manager.g_queue,
          &[queue_submit_info],
          self.render_fences[next_image_idx].value,
        )
        .map_err(|_| "Error when submitting render commands")?;
    }

    self.swapchain_manager.present_image(
      next_image_idx,
      vec![self.render_semaphores[next_image_idx].value],
    );
    Ok(())
  }

  pub fn set_camera(&mut self, camera3d: Camera3D) {
    self.camera = camera3d;
  }
}

impl Drop for Renderer {
  fn drop<'a>(&mut self) {
    unsafe {
      let _ = self.vk_manager.device.device_wait_idle();
    }
  }
}
