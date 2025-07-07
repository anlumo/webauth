# WebAuth

An authentication crate for Desktop applications written in Rust that have to implement web-based login workflows like openid or oauth2.

The idea is that you pass in a full URL and a URL scheme to the crate's main entry point `authenticate`. Then, the web page referenced by the URL is opened in a web browser. Whenever the page redirects to a URL of the supplied scheme, it ends the browser session and returns the full redirect URL.

The crate has been tested on Linux/Wayland, macOS and Windows. It should also run unchanged on X11 and Android, but these haven't been tested yet. iOS support is still pending and probably not a lot of work (it's mostly identical to the macOS implementation, but needs UIWindow instead of NSWindow).

## Features

- Completely independent of authentication protocol
- Uses the `ASWebAuthenticationSession` API on macOS and iOS, which is specifically designed for this
- Opens an embedded webview on the other platforms using the [wry crate](https://github.com/tauri-apps/wry) (so the platform-specific caveats of wry apply here), using the engine already installed on the system.
- Does *not* need to open a web server on localhost.

## Getting Started

The crate does not open up a window by itself, this is up to the caller. Unfortunately, this is platform specific:

* On macOS/iOS, supply a reference to the main window.
* On Linux, supply the reference to a GTK container (like a window or a view) the webview should be parented to.
* On all other systems, supply a window-like object that implements HasWindowHandle of the [raw_window_handle crate](https://github.com/rust-windowing/raw-window-handle).

The rest of the function call should be self-explanatory. It's an async function that returns the URL of that supplied scheme once the web site redirects to it. Additional header fields for the initial request can be supplied in the options, but usually it's a good idea to just use `Default::default()` for the options. It's also possible to request a private browsing session there if desired.

See [the openid_auth example](examples/openid_auth.rs) on how to use it. Note that the example fully implements openid authentication, so it's a bit more complicated than the bare minimum necessary to use the crate itself. This is especially so due to using the OS' event loop for async, HTTP requests, and waiting for the authentication, because everything has to work together here. It is using the wae crate to integrate that with winit. It also contains a connector between the openidconnect and nyquest crates, which was some quite unexpected yak shaving. Just using tokio doesn't work, because that one doesn't integrate with the OS.

## License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version 2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
</sub>
