mod render_object;
mod swapchain_manager;
mod transfer_manager;
mod triangle_pass;
mod vertex_pass;

use ash::vk;
use ash::vk::CommandBufferUsageFlags;
use gpu_allocator::vulkan::Allocator;
use gpu_allocator::MemoryLocation;
pub use mesh_structs::{Mesh, TriangleFaceInfo, Vertex};
use std::collections::HashMap;
use swapchain_manager::SwapchainManager;
use vertex_pass::VertexPass;
use vk_wrappers::structs::*;
use vk_wrappers::{GraphicsPassGenerator, VKManager};
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
    CommandBufferBeginError,
    CommandBufferEndError,
    QueueSubmitError,
    SwapchainManagerCreateError,
    AllocatorCreateError,
    MemoryMapFailed,
}

pub struct Renderer {
    pub meshes: Vec<Mesh>,
    render_fences: Vec<vk::Fence>,
    acquire_image_semaphores: Vec<vk::Semaphore>,
    sync_vertex_data_semaphores: Vec<vk::Semaphore>,
    render_semaphores: Vec<vk::Semaphore>,
    render_command_buffers: Vec<vk::CommandBuffer>,
    render_command_pool: vk::CommandPool,
    transfer_command_pool: vk::CommandPool,
    render_passes: HashMap<String, GraphicsPass>,
    images: HashMap<String, GPUImage>,
    image_views: HashMap<String, vk::ImageView>,
    vertex_buffers: Vec<GPUBuffer>,
    vertex_stage_buffers: Vec<GPUBuffer>,
    buffers: HashMap<String, GPUBuffer>,
    transfer_allocator: Allocator,
    vertex_allocator: Allocator,
    swapchain_manager: SwapchainManager,
    vk_manager: VKManager,
}

