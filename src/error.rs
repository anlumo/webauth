#[derive(Debug, thiserror::Error)]
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
}
