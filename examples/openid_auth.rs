use std::sync::atomic::AtomicBool;

use clap::Parser;
use futures::{FutureExt, select};
use openidconnect::OAuth2TokenResponse;
use url::Url;
use wae::{Hook, WindowHandler, WinitWindow};
use webauth::{WebAuthOptions, WebAuthSession};

#[path = "openid_auth/http_client.rs"]
mod http_client;
#[path = "openid_auth/openid.rs"]
mod openid;

#[derive(Parser)]
struct Args {
    #[clap(short, long)]
    auth_url: String,
    #[clap(short, long)]
    client_id: String,
}

fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    nyquest_preset::register();

    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
    ))]
    {
        gtk::init().unwrap();
    }

    wae::run(async move {
        let win = std::rc::Rc::new(Window::default());
        let app = std::rc::Rc::new(Application {
            auth_requested: AtomicBool::new(false),
            main_window: win.clone(),
            args,
        });

        wae::register_window(&win);
        wae::push_hook(app);

        win.close_requested.wait().await;
    })
    .unwrap();
}

struct Window {
    window: winit::window::Window,
    close_requested: wae::Signal<()>,
}

impl Default for Window {
    fn default() -> Self {
        let window = wae::create_window(
            winit::window::Window::default_attributes().with_title("WebAuth Example"),
        )
        .expect("Failed to create window");

        Self {
            window,
            close_requested: wae::Signal::default(),
        }
    }
}

impl WinitWindow for Window {
    fn id(&self) -> winit::window::WindowId {
        self.window.id()
    }
}

impl WindowHandler for Window {
    fn on_close_requested(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.close_requested
            .set(())
            .map_err(|_| "Failed to set close signal")?;
        Ok(())
    }
}

#[cfg(not(any(target_vendor = "apple", target_os = "linux")))]
struct BrowserWindow {
    window: winit::window::Window,
    close_requested: wae::Signal<()>,
}

#[cfg(not(any(target_vendor = "apple", target_os = "linux")))]
impl Default for BrowserWindow {
    fn default() -> Self {
        let window = wae::create_window(
            winit::window::Window::default_attributes().with_title("WebAuth Browser"),
        )
        .expect("Failed to create window");

        Self {
            window,
            close_requested: wae::Signal::default(),
        }
    }
}

#[cfg(not(any(target_vendor = "apple", target_os = "linux")))]
impl WinitWindow for BrowserWindow {
    fn id(&self) -> winit::window::WindowId {
        self.window.id()
    }
}

#[cfg(not(any(target_vendor = "apple", target_os = "linux")))]
impl WindowHandler for BrowserWindow {
    fn on_close_requested(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.close_requested
            .set(())
            .map_err(|_| "Failed to set close signal")?;
        Ok(())
    }
}

struct Application {
    auth_requested: AtomicBool,
    main_window: std::rc::Rc<Window>,
    args: Args,
}

impl Hook for Application {
    fn about_to_wait(
        &self,
    ) -> Result<winit::event_loop::ControlFlow, Box<dyn std::error::Error + Send + Sync>> {
        if !self
            .auth_requested
            .swap(true, std::sync::atomic::Ordering::Relaxed)
        {
            #[allow(unused)]
            let main_window = self.main_window.clone();
            let auth_url = self.args.auth_url.clone();
            let client_id = self.args.client_id.clone();
            wae::spawn(async move {
                let options = WebAuthOptions::default();
                let window;
                let window_wait;
                #[cfg(target_os = "macos")]
                {
                    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

                    let window_handle = main_window
                        .window
                        .window_handle()
                        .expect("Failed to get window handle");

                    let RawWindowHandle::AppKit(window_handle) = window_handle.as_raw() else {
                        panic!("Expected AppKit window handle");
                    };
                    let view: objc2::rc::Retained<objc2_app_kit::NSView> = unsafe {
                        use objc2::rc::Retained;
                        Retained::from_raw(window_handle.ns_view.as_ptr().cast())
                    }
                    .expect("Failed to retain NSView");
                    window = view.window().expect("Failed to get NSWindow from NSView");
                    window_wait = futures::future::pending::<()>();
                }
                #[cfg(target_os = "linux")]
                {
                    use gtk::traits::WidgetExt;

                    window = gtk::Window::builder()
                        .title("WebAuth Example")
                        .resizable(true)
                        .default_height(800)
                        .default_width(800)
                        .build();
                    window.show_all();
                    window_wait = futures::future::pending::<()>();
                }
                #[cfg(not(any(target_os = "linux", target_os = "macos")))]
                {
                    window = std::rc::Rc::new(BrowserWindow::default());
                    wae::register_window(&window);
                    window_wait = window.close_requested.wait();
                }

                select! {
                    login = openid::run(
                        auth_url,
                        client_id,
                        Url::parse("com.dungeonfog.foobar:authorized").unwrap(),
                        async |url| {
                            let result_url = WebAuthSession::authenticate(
                                &url,
                                "com.dungeonfog.foobar",
                                options,
                                #[cfg(not(any(target_os = "linux", target_os = "macos")))]
                                &window.window,
                                #[cfg(any(target_os = "linux", target_os = "macos"))]
                                &window,
                            )
                            .await?;
                            let mut code = None;
                            let mut state = None;
                            for (key, value) in result_url.query_pairs() {
                                if key == "code" {
                                    code = Some(value.to_string());
                                } else if key == "state" {
                                    state = Some(value.to_string());
                                }
                            }
                            if let Some(code) = code
                                && let Some(state) = state
                            {
                                Ok((code, state))
                            } else {
                                anyhow::bail!(
                                    "Authorization url doesn't contain both a code and a state"
                                );
                            }
                        },
                    ).fuse() => {
                        let (token, _) = login.expect("Failed openid");
                        tracing::info!("Access token: {:?}", token.access_token().secret());
                    }
                    _ = window_wait.fuse() => {
                        tracing::info!("Browser window closed prematurely.");
                    }
                }

                #[cfg(target_os = "linux")]
                {
                    use gtk::traits::GtkWindowExt;

                    window.close();
                }
                drop(window);
            });
        }
        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
        ))]
        {
            while gtk::events_pending() {
                gtk::main_iteration_do(false);
            }
            return Ok(winit::event_loop::ControlFlow::Poll);
        }
        #[allow(unused)]
        Ok(winit::event_loop::ControlFlow::Wait)
    }

    fn new_events(
        &self,
        _cause: &winit::event::StartCause,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    fn pre_window_event(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    fn window_destroyed(
        &self,
        _id: winit::window::WindowId,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    fn post_window_event(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}
