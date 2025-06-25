use std::sync::atomic::AtomicBool;

use url::Url;
use wae::{Hook, WindowHandler, WinitWindow};
use webauth::{WebAuthOptions, WebAuthSession};

fn main() {
    // let auth_url = Url::parse("https://example.com/auth").expect("Failed to parse URL");

    // let run_loop = irondash_run_loop::RunLoop::current();
    // let sender = run_loop.new_sender();
    // run_loop.spawn(async move {
    //     let result = WebAuthSession::authenticate(&auth_url, "com.dungeonfog.foobar").await;
    //     println!("Auth Result: {result:?}");

    //     sender.send(|| {
    //         RunLoop::current().stop();
    //     })
    // });

    // run_loop.run();

    wae::run(async move {
        let win = std::rc::Rc::new(Window::default());
        let app = std::rc::Rc::new(Application {
            auth_requested: AtomicBool::new(false),
            main_window: win.clone(),
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
}

impl Hook for Application {
    fn about_to_wait(
        &self,
    ) -> Result<winit::event_loop::ControlFlow, Box<dyn std::error::Error + Send + Sync>> {
        if !self
            .auth_requested
            .swap(true, std::sync::atomic::Ordering::Relaxed)
        {
            let main_window = self.main_window.clone();
            wae::spawn(async move {
                let auth_url = Url::parse("https://example.com/auth").expect("Failed to parse URL");
                let mut options = WebAuthOptions::default();
                #[cfg(target_os = "macos")]
                {
                    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

                    let window = main_window
                        .window
                        .window_handle()
                        .expect("Failed to get window handle");

                    let RawWindowHandle::AppKit(window_handle) = window.as_raw() else {
                        panic!("Expected AppKit window handle");
                    };
                    let view: objc2::rc::Retained<objc2_app_kit::NSView> = unsafe {
                        use objc2::rc::Retained;
                        Retained::from_raw(window_handle.ns_view.as_ptr().cast())
                    }
                    .expect("Failed to retain NSView");
                    let window = view.window().expect("Failed to get NSWindow from NSView");

                    options.window = Some(window);
                }
                let result =
                    WebAuthSession::authenticate(&auth_url, "com.dungeonfog.foobar", options).await;
                println!("Auth Result: {result:?}");
            });
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
