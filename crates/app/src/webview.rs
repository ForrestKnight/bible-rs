//! The website's object pages inside the app (`objects.rs`). The site draws each object in 3D at
//! `/<slug>/embed`; that page and the files it loads are bundled into the binary from `web/`
//! (made by `node site/scripts/app-web.mjs`, embedded by build.rs) and shown in a native webview
//! over the main area, under the toolbar.
//!
//! Nothing reaches the network: every file comes out of the binary over the `bible-rs:` scheme,
//! the verses the page quotes come from the app's own library (`verses.json`, answered here), and
//! a link to anywhere else opens in the browser. The page talks back with
//! `window.ipc.postMessage` ([`Message`]); the app tells it the theme and translation with
//! `window.ark.setTheme` / `setTranslation`.
//!
//! macOS only for now (wry over WKWebView): [`AVAILABLE`] is false elsewhere and the reader shows
//! no glyph. Windows can follow with WebView2, where wry serves the scheme as
//! `http://bible-rs.localhost`; that, and turning on the dependency and build.rs's table for the
//! target, is all it should take.

use std::borrow::Cow;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bible_core::books::VerseRef;
use bible_core::library::{Chapter, Library};
use eframe::egui;

use crate::objects::Object;
use crate::theme::Theme;

/// Whether this build can show the pages.
pub const AVAILABLE: bool = cfg!(target_os = "macos");

/// The custom scheme the bundled files are served on.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const SCHEME: &str = "bible-rs";

/// Where the bundled files are, as the webview sees them.
#[cfg(target_os = "macos")]
const BASE: &str = "bible-rs://localhost";

mod files {
    include!(concat!(env!("OUT_DIR"), "/web_files.rs"));
}

/// A bundled file by its path from the site's root (`/_app/immutable/…`); a page also answers
/// without its `.html`, as on the website.
pub fn file(path: &str) -> Option<&'static [u8]> {
    let path = path.trim_start_matches('/');
    let find = |p: &str| files::FILES.iter().find(|(name, _)| *name == p).map(|(_, bytes)| *bytes);
    find(path).or_else(|| find(&format!("{}.html", path.trim_end_matches('/'))))
}

/// How many files are bundled, and their total size (what the binary carries for them).
pub fn bundled_files() -> usize {
    files::FILES.len()
}

pub fn bundled_bytes() -> usize {
    files::FILES.iter().map(|(_, b)| b.len()).sum()
}

/// The `Content-Type` a file is served with.
pub fn mime(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, ext)| ext) {
        Some("html") => "text/html; charset=utf-8",
        // Module scripts load only with a JavaScript type.
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json",
        Some("wasm") => "application/wasm",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("woff2") => "font/woff2",
        Some("glb") => "model/gltf-binary",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// What the page asks of the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// `{"type":"read","ref":"Exod.25.10"}`: open the reader at a verse.
    Read(VerseRef),
    /// `{"type":"close"}` (the page's Escape): back to the reader.
    Close,
    /// `{"type":"ready"}`: the page has drawn.
    Ready,
}

pub fn parse_message(body: &str) -> Option<Message> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    Some(match v["type"].as_str()? {
        "read" => Message::Read(*crate::objects::osis_range(v["ref"].as_str()?)?.first()?),
        "close" => Message::Close,
        "ready" => Message::Ready,
        _ => return None,
    })
}

/// The theme's name in the page's address and `setTheme`.
pub fn page_theme(t: Theme) -> &'static str {
    match t {
        Theme::Light => "light",
        Theme::Sepia => "sepia",
        Theme::Dark => "dark",
    }
}

/// Tells the page the app's theme (`window.ark` in the page's Viewer.svelte).
fn set_theme(theme: Theme) -> String {
    format!("window.ark?.setTheme('{}')", page_theme(theme))
}

/// Tells the page the app's translation; it fetches `verses.json` for it again.
fn set_translation(translation: &str) -> String {
    format!("window.ark?.setTranslation({})", serde_json::json!(translation))
}

