#[derive(Debug, thiserror::Error, Clone)]
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
    #[cfg(not(target_vendor = "apple"))]
    #[error("Wry error: {0}")]
    Wry(#[from] wry::Error),
    #[cfg(not(target_vendor = "apple"))]
    #[error("Invalid header name: {0}")]
    InvalidHeaderName(#[from] wry::http::header::InvalidHeaderName),
    #[cfg(not(target_vendor = "apple"))]
    #[error("Invalid header value: {0}")]
    InvalidHeaderValue(#[from] wry::http::header::InvalidHeaderValue),
}
