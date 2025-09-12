#[cfg(target_vendor = "apple")]
mod darwin;
mod error;
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "android"))]
mod webview;

#[cfg(target_os = "linux")]
pub use gtk;

use std::collections::HashMap;

pub use error::Error;

#[cfg(target_vendor = "apple")]
pub use darwin::{CancelToken, authenticate, authenticate_async};
#[cfg(not(target_vendor = "apple"))]
pub use webview::{CancelToken, authenticate, authenticate_async};

#[cfg(target_os = "windows")]
pub use wry::raw_window_handle;

#[derive(Debug, Default)]
pub struct WebAuthOptions {
    pub prefers_ephemeral_web_browser_session: bool,
    pub additional_header_fields: HashMap<String, String>,
}
