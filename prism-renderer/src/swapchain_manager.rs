use vk_wrappers::vk;
use std::sync::Arc;
use vk_wrappers::structs::*;
use vk_wrappers::VKManager;
use winit::dpi::PhysicalSize;

pub struct SwapchainManager {
    vk_manager: Arc<VKManager>,
    pub present_images: Vec<vk::Image>,
    pub swapchain: vk::SwapchainKHR,
    pub resolution: vk::Extent2D,
    pub current_image_idx: usize,
}

impl SwapchainManager {
    fn select_surface_format(formats: Vec<vk::SurfaceFormatKHR>) -> vk::SurfaceFormatKHR {
        for format in &formats {
            if format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR {
                return *format;
            }
        }
        formats[0]
    }

    pub fn new(
        window_size: PhysicalSize<u32>,
        vk_manager: Arc<VKManager>,
        old_swapchain_manager: Option<&Self>,
    ) -> Result<Self, String> {
        let surface_formats = unsafe {
            vk_manager
                .surface_driver
                .get_physical_device_surface_formats(vk_manager.gpu, vk_manager.surface)
                .ok()
                .ok_or("Error getting surface formats")?
        };
        let surface_format = Self::select_surface_format(surface_formats);
        let surface_capabilities = unsafe {
            vk_manager
                .surface_driver
                .get_physical_device_surface_capabilities(vk_manager.gpu, vk_manager.surface)
                .ok()
                .ok_or("Error getting surface capabilities")?
        };
        let mut desired_image_count = surface_capabilities.min_image_count + 1;
        if surface_capabilities.max_image_count > 0
            && desired_image_count > surface_capabilities.max_image_count
        {
            desired_image_count = surface_capabilities.max_image_count;
        }
        let surface_resolution = match surface_capabilities.current_extent.width {
            u32::MAX => vk::Extent2D {
                width: window_size.width,
                height: window_size.height,
            },
            _ => surface_capabilities.current_extent,
        };
        let pre_transform = if surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };
        let present_modes = unsafe{
            vk_manager
                .surface_driver
                .get_physical_device_surface_present_modes(vk_manager.gpu, vk_manager.surface)
                .ok()
                .ok_or("Error getting present modes")?
        };
        let present_mode = present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::FIFO);

        let mut swapchain_info = vk::SwapchainCreateInfoKHR {
            surface: vk_manager.surface,
            min_image_count: desired_image_count,
            image_color_space: surface_format.color_space,
            image_format: surface_format.format,
            image_extent: surface_resolution,
            image_usage: vk::ImageUsageFlags::TRANSFER_DST |
                vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: vk::SharingMode::EXCLUSIVE,
            pre_transform,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode,
            clipped: vk::TRUE,
            image_array_layers: 1,
            ..Default::default()
        };

        let current_image_idx = match old_swapchain_manager {
            None => { 0 }
            Some(sm) => {
                swapchain_info.old_swapchain = sm.swapchain;
                sm.current_image_idx
            },
        };

        let swapchain = unsafe {
            vk_manager
                .swapchain_driver
                .create_swapchain(&swapchain_info, None)
                .ok()
                .ok_or("Error creating swapchain")?
        };

        let present_images = unsafe {
            vk_manager
                .swapchain_driver
                .get_swapchain_images(swapchain)
                .ok()
                .ok_or("Error getting swapchain images")?
        };

        Ok(Self {
            vk_manager,
            resolution: surface_resolution,
            swapchain,
            present_images,
            current_image_idx,
        })
    }

    pub fn init_images(
        &self,
        command_buffer: vk::CommandBuffer
    ){
        unsafe {
            for i in 0..3 {
                let init_layout_barrier = vk::ImageMemoryBarrier {
                    src_access_mask: vk::AccessFlags::NONE_KHR,
                    dst_access_mask: vk::AccessFlags::MEMORY_READ,
                    old_layout: vk::ImageLayout::UNDEFINED,
                    new_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                    src_queue_family_index: self.vk_manager.g_q_idx,
                    dst_queue_family_index: self.vk_manager.g_q_idx,
                    image: self.present_images[i],
                    subresource_range: vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    },
                    ..Default::default()
                };
                self.vk_manager.device.cmd_pipeline_barrier(
                    command_buffer,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::BY_REGION,
                    &[],
                    &[],
                    &[init_layout_barrier],
                );
            }
        }
    }

    pub fn blit_to_image(
        &self,
        command_buffer: vk::CommandBuffer,
        src_image: &SDImage,
        image_idx: usize,
    ) {
        let norm_image_idx = image_idx % self.present_images.len();
        let barrier_before_blit = vk::ImageMemoryBarrier {
            src_access_mask: vk::AccessFlags::MEMORY_READ,
            dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            old_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            src_queue_family_index: self.vk_manager.g_q_idx,
            dst_queue_family_index: self.vk_manager.g_q_idx,
            image: self.present_images[norm_image_idx],
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };
        let regions = vk::ImageBlit {
            src_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            src_offsets: [
                vk::Offset3D { x: 0, y: 0, z: 0 },
                vk::Offset3D {
                    x: src_image.current_res.width as i32,
                    y: src_image.current_res.height as i32,
                    z: 1,
                },
            ],
            dst_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            dst_offsets: [
                vk::Offset3D { x: 0, y: 0, z: 0 },
                vk::Offset3D {
                    x: self.resolution.width as i32,
                    y: self.resolution.height as i32,
                    z: 1,
                },
            ],
        };
        let barrier_after_blit = vk::ImageMemoryBarrier {
            src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            dst_access_mask: vk::AccessFlags::MEMORY_READ,
            old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            new_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            src_queue_family_index: self.vk_manager.g_q_idx,
            dst_queue_family_index: self.vk_manager.g_q_idx,
            image: self.present_images[norm_image_idx],
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };
        unsafe {
            self.vk_manager.device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::BY_REGION,
                &[],
                &[],
                &[barrier_before_blit],
            );
            self.vk_manager.device.cmd_blit_image(
                command_buffer,
                src_image.image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                self.present_images[norm_image_idx],
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[regions],
                vk::Filter::LINEAR,
            );
            self.vk_manager.device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::DependencyFlags::BY_REGION,
                &[],
                &[],
                &[barrier_after_blit],
            );
        }
    }

    pub fn get_next_image(&self, send_semaphore: vk::Semaphore) -> Result<usize, vk::Result> {
        unsafe {
            Ok(self
                .vk_manager
                .swapchain_driver
                .acquire_next_image(self.swapchain, 5000, send_semaphore, vk::Fence::null())?
                .0 as usize)
        }
    }

    pub fn present_image(&mut self, image_idx: usize, wait_semaphores: Vec<vk::Semaphore>) {
        let img_indices = [image_idx as u32];
        let present_info = vk::PresentInfoKHR {
            wait_semaphore_count: wait_semaphores.len() as u32,
            p_wait_semaphores: wait_semaphores.as_ptr(),
            swapchain_count: 1,
            p_swapchains: &self.swapchain,
            p_image_indices: img_indices.as_ptr(),
            ..Default::default()
        };
        unsafe {
            match self
                .vk_manager
                .swapchain_driver
                .queue_present(self.vk_manager.g_queue, &present_info)
            {
                Ok(_) => {}
                Err(_) => {}
            };
        }
        self.current_image_idx = image_idx;
    }
}

impl Drop for SwapchainManager{
    fn drop(&mut self) {
        unsafe {
            self.vk_manager.swapchain_driver.destroy_swapchain(self.swapchain, None);
        }
    }
}
