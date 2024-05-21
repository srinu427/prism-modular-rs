use ash::vk;

#[derive(Copy, Clone, Default)]
pub struct PWImage {
  pub inner: vk::Image,
  pub format: vk::Format,
  pub _type: vk::ImageType,
  pub resolution: vk::Extent3D,
}
