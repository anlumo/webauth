use std::{cell::RefCell, pin::Pin, task::Poll};

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

pub fn authenticate(
    auth_url: &url::Url,
    callback_scheme: &str,
    options: crate::WebAuthOptions,
    window: &objc2::rc::Retained<objc2_app_kit::NSWindow>,
    callback: impl FnOnce(Result<url::Url, crate::Error>) + 'static,
) -> Result<CancelToken, Error> {
    // NSWindow is not Send and must not be created on any other thread than the main thread
    // so this panic should never happen anyways.
    let mtm = MainThreadMarker::new().expect("NSWindow passed on non-main thread");
    let callback = RefCell::new(Some(callback));

    let completion_handler = RcBlock::new(move |url: *mut NSURL, error: *mut NSError| {
        tracing::trace!("Completion handler called with URL: {url:?}, error: {error:?}");
        if let Some(callback) = callback.take() {
            if url.is_null() && !error.is_null() {
                let error = unsafe { objc2::rc::Retained::retain(error) }.unwrap();
                tracing::error!(
                    "Error in ASWebAuthenticationSession: {:?}",
                    error.debugDescription()
                );
                callback(Err(Error::Darwin(error)));
            } else if !url.is_null() {
                if let Some(s) = unsafe { url.as_ref().unwrap().absoluteString() } {
                    autoreleasepool(|pool| match url::Url::parse(unsafe { s.to_str(pool) }) {
                        Ok(url) => {
                            callback(Ok(url));
                        }
                        Err(err) => {
                            callback(Err(Error::InvalidUrlInResponse(err)));
                        }
                    })
                } else {
                    callback(Err(Error::NoUrlInResponse));
                }
            }
        }
    });

    let presentation_context_provider = PresentationContextProvider::new(mtm, window.clone());
    tracing::trace!("Calling ASWebAuthenticationSession with URL: {auth_url}");
    let session = unsafe {
        ASWebAuthenticationSession::initWithURL_callback_completionHandler(
            ASWebAuthenticationSession::alloc(),
            &NSURL::URLWithString(&NSString::from_str(auth_url.as_str())).unwrap(),
            &ASWebAuthenticationSessionCallback::callbackWithCustomScheme(&NSString::from_str(
                callback_scheme,
            )),
            RcBlock::as_ptr(&completion_handler),
        )
    };

    unsafe {
        session.setPrefersEphemeralWebBrowserSession(options.prefers_ephemeral_web_browser_session);
    }
    if !options.additional_header_fields.is_empty() {
        let keys: Vec<_> = options
            .additional_header_fields
            .keys()
            .map(|key| NSString::from_str(key))
            .collect::<Vec<_>>();

        unsafe {
            session.setAdditionalHeaderFields(Some(
                &NSDictionary::from_retained_objects::<NSString>(
                    &keys.iter().map(|key| key.as_ref()).collect::<Vec<_>>(),
                    &options
                        .additional_header_fields
                        .values()
                        .map(|value| NSString::from_str(value))
                        .collect::<Vec<_>>(),
                ),
            ));
        }
    }

    let pcp = ProtocolObject::from_retained(presentation_context_provider);
    unsafe {
        session.setPresentationContextProvider(Some(&pcp));
        session.start();
    }

    Ok(CancelToken {
        session,
        _completion_handler: completion_handler,
    })
}

pub struct CancelToken {
    session: Retained<ASWebAuthenticationSession>,
    _completion_handler: RcBlock<dyn Fn(*mut NSURL, *mut NSError)>,
}

impl Drop for CancelToken {
    fn drop(&mut self) {
        unsafe {
            self.session.cancel();
        }
    }
}

pub fn authenticate_async(
    auth_url: &url::Url,
    callback_scheme: &str,
    options: crate::WebAuthOptions,
    window: &objc2::rc::Retained<objc2_app_kit::NSWindow>,
) -> AuthenticationFuture {
    let (sender, receiver) = futures::channel::oneshot::channel();
    let token = Some(authenticate(
        auth_url,
        callback_scheme,
        options,
        window,
        move |result| {
            sender.send(result).ok();
        },
    ));

    AuthenticationFuture { receiver, token }
}

pub struct AuthenticationFuture {
    receiver: futures::channel::oneshot::Receiver<Result<url::Url, crate::Error>>,
    token: Option<Result<CancelToken, Error>>,
}

impl std::future::Future for AuthenticationFuture {
    type Output = Result<url::Url, crate::Error>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        if let Some(Err(err)) = this.token.take_if(|token| token.is_err()) {
            return Poll::Ready(Err(err));
        }
        match Pin::new(&mut this.receiver).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(v)) => Poll::Ready(v),
            Poll::Ready(Err(_)) => Poll::Ready(Err(crate::Error::Aborted)),
        }
    }
}

#[derive(Debug, Clone)]
struct Ivars {
    window: Retained<objc2_app_kit::NSWindow>,
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
            Retained::autorelease_return(self.ivars().window.clone()) as *mut ASPresentationAnchor
        }
    }
);

impl PresentationContextProvider {
    fn new(mtm: MainThreadMarker, window: Retained<objc2_app_kit::NSWindow>) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Ivars { window });
        // Call `NSObject`'s `init` method.
        unsafe { msg_send![super(this), init] }
    }
}