/// `/<slug>/verses.json`: the verses the object's page quotes, in `translation` and then the
/// KJV for whatever it lacks. An online translation gives only the chapters already saved on
/// this Mac; asking for the rest would reach the network.
pub fn verses_json(object: &Object, library: &Library, translation: &str) -> Vec<u8> {
    let mut ids = vec![translation];
    if translation != bible_core::bundled::DEFAULT_ID {
        ids.push(bible_core::bundled::DEFAULT_ID);
    }
    let translations: Vec<serde_json::Value> = ids
        .into_iter()
        .filter_map(|id| {
            let entry = library.get(id)?;
            let saved = entry.online().map(|_| library.cached_chapters(id));
            let mut verses = serde_json::Map::new();
            let mut chapter = None;
            for (osis, r) in &object.verses {
                let c = r.chapter_idx();
                if saved.as_ref().is_some_and(|s| !s.contains(&c)) {
                    continue;
                }
                if chapter.as_ref().is_none_or(|(at, _)| *at != c) {
                    chapter = Some((c, library.chapter(id, c)));
                }
                if let Some((_, Chapter::Ready(t))) = &chapter
                    && let Some(v) = t.find(*r)
                {
                    verses.insert(osis.clone(), v.plain_text().into());
                }
            }
            Some(serde_json::json!({
                "id": entry.meta.id,
                "abbrev": entry.meta.abbrev,
                "name": entry.meta.name,
                "verses": verses,
            }))
        })
        .collect();
    serde_json::to_vec(&serde_json::json!({ "translations": translations })).unwrap_or_default()
}

/// A bundled file, or the verses, for the webview.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn respond(path: &str, slug: &str, verses: &Mutex<Arc<Vec<u8>>>) -> (u16, &'static str, Cow<'static, [u8]>) {
    if path.trim_start_matches('/') == format!("{slug}/verses.json") {
        let body = verses.lock().map(|v| v.as_ref().clone()).unwrap_or_default();
        return (200, mime(".json"), Cow::Owned(body));
    }
    match file(path) {
        Some(bytes) => {
            let name = if path.contains('.') { path.to_string() } else { format!("{path}.html") };
            (200, mime(&name), Cow::Borrowed(bytes))
        }
        None => (404, mime(".txt"), Cow::Borrowed(b"Not in the app's bundle".as_slice())),
    }
}

/// The object's page in the bundle, for a translation and theme.
pub fn page_url(object: &Object, translation: &str, theme: Theme) -> String {
    #[cfg(target_os = "macos")]
    let base = BASE;
    #[cfg(not(target_os = "macos"))]
    let base = "";
    format!(
        "{base}/{}/embed?translation={}&theme={}",
        object.slug,
        translation.replace(|c: char| !c.is_ascii_alphanumeric() && c != '-', ""),
        page_theme(theme)
    )
}

/// What the pages may load: their own files and nothing from the network, which a navigation
/// handler alone wouldn't stop (it sees only navigations, not `fetch` or a script tag).
#[cfg(target_os = "macos")]
const POLICY: &str = "default-src 'self' bible-rs: data: blob:; script-src 'self' bible-rs: 'unsafe-inline' 'wasm-unsafe-eval'; style-src 'self' bible-rs: 'unsafe-inline'; connect-src 'self' bible-rs:";

/// Something asked of the bundle: its path and the status it got.
#[cfg(target_os = "macos")]
pub type RequestLog = Arc<dyn Fn(&str, u16) + Send + Sync>;

/// A webview on the bundled files at `url`: the scheme and its `verses.json`, the page's
/// messages, and no way out to the network (links open in the browser).
#[cfg(target_os = "macos")]
pub fn builder(
    url: String,
    slug: &str,
    verses: Arc<Mutex<Arc<Vec<u8>>>>,
    on_message: impl Fn(Message) + 'static,
    on_request: Option<RequestLog>,
) -> wry::WebViewBuilder<'static> {
    let slug = slug.to_string();
    wry::WebViewBuilder::new()
        .with_url(url)
        .with_custom_protocol(SCHEME.into(), move |_, request| {
            let path = request.uri().path();
            let (status, kind, body) = respond(path, &slug, &verses);
            if let Some(log) = &on_request {
                log(path, status);
            }
            let mut response =
                wry::http::Response::builder().status(status).header(wry::http::header::CONTENT_TYPE, kind);
            if kind.starts_with("text/html") {
                response = response.header("Content-Security-Policy", POLICY);
            }
            response.body(body).unwrap_or_else(|_| wry::http::Response::new(Cow::Borrowed(&[][..])))
        })
        .with_ipc_handler(move |request| {
            if let Some(m) = parse_message(request.body()) {
                on_message(m);
            }
        })
        // Only the bundle loads here; links out open in the browser.
        .with_navigation_handler(|url| {
            if url.starts_with(BASE) || url.starts_with("about:") {
                return true;
            }
            if url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:") {
                crate::app::open_url(&url);
            }
            false
        })
        .with_new_window_req_handler(|url, _| {
            if url.starts_with("https://") || url.starts_with("http://") {
                crate::app::open_url(&url);
            }
            wry::NewWindowResponse::Deny
        })
}

