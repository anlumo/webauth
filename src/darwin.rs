use std::cell::RefCell;

use block2::RcBlock;
use objc2::{
    AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send,
    rc::{Retained, autoreleasepool},
    runtime::ProtocolObject,
};
use objc2_authentication_services::{
    ASPresentationAnchor, ASWebAuthenticationPresentationContextProviding,
    ASWebAuthenticationSession, ASWebAuthenticationSessionCallback,
};
use objc2_foundation::{NSDictionary, NSError, NSObject, NSObjectProtocol, NSString, NSURL};

use crate::Error;

pub struct WebAuthSession;

impl WebAuthSession {
    pub async fn authenticate(
        auth_url: &url::Url,
        callback_scheme: &str,
        options: crate::WebAuthOptions,
    ) -> Result<url::Url, crate::error::Error> {
        let (sender, receiver) = futures::channel::oneshot::channel();
        let sender = RefCell::new(Some(sender));
        let completion_handler = RcBlock::new(move |url: *mut NSURL, error: *mut NSError| {
            eprintln!("Completion handler called with URL: {url:?}, error: {error:?}");
            if let Some(sender) = sender.take() {
                if url.is_null() && !error.is_null() {
                    let error = unsafe { objc2::rc::Retained::retain(error) }.unwrap();
                    eprintln!(
                        "Error in ASWebAuthenticationSession: {:?}",
                        error.debugDescription()
                    );
                    sender.send(Err(Error::Darwin(error))).ok();
                } else if !url.is_null() {
                    if let Some(s) = unsafe { url.as_ref().unwrap().absoluteString() } {
                        autoreleasepool(|pool| match url::Url::parse(unsafe { s.to_str(pool) }) {
                            Ok(url) => {
                                sender.send(Ok(url)).ok();
                            }
                            Err(err) => {
                                sender.send(Err(Error::InvalidUrlInResponse(err))).ok();
                            }
                        })
                    } else {
                        sender.send(Err(Error::NoUrlInResponse)).ok();
                    }
                }
            }
        });

        eprintln!("Calling ASWebAuthenticationSession with URL: {}", auth_url);

        unsafe {
            let mtm = MainThreadMarker::new().ok_or(Error::NeedsToRunOnMainThread)?;
            let presentation_context_provider =
                PresentationContextProvider::new(mtm, options.window);
            let session = ASWebAuthenticationSession::initWithURL_callback_completionHandler(
                ASWebAuthenticationSession::alloc(),
                &NSURL::URLWithString(&NSString::from_str(auth_url.as_str())).unwrap(),
                &ASWebAuthenticationSessionCallback::callbackWithCustomScheme(&NSString::from_str(
                    callback_scheme,
                )),
                RcBlock::as_ptr(&completion_handler),
            );
            session.setPrefersEphemeralWebBrowserSession(
                options.prefers_ephemeral_web_browser_session,
            );
            if !options.additional_header_fields.is_empty() {
                let keys: Vec<_> = options
                    .additional_header_fields
                    .keys()
                    .map(|key| NSString::from_str(key))
                    .collect::<Vec<_>>();

                session.setAdditionalHeaderFields(Some(&NSDictionary::from_retained_objects::<
                    NSString,
                >(
                    &keys.iter().map(|key| key.as_ref()).collect::<Vec<_>>(),
                    &options
                        .additional_header_fields
                        .values()
                        .map(|value| NSString::from_str(value))
                        .collect::<Vec<_>>(),
                )));
            }

            let pcp = ProtocolObject::from_retained(presentation_context_provider);
            session.setPresentationContextProvider(Some(&pcp));
            session.start();
        }

        receiver.await.unwrap_or(Err(Error::Aborted))
    }
}

#[derive(Debug, Clone)]
struct Ivars {
    window: Option<Retained<objc2_app_kit::NSWindow>>,
}

define_class!(
    // SAFETY:
    // - The superclass NSObject does not have any subclassing requirements.
    // - `PresentationContextProvider` does not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    struct PresentationContextProvider;

    unsafe impl NSObjectProtocol for PresentationContextProvider {}

    unsafe impl ASWebAuthenticationPresentationContextProviding for PresentationContextProvider {
        #[unsafe(method(presentationAnchorForWebAuthenticationSession:))]
        unsafe fn presentation_anchor_for_web_authentication_session(
            &self,
            _session: &ASWebAuthenticationSession,
        ) -> *mut ASPresentationAnchor {
            if let Some(window) = &self.ivars().window {
                // If a specific window is provided, use it.
                Retained::autorelease_return(window.clone()) as *mut ASPresentationAnchor
            } else {
                // Otherwise, use the key window of the shared application.
                let mtm = MainThreadMarker::from(self);
                let key_window = objc2_app_kit::NSApplication::sharedApplication(mtm).keyWindow();
                if let Some(window) = key_window {
                    Retained::autorelease_return(window) as *mut ASPresentationAnchor
                } else {
                    eprintln!("No key window found for ASWebAuthenticationSession.");
                    std::ptr::null_mut()
                }
            }
        }
    }
);

impl PresentationContextProvider {
    fn new(
        mtm: MainThreadMarker,
        window: Option<Retained<objc2_app_kit::NSWindow>>,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Ivars { window });
        // Call `NSObject`'s `init` method.
        unsafe { msg_send![super(this), init] }
    }
}
