use std::str::FromStr;

#[cfg(target_os = "linux")]
use gtk::{Container, glib::IsA};
use url::Url;
#[cfg(target_os = "linux")]
use wry::WebViewBuilderExtUnix;
#[cfg(target_os = "windows")]
use wry::raw_window_handle::HasWindowHandle;
use wry::{
    WebViewAttributes, WebViewBuilder,
    http::{HeaderMap, HeaderName, HeaderValue},
};

pub async fn authenticate(
    auth_url: &url::Url,
    callback_scheme: &str,
    options: crate::WebAuthOptions,
    #[cfg(target_os = "linux")] widget: &impl IsA<Container>,
    #[cfg(not(target_os = "linux"))] window: &impl HasWindowHandle,
) -> Result<url::Url, crate::error::Error> {
    tracing::trace!("Calling authenticate with URL: {auth_url}");
    let callback_scheme = format!("{callback_scheme}:");
    let (sender, receiver) = futures::channel::oneshot::channel();
    let sender = std::cell::RefCell::new(Some(sender));
    let attributes = WebViewAttributes {
        user_agent: Some("WebAuth".to_string()),
        incognito: options.prefers_ephemeral_web_browser_session,
        focused: true,
        ..Default::default()
    };

    let builder = WebViewBuilder::new_with_attributes(attributes)
        .with_navigation_handler(move |url| {
            if url.starts_with(&callback_scheme)
                && let Some(sender) = sender.take()
            {
                sender
                    .send(Url::parse(&url).map_err(crate::Error::InvalidUrlInResponse))
                    .ok();
                false
            } else {
                true
            }
        })
        .with_headers(
            options
                .additional_header_fields
                .into_iter()
                .map(|(key, value)| {
                    Ok((HeaderName::from_str(&key)?, HeaderValue::from_str(&value)?))
                })
                .collect::<Result<HeaderMap, crate::Error>>()?,
        )
        .with_url(auth_url.to_string());

    let web_view;
    #[cfg(target_os = "linux")]
    {
        web_view = builder.build_gtk(widget)?;
    }
    #[cfg(not(target_os = "linux"))]
    {
        web_view = builder.build(window)?;
    }

    let result = receiver.await.unwrap_or(Err(crate::Error::Aborted));
    drop(web_view);
    result
}