impl Renderer {
    pub fn new(window: &Window) -> Result<Self, RendererError> {
        let vk_manager = VKManager::new(window)
            .ok()
            .ok_or(RendererError::VKManagerInitError)?;

        let transfer_command_pool_info = vk::CommandPoolCreateInfo {
            flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            queue_family_index: vk_manager.t_q_idx,
            ..Default::default()
        };
        let transfer_command_pool = unsafe {
            vk_manager
                .device
                .create_command_pool(&transfer_command_pool_info, None)
                .ok()
                .ok_or(RendererError::CommandPoolCreateError)?
        };

        let render_command_pool_info = vk::CommandPoolCreateInfo {
            flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            queue_family_index: vk_manager.g_q_idx,
            ..Default::default()
        };
        let render_command_pool = unsafe {
            vk_manager
                .device
                .create_command_pool(&render_command_pool_info, None)
                .ok()
                .ok_or(RendererError::CommandPoolCreateError)?
        };

        let mut vertex_graphics_pass =
            VertexPass::make_gpu_render_pass(&vk_manager, vk::Format::R8G8B8A8_UNORM)
                .ok()
                .ok_or(RendererError::GraphicsPassCreateError)?;

        let render_command_buffer_info = vk::CommandBufferAllocateInfo {
            command_pool: render_command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: 3,
            ..Default::default()
        };
        let render_command_buffers = unsafe {
            vk_manager
                .device
                .allocate_command_buffers(&render_command_buffer_info)
                .ok()
                .ok_or(RendererError::CommandBuffersCreateError)?
        };

        let transfer_allocator = vk_manager
            .make_mem_allocator()
            .ok()
            .ok_or(RendererError::AllocatorCreateError)?;

        let mut vertex_allocator = vk_manager
            .make_mem_allocator()
            .ok()
            .ok_or(RendererError::AllocatorCreateError)?;

        let mut render_semaphores = Vec::with_capacity(3);
        let mut acquire_image_semaphores = Vec::with_capacity(3);
        let mut sync_vertex_data_semaphores = Vec::with_capacity(3);
        let mut render_fences = Vec::with_capacity(3);
        let mut vertex_buffers = Vec::with_capacity(3);
        for _ in 0..3 {
            let render_semaphore = unsafe {
                vk_manager
                    .device
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                    .ok()
                    .ok_or(RendererError::SemaphoreCreateError)?
            };
            render_semaphores.push(render_semaphore);
            let acquire_image_semaphore = unsafe {
                vk_manager
                    .device
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                    .ok()
                    .ok_or(RendererError::SemaphoreCreateError)?
            };
            acquire_image_semaphores.push(acquire_image_semaphore);
            let sync_vertex_data_semaphore = unsafe {
                vk_manager
                    .device
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                    .ok()
                    .ok_or(RendererError::SemaphoreCreateError)?
            };
            sync_vertex_data_semaphores.push(sync_vertex_data_semaphore);
            let render_fence = unsafe {
                vk_manager
                    .device
                    .create_fence(
                        &vk::FenceCreateInfo {
                            flags: vk::FenceCreateFlags::empty(),
                            ..Default::default()
                        },
                        None,
                    )
                    .ok()
                    .ok_or(RendererError::FenceCreateError)?
            };
            render_fences.push(render_fence);
            let vert_buffer = vk_manager
                .create_buffer(
                    &mut vertex_allocator,
                    "vert_buffer",
                    1024,
                    vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                    MemoryLocation::GpuOnly,
                )
                .ok()
                .ok_or(RendererError::GraphicsPassResourceCreateFailed)?;
            vertex_buffers.push(vert_buffer);
        }

        let init_cmd_buffer_begin_info = vk::CommandBufferBeginInfo::default();
        unsafe {
            vk_manager
                .device
                .begin_command_buffer(render_command_buffers[0], &init_cmd_buffer_begin_info)
                .ok()
                .ok_or(RendererError::CommandBufferBeginError)?;
        }
        let swapchain_manager = unsafe {
            SwapchainManager::new(
                window.inner_size(),
                &vk_manager,
                render_command_buffers[0],
                None,
            )
            .ok()
            .ok_or(RendererError::SwapchainManagerCreateError)?
        };

        VertexPass::create_per_frame_resources(
            &vk_manager,
            &mut vertex_graphics_pass,
            &mut vertex_allocator,
            swapchain_manager.resolution,
        )
        .ok()
        .ok_or(RendererError::GraphicsPassResourceCreateFailed)?;
        let _ = VertexPass::add_init_per_frame_resources_commands(
            &vk_manager,
            &vertex_graphics_pass,
            render_command_buffers[0],
        );

        unsafe {
            vk_manager
                .device
                .end_command_buffer(render_command_buffers[0])
                .ok()
                .ok_or(RendererError::CommandBufferEndError)?;
            let queue_submit_info = vk::SubmitInfo {
                command_buffer_count: 1,
                p_command_buffers: &render_command_buffers[0],
                ..Default::default()
            };
            vk_manager
                .device
                .queue_submit(vk_manager.g_queue, &[queue_submit_info], render_fences[0])
                .ok()
                .ok_or(RendererError::QueueSubmitError)?;
        }

        let mut render_passes = HashMap::new();
        render_passes.insert("vertex".into(), vertex_graphics_pass);

        Ok(Self {
            vk_manager,
            swapchain_manager,
            render_passes,
            vertex_allocator,
            transfer_allocator,
            images: HashMap::new(),
            image_views: HashMap::new(),
            buffers: HashMap::new(),
            vertex_buffers,
            vertex_stage_buffers: Vec::new(),
            transfer_command_pool,
            render_command_pool,
            render_command_buffers,
            render_semaphores,
            sync_vertex_data_semaphores,
            acquire_image_semaphores,
            render_fences,
            meshes: Vec::new(),
        })
    }

