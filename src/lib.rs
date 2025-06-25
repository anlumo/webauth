#[cfg(target_vendor = "apple")]
mod darwin;
mod error;

use std::collections::HashMap;

pub use error::Error;

#[cfg(target_vendor = "apple")]
pub use darwin::WebAuthSession;

#[derive(Debug, Default)]
pub struct WebAuthOptions {
    pub prefers_ephemeral_web_browser_session: bool,
    pub additional_header_fields: HashMap<String, String>,
    #[cfg(target_os = "macos")]
    pub window: Option<objc2::rc::Retained<objc2_app_kit::NSWindow>>, // None => use key window
}
