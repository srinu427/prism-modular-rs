mod layouts;
mod texture;

use mesh_structs::{Mesh, TriangleFaceInfo, Vertex};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use vk_wrappers::structs::{SDBuffer, SDCommandPool, SDDescriptorPool, SDDescriptorSet, SDImage};
use vk_wrappers::vk;
use vk_wrappers::vk_mem::Allocator;
use vk_wrappers::VKManager;

pub struct RenderableMesh {
  name: String,
  pub vertex_buffers: Vec<SDBuffer>,
  pub index_buffers: Vec<SDBuffer>,
}

pub fn make_cube(
  vk_manager: &VKManager,
  allocator: Arc<Mutex<Allocator>>,
  command_pool: vk::CommandPool,
  name: &str,
  x: f32,
  y: f32,
  z: f32,
) -> Result<RenderableMesh, String> {
  let mesh = Mesh::new_cube(x, y, z);

  let vertex_buffer_size = mesh.vertices.len() * std::mem::size_of::<Vertex>();
  let index_buffer_size = mesh.faces.len() * 3 * std::mem::size_of::<u32>();

  let vert_upload_cmd_buffer = unsafe {
    vk_manager
      .device
      .allocate_command_buffers(&vk::CommandBufferAllocateInfo {
        command_pool,
        level: vk::CommandBufferLevel::PRIMARY,
        command_buffer_count: 1,
        ..Default::default()
      })
      .map_err(|_| "Error cmd buffer to upload vertex buffer")?[0]
  };
  unsafe {
    vk_manager
      .device
      .begin_command_buffer(
        vert_upload_cmd_buffer,
        &vk::CommandBufferBeginInfo {
          flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
          ..Default::default()
        },
      )
      .map_err(|_| "Error starting vertex upload command buffer")?;
  }

  let mut vertex_buffers = vec![];
  let mut index_buffers = vec![];
  let mut vertex_stage_buffers = vec![];
  let mut index_stage_buffers = vec![];
  for i in 0..3 {
    let vertex_buffer = vk_manager
      .create_buffer(
        Arc::clone(&allocator),
        vertex_buffer_size as vk::DeviceSize,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
      )
      .map_err(|_| "Error creating vertex buffer")?;
    let index_buffer = vk_manager
      .create_buffer(
        Arc::clone(&allocator),
        index_buffer_size as vk::DeviceSize,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
      )
      .map_err(|_| "Error creating index buffer")?;
    let mut vertex_stage_buffer = vk_manager
      .create_buffer(
        Arc::clone(&allocator),
        vertex_buffer_size as vk::DeviceSize,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE,
      )
      .map_err(|_| "Error creating vertex staging buffer")?;
    let mut index_stage_buffer = vk_manager
      .create_buffer(
        Arc::clone(&allocator),
        index_buffer_size as vk::DeviceSize,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE,
      )
      .map_err(|_| "Error creating index staging buffer")?;

    unsafe {
      allocator
        .lock()
        .map_err(|e| format!("error getting allocator lock: {e}"))?
        .map_memory(&mut vertex_stage_buffer.allocation.ok_or("unallocated buffer".to_string())?)
        .map_err(|e| format!("unable to map memory: {e}"))?
        .copy_from(mesh.vertices.as_ptr() as *const u8, vertex_buffer_size);
      allocator
        .lock()
        .map_err(|e| format!("error getting allocator lock: {e}"))?
        .unmap_memory(&mut vertex_stage_buffer.allocation.ok_or("unallocated buffer".to_string())?);
      allocator
        .lock()
        .map_err(|e| format!("error getting allocator lock: {e}"))?
        .map_memory(&mut index_stage_buffer.allocation.ok_or("unallocated buffer".to_string())?)
        .map_err(|e| format!("unable to map memory: {e}"))?
        .copy_from(mesh.vertices.as_ptr() as *const u8, vertex_buffer_size);

      index_stage_buffer
        .allocation
        .as_mut()
        .ok_or("Error accessing buffer memory")?
        .mapped_slice_mut()
        .ok_or("Error writing to index staging buffer")?
        .as_mut_ptr()
        .copy_from(
          mesh.get_draw_index_list().as_ptr() as *const u8,
          index_buffer_size,
        );
      vk_manager.device.cmd_copy_buffer(
        vert_upload_cmd_buffer,
        vertex_stage_buffer.buffer,
        vertex_buffer.buffer,
        &[vk::BufferCopy {
          src_offset: 0,
          dst_offset: 0,
          size: vertex_buffer_size as u64,
        }],
      );
      vk_manager.device.cmd_copy_buffer(
        vert_upload_cmd_buffer,
        index_stage_buffer.buffer,
        index_buffer.buffer,
        &[vk::BufferCopy {
          src_offset: 0,
          dst_offset: 0,
          size: index_buffer_size as u64,
        }],
      );
    }
    vertex_buffers.push(vertex_buffer);
    index_buffers.push(index_buffer);
    vertex_stage_buffers.push(vertex_stage_buffer);
    index_stage_buffers.push(index_stage_buffer);
  }

  unsafe {
    vk_manager
      .device
      .end_command_buffer(vert_upload_cmd_buffer)
      .map_err(|_| "Error ending vertex upload command buffer")?;

    let vert_sync_fence = vk_manager
      .device
      .create_fence(&vk::FenceCreateInfo::default(), None)
      .map_err(|_| "Error creating fence to upload vertex buffers")?;
    vk_manager
      .device
      .queue_submit(
        vk_manager.t_queue,
        &[vk::SubmitInfo {
          command_buffer_count: 1,
          p_command_buffers: &vert_upload_cmd_buffer,
          ..Default::default()
        }],
        vert_sync_fence,
      )
      .map_err(|_| "Error submitting vertex upload commands")?;
    vk_manager
      .device
      .wait_for_fences(&[vert_sync_fence], true, u64::MAX)
      .map_err(|_| "Error waiting for vertex upload fence")?;
    vk_manager.device.destroy_fence(vert_sync_fence, None);
    vk_manager
      .device
      .free_command_buffers(command_pool, &[vert_upload_cmd_buffer]);
  }

  for stage_buffer in vertex_stage_buffers {
    unsafe {
      vk_manager.device.destroy_buffer(stage_buffer.buffer, None);
    }
  }
  for stage_buffer in index_stage_buffers {
    unsafe {
      vk_manager.device.destroy_buffer(stage_buffer.buffer, None);
    }
  }

  Ok(RenderableMesh {
    name: String::from(name),
    vertex_buffers,
    index_buffers,
  })
}

