use ash::vk;
use std::sync::Arc;

pub struct AdSemaphore {
  pub(crate) device: Arc<ash::Device>,
  pub inner: vk::Semaphore,
}

impl Drop for AdSemaphore {
  fn drop(&mut self) {
    unsafe {
      self.device.destroy_semaphore(self.inner, None);
    }
  }
}

pub struct AdFence {
  pub(crate) device: Arc<ash::Device>,
  pub inner: vk::Fence,
}

impl Drop for AdFence {
  fn drop(&mut self) {
    unsafe {
      self.device.destroy_fence(self.inner, None);
    }
  }
}

pub struct AdCommandPool {
  pub(crate) device: Arc<ash::Device>,
  pub inner: vk::CommandPool,
}

impl Drop for AdCommandPool {
  fn drop(&mut self) {
    unsafe {
      self.device.destroy_command_pool(self.inner, None);
    }
  }
}
