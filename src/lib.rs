#[cfg(target_vendor = "apple")]
mod darwin;
mod error;
#[cfg(any(target_os = "linux", target_os = "windows", target_os = "android"))]
mod webview;

use std::collections::HashMap;

pub use error::Error;

#[cfg(target_vendor = "apple")]
pub use darwin::authenticate;
#[cfg(not(target_vendor = "apple"))]
pub use webview::authenticate;

#[derive(Debug, Default)]
pub struct WebAuthOptions {
    pub prefers_ephemeral_web_browser_session: bool,
    pub additional_header_fields: HashMap<String, String>,
}