    pub fn refresh_resolution(&mut self, window: &Window) -> Result<(), String> {
        unsafe {
            self.vk_manager
                .device
                .wait_for_fences(
                    &[self.render_fences[self.swapchain_manager.current_image_idx]],
                    true,
                    u64::MAX,
                )
                .ok()
                .ok_or("Error waiting for fences")?;
        }
        for i in 0..3 {
            unsafe {
                self.vk_manager
                    .device
                    .reset_fences(&[self.render_fences[i]])
                    .ok()
                    .ok_or("Error resetting fences")?;
                self.vk_manager
                    .device
                    .reset_command_buffer(
                        self.render_command_buffers[i],
                        vk::CommandBufferResetFlags::default(),
                    )
                    .ok()
                    .ok_or("Error resetting command buffer for vertex rendering")?;
            }
        }
        let init_cmd_buffer_begin_info = vk::CommandBufferBeginInfo::default();
        unsafe {
            self.vk_manager
                .device
                .begin_command_buffer(self.render_command_buffers[0], &init_cmd_buffer_begin_info)
                .ok()
                .ok_or("Error starting recording command buffer for vertex rendering")?;
        }
        self.swapchain_manager = unsafe {
            SwapchainManager::new(
                window.inner_size(),
                &self.vk_manager,
                self.render_command_buffers[0],
                Some(&self.swapchain_manager),
            )
        }?;
        let mut vertex_render_pass = self
            .render_passes
            .get_mut("vertex")
            .ok_or("Error getting render pass for vertex rendering")?;

        for vertex_per_frame_resource in vertex_render_pass.per_frame_resources.drain(..) {
            unsafe {
                self.vk_manager.destroy_per_frame_render_pass_resources(
                    vertex_per_frame_resource,
                    &mut self.vertex_allocator,
                );
            }
        }
        VertexPass::create_per_frame_resources(
            &self.vk_manager,
            &mut vertex_render_pass,
            &mut self.vertex_allocator,
            self.swapchain_manager.resolution,
        )?;
        let _ = VertexPass::add_init_per_frame_resources_commands(
            &self.vk_manager,
            &mut vertex_render_pass,
            self.render_command_buffers[0],
        );

        unsafe {
            self.vk_manager
                .device
                .end_command_buffer(self.render_command_buffers[0])
                .ok()
                .ok_or("Error ending recording command buffer for init swapchain images")?;
            let queue_submit_info = vk::SubmitInfo {
                command_buffer_count: 1,
                p_command_buffers: &self.render_command_buffers[0],
                ..Default::default()
            };
            self.vk_manager
                .device
                .queue_submit(
                    self.vk_manager.g_queue,
                    &[queue_submit_info],
                    self.render_fences[0],
                )
                .ok()
                .ok_or("Error when submitting init commands")?;
        }

        Ok(())
    }

