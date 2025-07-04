use std::sync::atomic::AtomicBool;

use clap::Parser;
use openidconnect::OAuth2TokenResponse;
use url::Url;
use wae::{Hook, WindowHandler, WinitWindow};
use webauth::{WebAuthOptions, WebAuthSession};

#[path = "basic/http_client.rs"]
mod http_client;
#[path = "basic/openid.rs"]
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
                    window = Some(view.window().expect("Failed to get NSWindow from NSView"));
                }
                #[cfg(target_os = "linux")]
                {
                    use gtk::traits::WidgetExt;

                    window = gtk::Window::builder()
                        .name("WebAuth Example")
                        .resizable(true)
                        .height_request(800)
                        .width_request(800)
                        .build();
                    window.show_all();
                }
                let (token, _) = openid::run(
                    auth_url,
                    client_id,
                    Url::parse("com.dungeonfog.foobar:authorized").unwrap(),
                    async |url| {
                        let result_url = WebAuthSession::authenticate(
                            &url,
                            "com.dungeonfog.foobar",
                            options,
                            #[cfg(not(target_os = "linux"))]
                            window,
                            #[cfg(target_os = "linux")]
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
                )
                .await
                .expect("Failed openid");
                println!("Access token: {:?}", token.access_token().secret());
            });
        }
        #[cfg(any(
            target_os = "linux",
            target_os = "dragonfly",
            target_os = "freebsd",
            target_os = "netbsd",
            target_os = "openbsd",
        ))]
        while gtk::events_pending() {
            gtk::main_iteration_do(false);
        }
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
