#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[cfg(target_vendor = "apple")]
    #[error("Darwin error: {0}")]
    Darwin(objc2::rc::Retained<objc2_foundation::NSError>),
    #[error("No URL in response")]
    NoUrlInResponse,
    #[error("Invalid URL in response: {0}")]
    InvalidUrlInResponse(url::ParseError),
    #[error("Aborted")]
    Aborted,
    #[error("Needs to run on main thread")]
    NeedsToRunOnMainThread,
    #[cfg(target_os = "linux")]
    #[error("Wry error: {0}")]
    Wry(#[from] wry::Error),
    #[cfg(target_os = "linux")]
    #[error("Invalid header name: {0}")]
    InvalidHeaderName(#[from] wry::http::header::InvalidHeaderName),
    #[cfg(target_os = "linux")]
    #[error("Invalid header value: {0}")]
    InvalidHeaderValue(#[from] wry::http::header::InvalidHeaderValue),
}
