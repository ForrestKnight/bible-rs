//! Loads an object's bundled page (`web/`, see `src/webview.rs`) in a hidden window, the way the
//! app does, and lists every file it asks for. It fails when something isn't in the bundle or the
//! page throws, so a bundle missing a chunk is caught before the app ships with it.
//!
//! ```sh
//! cargo run -p bible-rs --example webview_check -- [translation] [seconds]
//! ```
//!
//! macOS only, like the webview; it needs a logged-in session (the window is never shown).

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("webview_check: the app's webview is macOS only");
}

#[cfg(target_os = "macos")]
fn main() {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use bible_rs::webview::{self, Message};
    use winit::application::ApplicationHandler;
    use winit::event::WindowEvent;
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
    use winit::window::{Window, WindowId};

    /// Errors the page raises, gathered by a script that runs before its own.
    const CATCH: &str = r#"window.__errors = [];
addEventListener('error', (e) => __errors.push(String(e.message || e)));
addEventListener('unhandledrejection', (e) => __errors.push('unhandled: ' + String(e.reason && e.reason.stack || e.reason)));
const __error = console.error; console.error = (...a) => { __errors.push(a.map(String).join(' ')); __error(...a); };
window.__blocked = [];
addEventListener('securitypolicyviolation', (e) => __blocked.push(e.blockedURI));"#;

    struct Check {
        translation: String,
        wait: Duration,
        started: Instant,
        window: Option<Window>,
        view: Option<wry::WebView>,
        requests: Arc<Mutex<Vec<(String, u16)>>>,
        messages: Arc<Mutex<Vec<(Duration, Message)>>>,
        errors: Arc<Mutex<Option<String>>>,
        asked: bool,
    }

    impl ApplicationHandler for Check {
        fn resumed(&mut self, el: &ActiveEventLoop) {
            if self.window.is_some() {
                return;
            }
            let attrs = Window::default_attributes()
                .with_visible(false)
                .with_inner_size(winit::dpi::LogicalSize::new(1100.0, 760.0));
            let window = el.create_window(attrs).expect("window");
            let dir = std::env::temp_dir().join(format!("bible-rs-webview-check-{}", std::process::id()));
            let library = bible_core::library::Library::open(&dir, Arc::new(|| {}));
            let ark = bible_rs::objects::ark();
            let verses =
                Arc::new(Mutex::new(Arc::new(webview::verses_json(ark, &library, &self.translation))));
            let (messages, started) = (self.messages.clone(), self.started);
            let requests = self.requests.clone();
            let url = webview::page_url(ark, &self.translation, bible_rs::theme::Theme::Light);
            println!(
                "loading {url} ({} bundled files, {} KB)",
                webview::bundled_files(),
                webview::bundled_bytes() / 1024
            );
            let view = webview::builder(
                url,
                &ark.slug,
                verses,
                move |m| messages.lock().unwrap().push((started.elapsed(), m)),
                Some(Arc::new(move |path: &str, status: u16| {
                    requests.lock().unwrap().push((path.into(), status))
                })),
            )
            .with_initialization_script(CATCH)
            .with_bounds(wry::Rect {
                position: wry::dpi::LogicalPosition::new(0.0, 0.0).into(),
                size: wry::dpi::LogicalSize::new(1100.0, 760.0).into(),
            })
            .build_as_child(&window)
            .expect("webview");
            self.window = Some(window);
            self.view = Some(view);
        }

        fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}

        fn about_to_wait(&mut self, el: &ActiveEventLoop) {
            el.set_control_flow(ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(50)));
            let ready = self.messages.lock().unwrap().iter().any(|(_, m)| *m == Message::Ready);
            let late = self.started.elapsed() > self.wait;
            if (ready || late) && !self.asked {
                self.asked = true;
                let errors = self.errors.clone();
                if let Some(view) = &self.view {
                    let _ = view.evaluate_script_with_callback(
                        "JSON.stringify({ errors: window.__errors || [], blocked: window.__blocked || [] })",
                        move |s| {
                            *errors.lock().unwrap() = Some(s);
                        },
                    );
                }
            }
            if self.asked && (self.errors.lock().unwrap().is_some() || self.started.elapsed() > self.wait * 2)
            {
                el.exit();
            }
        }
    }

    let mut args = std::env::args().skip(1);
    let translation = args.next().unwrap_or_else(|| "kjv".into());
    let wait = Duration::from_secs_f64(args.next().and_then(|s| s.parse().ok()).unwrap_or(10.0));
    let mut check = Check {
        translation,
        wait,
        started: Instant::now(),
        window: None,
        view: None,
        requests: Default::default(),
        messages: Default::default(),
        errors: Default::default(),
        asked: false,
    };
    let el = EventLoop::new().expect("event loop");
    el.run_app(&mut check).expect("event loop");

    let requests = check.requests.lock().unwrap().clone();
    let messages = check.messages.lock().unwrap().clone();
    // The script's result arrives as JSON text of the JSON the page made.
    let report: serde_json::Value = check
        .errors
        .lock()
        .unwrap()
        .as_deref()
        .and_then(|s| serde_json::from_str::<String>(s).ok().or_else(|| Some(s.to_string())))
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    for (path, status) in &requests {
        println!("{status} {path}");
    }
    for (t, m) in &messages {
        println!("message after {:.0} ms: {m:?}", t.as_secs_f64() * 1000.0);
    }
    let errors = report["errors"].as_array().cloned().unwrap_or_default();
    println!("page errors: {errors:?}");
    // The policy stops these before they reach the network.
    println!("blocked by the content security policy: {:?}", report["blocked"]);
    // Vercel's analytics, which the site's root layout adds: never bundled, and the 404 keeps it
    // off the network.
    let expected = |path: &str| path.starts_with("/_vercel/");
    let missing = requests.iter().filter(|(p, s)| *s != 200 && !expected(p)).count();
    // A hidden window never runs animation frames, so `ready` (sent from the first one) only
    // arrives when the window can draw.
    let ready = messages.iter().any(|(_, m)| *m == Message::Ready);
    if missing > 0 || !errors.is_empty() || report.is_null() {
        eprintln!("webview_check: {missing} files missing, {} page errors", errors.len());
        std::process::exit(1);
    }
    println!("ok: {} files served, no errors (ready: {ready})", requests.len());
}