/// The page over the main area while an object is open.
#[derive(Default)]
pub struct ObjectView {
    #[cfg(target_os = "macos")]
    view: Option<wry::WebView>,
    inbox: Arc<Mutex<Vec<Message>>>,
    /// What `verses.json` answers.
    verses: Arc<Mutex<Arc<Vec<u8>>>>,
    /// Where the view is, in physical pixels, and whether it's showing.
    placed: Option<(egui::Rect, bool)>,
    theme: Option<Theme>,
    translation: String,
    opened: Option<Instant>,
    /// The view couldn't be made: don't try again every frame until the page is left.
    failed: bool,
    /// From asking for the view to the page's first drawing (its `ready`).
    pub first_draw: Option<Duration>,
}

impl ObjectView {
    pub fn is_open(&self) -> bool {
        #[cfg(target_os = "macos")]
        return self.view.is_some();
        #[cfg(not(target_os = "macos"))]
        false
    }

    /// Shows `object`'s page over `rect` (egui points), opening it the first time. While
    /// `covered`, egui is drawing over the rect (a sheet, the palette), so the view hides: a native
    /// view always sits above egui's drawing.
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        frame: &eframe::Frame,
        ctx: &egui::Context,
        object: &Object,
        rect: egui::Rect,
        covered: bool,
        theme: Theme,
        translation: &str,
        library: &Library,
    ) {
        let ppp = ctx.pixels_per_point();
        let physical = egui::Rect::from_min_max(
            (rect.min.to_vec2() * ppp).to_pos2(),
            (rect.max.to_vec2() * ppp).to_pos2(),
        );
        if self.failed {
            return;
        }
        if !self.is_open() {
            *self.verses.lock().unwrap() = Arc::new(verses_json(object, library, translation));
            self.translation = translation.to_string();
            self.theme = Some(theme);
            self.opened = Some(Instant::now());
            self.first_draw = None;
            self.open(frame, ctx, object, physical, theme, translation);
            self.placed = Some((physical, true));
        }
        if self.translation != translation {
            *self.verses.lock().unwrap() = Arc::new(verses_json(object, library, translation));
            self.translation = translation.to_string();
            self.script(&set_translation(translation));
        }
        if self.theme != Some(theme) {
            self.theme = Some(theme);
            self.script(&set_theme(theme));
        }
        let want = Some((physical, !covered));
        if self.placed != want {
            self.place(physical, !covered);
            self.placed = want;
        }
    }

    /// Takes the page's messages since the last frame.
    pub fn messages(&mut self) -> Vec<Message> {
        let out = std::mem::take(&mut *self.inbox.lock().unwrap());
        if out.contains(&Message::Ready) {
            if let Some(t) = self.opened.take() {
                self.first_draw = Some(t.elapsed());
            }
            // A change sent while the page was starting reached no `window.ark`: send the
            // settings again now that it's listening.
            if let Some(theme) = self.theme {
                self.script(&set_theme(theme));
            }
            self.script(&set_translation(&self.translation));
        }
        out
    }

    /// Closes the page and gives the keyboard back to egui.
    pub fn close(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(view) = self.view.take() {
            let _ = view.focus_parent();
        }
        self.placed = None;
        self.opened = None;
        self.failed = false;
        self.inbox.lock().unwrap().clear();
    }

    #[cfg(target_os = "macos")]
    fn open(
        &mut self,
        frame: &eframe::Frame,
        ctx: &egui::Context,
        object: &Object,
        at: egui::Rect,
        theme: Theme,
        translation: &str,
    ) {
        let inbox = self.inbox.clone();
        let ctx = ctx.clone();
        let paper = theme.palette().paper.to_array();
        let on_message = move |m| {
            inbox.lock().unwrap().push(m);
            ctx.request_repaint();
        };
        let built = builder(
            page_url(object, translation, theme),
            &object.slug,
            self.verses.clone(),
            on_message,
            None,
        )
        .with_bounds(bounds(at))
        .with_background_color((paper[0], paper[1], paper[2], 255))
        .with_accept_first_mouse(true)
        .build_as_child(frame);
        match built {
            Ok(view) => {
                let _ = view.focus();
                self.view = Some(view);
            }
            Err(e) => {
                eprintln!("bible-rs: couldn't open the {} page: {e}", object.slug);
                self.failed = true;
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn open(&mut self, _: &eframe::Frame, _: &egui::Context, _: &Object, _: egui::Rect, _: Theme, _: &str) {}

    fn place(&self, at: egui::Rect, visible: bool) {
        #[cfg(target_os = "macos")]
        if let Some(view) = &self.view {
            let _ = view.set_bounds(bounds(at));
            let _ = view.set_visible(visible);
            if !visible {
                // Keys go to whatever egui is drawing over it.
                let _ = view.focus_parent();
            }
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (at, visible);
    }

    fn script(&self, js: &str) {
        #[cfg(target_os = "macos")]
        if let Some(view) = &self.view {
            let _ = view.evaluate_script(js);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = js;
    }
}

#[cfg(target_os = "macos")]
fn bounds(at: egui::Rect) -> wry::Rect {
    wry::Rect {
        position: wry::dpi::PhysicalPosition::new(at.min.x.round() as i32, at.min.y.round() as i32).into(),
        size: wry::dpi::PhysicalSize::new(at.width().round() as u32, at.height().round() as u32).into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bible_core::books::BookId;

    #[test]
    fn messages_from_the_page() {
        assert_eq!(
            parse_message(r#"{"type":"read","ref":"Exod.25.10"}"#),
            Some(Message::Read(VerseRef::new(BookId(1), 25, 10)))
        );
        assert_eq!(parse_message(r#"{"type":"close"}"#), Some(Message::Close));
        assert_eq!(parse_message(r#"{"type":"ready"}"#), Some(Message::Ready));
        assert_eq!(parse_message(r#"{"type":"read","ref":"Nope.1.1"}"#), None);
        assert_eq!(parse_message("not json"), None);
    }

    #[test]
    fn settings_reach_the_page_as_scripts() {
        assert_eq!(set_theme(Theme::Sepia), "window.ark?.setTheme('sepia')");
        assert_eq!(set_translation("bsb"), r#"window.ark?.setTranslation("bsb")"#);
        // A translation id can't close the string it's sent in.
        assert_eq!(set_translation("a\"b"), r#"window.ark?.setTranslation("a\"b")"#);
    }

    #[test]
    fn modules_and_wasm_get_their_types() {
        assert_eq!(mime("/_app/immutable/entry/start.abc.js"), "text/javascript; charset=utf-8");
        assert_eq!(mime("/x.wasm"), "application/wasm");
        assert_eq!(mime("/ark-of-the-covenant/embed.html"), "text/html; charset=utf-8");
    }

    #[test]
    fn verses_come_from_the_library_with_the_kjv_after() {
        let dir = std::env::temp_dir().join(format!("bible-rs-webview-{}", std::process::id()));
        let library = Library::open(&dir, Arc::new(|| {}));
        let ark = crate::objects::ark();
        let json: serde_json::Value = serde_json::from_slice(&verses_json(ark, &library, "bsb")).unwrap();
        let t = json["translations"].as_array().unwrap();
        assert_eq!(t.len(), 2);
        assert_eq!(t[0]["id"], "bsb");
        assert_eq!(t[1]["id"], "kjv");
        assert!(t[0]["verses"]["Exod.25.10"].as_str().unwrap().contains("acacia"));
        assert!(t[1]["verses"]["Exod.25.10"].as_str().unwrap().contains("shittim"));
        assert_eq!(t[1]["verses"].as_object().unwrap().len(), ark.verses.len());
        let kjv: serde_json::Value = serde_json::from_slice(&verses_json(ark, &library, "kjv")).unwrap();
        assert_eq!(kjv["translations"].as_array().unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }
}