    pub fn draw(&mut self, window: &Window) -> Result<(), String> {
        let next_image_idx = match self.swapchain_manager.get_next_image(
            &self.vk_manager,
            self.acquire_image_semaphores[self.swapchain_manager.current_image_idx],
        ) {
            Ok(x) => x,
            Err(e) => {
                if e == vk::Result::ERROR_OUT_OF_DATE_KHR {
                    self.refresh_resolution(&window)?;
                }
                return Ok(());
            }
        };

        let vert_buffer = &self.vertex_buffers[next_image_idx];
        let vert_sync_cmd_buffer = unsafe {
            self.vk_manager
                .device
                .allocate_command_buffers(&vk::CommandBufferAllocateInfo {
                    command_pool: self.transfer_command_pool,
                    level: vk::CommandBufferLevel::PRIMARY,
                    command_buffer_count: 1,
                    ..Default::default()
                })
                .ok()
                .ok_or("Error cmd buffer to sync vertex buffer")?[0]
        };
        unsafe {
            self.vk_manager
                .device
                .begin_command_buffer(
                    vert_sync_cmd_buffer,
                    &vk::CommandBufferBeginInfo {
                        flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                        ..Default::default()
                    },
                )
                .ok()
                .ok_or("Error starting vertex sync comand buffer")?;
        }
        for x in self.vertex_stage_buffers.drain(..) {
            let _ = self
                .vk_manager
                .destroy_buffer(&mut self.transfer_allocator, x);
        }
        let mut stage_vertex_buffers = Vec::with_capacity(self.meshes.len());
        let mut write_offset = 0;
        for (i, mesh) in self.meshes.iter().enumerate() {
            let write_size = mesh.vertices.len() * std::mem::size_of::<Vertex>();
            let mut staging_buffer = self
                .vk_manager
                .create_buffer(
                    &mut self.transfer_allocator,
                    &format!("staging_buffer_{}", i),
                    4096,
                    vk::BufferUsageFlags::TRANSFER_SRC,
                    MemoryLocation::CpuToGpu,
                )
                .ok()
                .ok_or("Error creating staging buffer")?;
            unsafe {
                staging_buffer
                    .allocation
                    .mapped_slice_mut()
                    .ok_or("Error writing to staging buffer")?
                    .as_mut_ptr()
                    .copy_from(mesh.vertices.as_ptr() as *const u8, write_size);
                self.vk_manager.device.cmd_copy_buffer(
                    vert_sync_cmd_buffer,
                    staging_buffer.buffer,
                    self.vertex_buffers[next_image_idx].buffer,
                    &[vk::BufferCopy {
                        src_offset: 0,
                        dst_offset: write_offset,
                        size: write_size as u64,
                    }],
                );
            }
            stage_vertex_buffers.push(staging_buffer);
            write_offset += write_size as u64;
        }
        unsafe {
            self.vk_manager
                .device
                .end_command_buffer(vert_sync_cmd_buffer)
                .ok()
                .ok_or("Error ending vertex sync command buffer")?;

            self.vk_manager
                .device
                .queue_submit(
                    self.vk_manager.t_queue,
                    &[vk::SubmitInfo {
                        command_buffer_count: 1,
                        p_command_buffers: &vert_sync_cmd_buffer,
                        signal_semaphore_count: 1,
                        p_signal_semaphores: &self.sync_vertex_data_semaphores[next_image_idx],
                        ..Default::default()
                    }],
                    vk::Fence::null(),
                )
                .ok()
                .ok_or("Error submitting vertex sync commands")?;
        }

        let vertex_render_pass = self
            .render_passes
            .get("vertex")
            .ok_or("Error getting render pass for vertex rendering")?;

        let vertex_image =
            &vertex_render_pass.per_frame_resources[next_image_idx as usize].attachments[0];
        let vertex_command_buffer = self.render_command_buffers[next_image_idx as usize];

        unsafe {
            self.vk_manager
                .device
                .wait_for_fences(
                    &[self.render_fences[self.swapchain_manager.current_image_idx]],
                    true,
                    u64::MAX,
                )
                .ok()
                .ok_or("Error waiting for fences")?;
            self.vk_manager
                .device
                .reset_fences(&[self.render_fences[self.swapchain_manager.current_image_idx]])
                .ok()
                .ok_or("Error resetting fences")?;
            self.vk_manager
                .device
                .reset_command_buffer(
                    vertex_command_buffer,
                    vk::CommandBufferResetFlags::default(),
                )
                .ok()
                .ok_or("Error resetting command buffer for vertex rendering")?;

            let vertex_cmd_buffer_begin_info = vk::CommandBufferBeginInfo::default();
            self.vk_manager
                .device
                .begin_command_buffer(vertex_command_buffer, &vertex_cmd_buffer_begin_info)
                .ok()
                .ok_or("Error starting recording command buffer for vertex rendering")?;

            let vertex_render_pass_begin_info = vk::RenderPassBeginInfo {
                render_pass: vertex_render_pass.raw,
                framebuffer: vertex_render_pass.per_frame_resources[next_image_idx as usize]
                    .frame_buffer,
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
                vertex_render_pass.pipelines[0].1,
            );

            self.vk_manager.device.cmd_set_viewport(
                vertex_command_buffer,
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: self.swapchain_manager.resolution.height as f32,
                    width: self.swapchain_manager.resolution.width as f32,
                    height: -1.0 * self.swapchain_manager.resolution.height as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
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

            self.vk_manager.device.cmd_bind_vertex_buffers(
                vertex_command_buffer,
                0,
                &[vert_buffer.buffer],
                &[0],
            );

            let mut vert_offset = 0;
            for mesh in self.meshes.iter() {
                self.vk_manager.device.cmd_draw(
                    vertex_command_buffer,
                    mesh.vertices.len() as u32,
                    1,
                    vert_offset,
                    0,
                );
                vert_offset += mesh.vertices.len() as u32;
            }

            self.vk_manager
                .device
                .cmd_end_render_pass(vertex_command_buffer);
        }

        self.swapchain_manager.blit_to_image(
            &self.vk_manager,
            vertex_command_buffer,
            vertex_image,
            next_image_idx,
        );

        unsafe {
            self.vk_manager
                .device
                .end_command_buffer(vertex_command_buffer)
                .ok()
                .ok_or("Error ending recording command buffer for vertex rendering")?;
        }

        let wait_semaphores = [
            self.acquire_image_semaphores[self.swapchain_manager.current_image_idx],
            self.sync_vertex_data_semaphores[next_image_idx],
        ];
        let wait_stages = [
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
            p_signal_semaphores: &self.render_semaphores[next_image_idx as usize],
            ..Default::default()
        };

        unsafe {
            self.vk_manager
                .device
                .queue_submit(
                    self.vk_manager.g_queue,
                    &[queue_submit_info],
                    self.render_fences[next_image_idx as usize],
                )
                .ok()
                .ok_or("Error when submitting render commands")?;
        }

        self.swapchain_manager.present_image(
            &self.vk_manager,
            next_image_idx,
            vec![self.render_semaphores[next_image_idx as usize]],
        );
        self.vertex_stage_buffers = stage_vertex_buffers;
        Ok(())
    }
}

impl Drop for Renderer {
    fn drop<'a>(&mut self) {
        unsafe {
            let _ = self.vk_manager.device.device_wait_idle();
        }
        for fence in self.render_fences.clone() {
            unsafe { self.vk_manager.device.destroy_fence(fence, None) };
        }
        for semaphore in self.acquire_image_semaphores.clone() {
            unsafe { self.vk_manager.device.destroy_semaphore(semaphore, None) };
        }
        for semaphore in self.sync_vertex_data_semaphores.clone() {
            unsafe { self.vk_manager.device.destroy_semaphore(semaphore, None) };
        }
        for semaphore in self.render_semaphores.clone() {
            unsafe { self.vk_manager.device.destroy_semaphore(semaphore, None) };
        }
        unsafe {
            self.vk_manager
                .device
                .destroy_command_pool(self.render_command_pool, None);
            self.vk_manager
                .device
                .destroy_command_pool(self.transfer_command_pool, None);
        }
        for (_, render_pass) in self.render_passes.drain() {
            self.vk_manager
                .destroy_gpu_render_pass(render_pass, &mut self.vertex_allocator);
        }
        for (_, image_view) in self.image_views.drain() {
            unsafe {
                self.vk_manager.device.destroy_image_view(image_view, None);
            }
        }
        for (_, image) in self.images.drain() {
            let _ = self
                .vk_manager
                .destroy_image(&mut self.vertex_allocator, image);
        }
        for (_, buffer) in self.buffers.drain() {
            let _ = self
                .vk_manager
                .destroy_buffer(&mut self.vertex_allocator, buffer);
        }
        for buffer in self.vertex_buffers.drain(..) {
            let _ = self
                .vk_manager
                .destroy_buffer(&mut self.vertex_allocator, buffer);
        }
        for buffer in self.vertex_stage_buffers.drain(..) {
            let _ = self
                .vk_manager
                .destroy_buffer(&mut self.transfer_allocator, buffer);
        }
        unsafe {
            self.vk_manager
                .swapchain_driver
                .destroy_swapchain(self.swapchain_manager.swapchain, None);
        }
    }
}
