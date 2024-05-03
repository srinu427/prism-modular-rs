use ash::vk::{
  Bool32, DebugUtilsMessageSeverityFlagsEXT, DebugUtilsMessageTypeFlagsEXT,
  DebugUtilsMessengerCallbackDataEXT, DebugUtilsMessengerCreateInfoEXT, FALSE,
};
use std::borrow::Cow;
use std::ffi::CStr;

unsafe extern "system" fn vulkan_debug_callback(
  message_severity: DebugUtilsMessageSeverityFlagsEXT,
  message_type: DebugUtilsMessageTypeFlagsEXT,
  p_callback_data: *const DebugUtilsMessengerCallbackDataEXT,
  _user_data: *mut std::os::raw::c_void,
) -> Bool32 {
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

  FALSE
}

pub fn make_debug_mgr_create_info() -> DebugUtilsMessengerCreateInfoEXT<'static> {
  DebugUtilsMessengerCreateInfoEXT {
    message_severity: DebugUtilsMessageSeverityFlagsEXT::VERBOSE
      | DebugUtilsMessageSeverityFlagsEXT::INFO
      | DebugUtilsMessageSeverityFlagsEXT::WARNING
      | DebugUtilsMessageSeverityFlagsEXT::ERROR,
    message_type: DebugUtilsMessageTypeFlagsEXT::GENERAL
      | DebugUtilsMessageTypeFlagsEXT::VALIDATION
      | DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
    pfn_user_callback: Some(vulkan_debug_callback),
    ..Default::default()
  }
}
