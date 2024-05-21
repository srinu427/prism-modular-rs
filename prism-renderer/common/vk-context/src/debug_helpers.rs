use ash::vk;
use std::borrow::Cow;
use std::ffi::CStr;

unsafe extern "system" fn vulkan_debug_callback(
  message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
  message_type: vk::DebugUtilsMessageTypeFlagsEXT,
  p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
  _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
  let callback_data = *p_callback_data;
  let message_id_number = callback_data.message_id_number;

  let message_id_name = if callback_data.p_message_id_name.is_null() {
    Cow::from("")
  } else {
    CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
  };

  let message = if callback_data.p_message.is_null() {
    Cow::from("")
  } else {
    CStr::from_ptr(callback_data.p_message).to_string_lossy()
  };

  println!(
    "{message_severity:?}:\n\
        {message_type:?} [{message_id_name} ({message_id_number})] : {message}",
  );

  vk::FALSE
}

pub fn make_debug_mgr_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT<'static> {
  vk::DebugUtilsMessengerCreateInfoEXT::default()
    .message_severity(
      vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
        | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
        | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
    )
    .message_type(
      vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
    )
    .pfn_user_callback(Some(vulkan_debug_callback))
}