pub struct RenderableMaterial {
  textures: Vec<SDImage>,
  texture_views: Vec<vk::ImageView>,
  pub descriptor_sets: Vec<SDDescriptorSet>,
  pub meshes: Vec<RenderableMesh>,
}

impl RenderableMaterial {
  pub fn new(
    name: &str,
    vk_manager: &VKManager,
    allocator: Arc<Mutex<Allocator>>,
    command_pool: &SDCommandPool,
    texture_files: Vec<PathBuf>,
    format: vk::Format,
    sampler: vk::Sampler,
    descriptor_pool: &SDDescriptorPool,
    cis_set_layout: vk::DescriptorSetLayout,
  ) -> Result<RenderableMaterial, String> {
    let texture_upload_cmd_buffer = unsafe {
      vk_manager
        .device
        .allocate_command_buffers(&vk::CommandBufferAllocateInfo {
          command_pool: command_pool.value,
          level: vk::CommandBufferLevel::PRIMARY,
          command_buffer_count: 1,
          ..Default::default()
        })
        .map_err(|_| "Error cmd buffer to upload texture")?[0]
    };
    unsafe {
      vk_manager
        .device
        .begin_command_buffer(
          texture_upload_cmd_buffer,
          &vk::CommandBufferBeginInfo {
            flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            ..Default::default()
          },
        )
        .map_err(|_| "Error starting texture upload command buffer")?;
    }
    let mut tex_stage_buffers = vec![];
    let mut textures = vec![];
    let mut texture_views = vec![];
    for (i, image_path) in texture_files.iter().enumerate() {
      let image_info = image::open(image_path).map_err(|_| "error loading image")?;
      let image_rgba8 = image_info.to_rgba8();
      let tex_image = vk_manager
        .create_2d_image(
          Arc::clone(&allocator),
          vk::Extent2D {
            width: image_info.width(),
            height: image_info.height(),
          },
          format,
          vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
          vk::MemoryPropertyFlags::DEVICE_LOCAL
        )
        .map_err(|_| "Error creating texture image")?;
      let tex_image_view = unsafe {
        vk_manager
          .device
          .create_image_view(
            &vk::ImageViewCreateInfo {
              image: tex_image.image,
              view_type: vk::ImageViewType::TYPE_2D,
              format,
              components: vk::ComponentMapping::builder()
                .r(vk::ComponentSwizzle::IDENTITY)
                .g(vk::ComponentSwizzle::IDENTITY)
                .b(vk::ComponentSwizzle::IDENTITY)
                .a(vk::ComponentSwizzle::IDENTITY)
                .build(),
              subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
              },
              ..Default::default()
            },
            None,
          )
          .map_err(|_| "Error creating texture view")?
      };
      let mut tex_stage_buffer = vk_manager
        .create_buffer(
          Arc::clone(&allocator),
          image_rgba8.len() as vk::DeviceSize,
          vk::BufferUsageFlags::TRANSFER_SRC,
          vk::MemoryPropertyFlags::HOST_VISIBLE,
        )
        .map_err(|_| "Error creating staging buffer")?;
      tex_stage_buffer
        .allocation
        .as_mut()
        .ok_or("Error accessing buffer memory")?
        .mapped_slice_mut()
        .ok_or("Error mapping stage buffer memory to CPU")?
        .copy_from_slice(image_rgba8.as_raw().as_slice());
      unsafe {
        vk_manager.device.cmd_pipeline_barrier(
          texture_upload_cmd_buffer,
          vk::PipelineStageFlags::NONE,
          vk::PipelineStageFlags::TRANSFER,
          vk::DependencyFlags::BY_REGION,
          &[],
          &[],
          &[vk::ImageMemoryBarrier {
            src_access_mask: vk::AccessFlags::NONE,
            dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            old_layout: vk::ImageLayout::UNDEFINED,
            new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            src_queue_family_index: vk_manager.t_q_idx,
            dst_queue_family_index: vk_manager.t_q_idx,
            image: tex_image.image,
            subresource_range: vk::ImageSubresourceRange {
              aspect_mask: vk::ImageAspectFlags::COLOR,
              base_mip_level: 0,
              level_count: 1,
              base_array_layer: 0,
              layer_count: 1,
            },
            ..Default::default()
          }],
        );
        vk_manager.device.cmd_copy_buffer_to_image(
          texture_upload_cmd_buffer,
          tex_stage_buffer.buffer,
          tex_image.image,
          vk::ImageLayout::TRANSFER_DST_OPTIMAL,
          &[vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers {
              aspect_mask: vk::ImageAspectFlags::COLOR,
              mip_level: 0,
              base_array_layer: 0,
              layer_count: 1,
            },
            image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
            image_extent: vk::Extent3D {
              width: tex_image.current_res.width,
              height: tex_image.current_res.height,
              depth: 1,
            },
          }],
        );
        vk_manager.device.cmd_pipeline_barrier(
          texture_upload_cmd_buffer,
          vk::PipelineStageFlags::TRANSFER,
          vk::PipelineStageFlags::BOTTOM_OF_PIPE,
          vk::DependencyFlags::BY_REGION,
          &[],
          &[],
          &[vk::ImageMemoryBarrier {
            src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            dst_access_mask: vk::AccessFlags::SHADER_READ,
            old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            src_queue_family_index: vk_manager.t_q_idx,
            dst_queue_family_index: vk_manager.t_q_idx,
            image: tex_image.image,
            subresource_range: vk::ImageSubresourceRange {
              aspect_mask: vk::ImageAspectFlags::COLOR,
              base_mip_level: 0,
              level_count: 1,
              base_array_layer: 0,
              layer_count: 1,
            },
            ..Default::default()
          }],
        );
      }
      tex_stage_buffers.push(tex_stage_buffer);
      textures.push(tex_image);
      texture_views.push(tex_image_view);
    }

    unsafe {
      vk_manager
        .device
        .end_command_buffer(texture_upload_cmd_buffer)
        .map_err(|_| "Error ending texture upload command buffer")?;
      let texture_upload_fence = vk_manager
        .device
        .create_fence(&vk::FenceCreateInfo::default(), None)
        .map_err(|_| "Error creating fence to upload textures")?;
      vk_manager
        .device
        .queue_submit(
          vk_manager.t_queue,
          &[vk::SubmitInfo {
            command_buffer_count: 1,
            p_command_buffers: &texture_upload_cmd_buffer,
            ..Default::default()
          }],
          texture_upload_fence,
        )
        .map_err(|_| "Error submitting texture upload commands")?;
      vk_manager
        .device
        .wait_for_fences(&[texture_upload_fence], true, u64::MAX)
        .map_err(|_| "Error waiting for texture upload fence")?;
      vk_manager.device.destroy_fence(texture_upload_fence, None);
      vk_manager
        .device
        .free_command_buffers(command_pool.value, &[texture_upload_cmd_buffer]);
    }

    let tex_set = descriptor_pool
      .make_sd_descriptor_sets(vec![cis_set_layout])
      .map_err(|e| "error creating tex set")?
      .remove(0);
    tex_set.write_image(
      texture_views[0],
      sampler,
      vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    );

    for stage_buffer in tex_stage_buffers {
      unsafe {
        vk_manager.device.destroy_buffer(stage_buffer.buffer, None);
      }
    }

    Ok(RenderableMaterial {
      textures,
      texture_views,
      descriptor_sets: vec![tex_set],
      meshes: vec![],
    })
  }

  pub fn add_mesh(&mut self, mesh: RenderableMesh) {
    self.meshes.push(mesh);
  }
}
