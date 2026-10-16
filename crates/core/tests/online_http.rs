//! End-to-end tests of the online providers against a local mock HTTP server.
//! Response bodies use each API's *format* filled with public-domain (KJV) text.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bible_core::books::{BookId, ChapterIdx, VerseRef};
use bible_core::library::{Chapter, Library};
use bible_core::online::apibible::{ApiBible, list_bibles};
use bible_core::online::esv::Esv;
use bible_core::online::net::Net;
use bible_core::online::nlt::Nlt;
use bible_core::online::{OnlineConfig, ProviderKind};

/// Keeps these tests out of the user's own Keychain entries (macOS stores API keys there).
fn isolate_keychain() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    // SAFETY: every test calls this first, and `call_once` blocks the others until it is set.
    ONCE.call_once(|| unsafe { std::env::set_var("BIBLE_RS_KEYCHAIN_SERVICE", "bible-rs-tests") });
}

#[derive(Debug, Clone)]
struct Req {
    path: String,
    headers: Vec<(String, String)>,
}

type Handler = dyn Fn(&str) -> (u16, String) + Send + Sync;

fn serve(handler: Arc<Handler>) -> (String, Arc<Mutex<Vec<Req>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let log = Arc::new(Mutex::new(Vec::new()));
    let log2 = log.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                continue;
            }
            let path = line.split_whitespace().nth(1).unwrap_or("").to_string();
            let mut headers = Vec::new();
            loop {
                let mut h = String::new();
                if reader.read_line(&mut h).is_err() || h.trim().is_empty() {
                    break;
                }
                if let Some((k, v)) = h.trim().split_once(':') {
                    headers.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
                }
            }
            let (status, body) = handler(&path);
            log2.lock().unwrap().push(Req { path, headers });
            let _ = write!(
                stream,
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    (base, log)
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("bible-rs-online-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn notifier() -> (Arc<dyn Fn() + Send + Sync>, Receiver<()>) {
    let (tx, rx) = channel();
    let tx = Mutex::new(tx);
    (
        Arc::new(move || {
            let _ = tx.lock().unwrap().send(());
        }),
        rx,
    )
}

fn wait_ready(lib: &Library, id: &str, c: ChapterIdx, rx: &Receiver<()>) -> Chapter {
    for _ in 0..50 {
        match lib.chapter(id, c) {
            Chapter::Pending => {
                let _ = rx.recv_timeout(Duration::from_millis(200));
            }
            other => return other,
        }
    }
    panic!("timed out")
}

const ESV_BODY: &str = r#"{"query":"John 3","canonical":"John 3","passages":["For God So Loved the World\n\n  [16] For God so loved the world, that he gave his only begotten Son, that whosoever believeth in him should not perish, but have everlasting life.\n"]}"#;

#[test]
fn esv_fetches_with_token_and_caches_on_disk() {
    isolate_keychain();
    let (base, log) = serve(Arc::new(|path: &str| {
        if path.starts_with("/v3/passage/text/") { (200, ESV_BODY.to_string()) } else { (404, "{}".into()) }
    }));
    let dir = temp_dir("esv");
    let (notify, rx) = notifier();
    let mut lib = Library::open(&dir, notify.clone());
    let cfg = OnlineConfig {
        id: "esv".into(),
        abbrev: "ESV".into(),
        name: "English Standard Version".into(),
        provider: ProviderKind::Esv,
        copyright: String::new(),
    };
    lib.add_online_with_source(cfg.clone(), Arc::new(Esv::with_base(&base)));
    let jn3 = ChapterIdx::new(BookId(42), 3).unwrap();
    assert!(matches!(lib.chapter("esv", jn3), Chapter::NoKey));
    lib.set_api_key("esv-api-key", "SECRET").unwrap();
    let Chapter::Ready(t) = wait_ready(&lib, "esv", jn3, &rx) else { panic!("not ready") };
    assert!(t.find(VerseRef::new(BookId(42), 3, 16)).unwrap().text.starts_with("For God so loved"));
    let reqs = log.lock().unwrap().clone();
    assert_eq!(reqs.len(), 1);
    assert!(reqs[0].path.contains("q=John+3"));
    assert!(reqs[0].headers.contains(&("authorization".into(), "Token SECRET".into())));
    assert!(lib.get("esv").unwrap().copyright().contains("Crossway"));

    // A fresh session is served from the disk cache: no new request.
    let (notify2, rx2) = notifier();
    let mut lib2 = Library::open(&dir, notify2);
    lib2.add_online_with_source(cfg, Arc::new(Esv::with_base(&base)));
    assert!(matches!(wait_ready(&lib2, "esv", jn3, &rx2), Chapter::Ready(_)));
    assert_eq!(log.lock().unwrap().len(), 1);
    lib.set_api_key("esv-api-key", "").unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

const API_BIBLE_BODY: &str = r#"{"data":{"id":"JHN.3","copyright":"PUBLIC DOMAIN except in the United Kingdom","content":[
  {"name":"para","type":"tag","attrs":{"style":"p"},"items":[
    {"name":"verse","type":"tag","attrs":{"number":"16","style":"v"},"items":[{"text":"16","type":"text"}]},
    {"text":"For God so loved the world, ","type":"text"},
    {"name":"char","type":"tag","attrs":{"style":"wj"},"items":[{"text":"that he gave his only begotten Son","type":"text"}]}]}
]},"meta":{"fumsToken":"tok123"}}"#;

#[test]
fn api_bible_fetches_reports_fums_and_lists_bibles() {
    isolate_keychain();
    let (base, log) = serve(Arc::new(|path: &str| {
        if path.starts_with("/v1/bibles/abc-01/chapters/JHN.3") {
            (200, API_BIBLE_BODY.to_string())
        } else if path.starts_with("/v1/bibles/bad") {
            (401, "{}".into())
        } else if path.starts_with("/v1/bibles") {
            (200, r#"{"data":[{"id":"abc-01","name":"King James (Authorised) Version","abbreviation":"engKJV","abbreviationLocal":"KJV","language":{"name":"English"},"description":"Protestant"}]}"#.into())
        } else if path.starts_with("/f3") {
            (200, "{}".into())
        } else {
            (404, "{}".into())
        }
    }));
    let bibles = list_bibles(&format!("{base}/v1"), "K", Some("eng")).unwrap();
    assert_eq!(bibles[0].abbreviation, "KJV");
    assert_eq!(bibles[0].language, "English");
    assert!(log.lock().unwrap()[0].path.ends_with("/v1/bibles?language=eng"));

    let dir = temp_dir("apibible");
    let (notify, rx) = notifier();
    let mut lib = Library::open(&dir, notify);
    lib.set_api_key("api-bible-key", "K2").unwrap();
    let cfg = OnlineConfig {
        id: "kjvapi".into(),
        abbrev: "KJV".into(),
        name: "KJV via API.Bible".into(),
        provider: ProviderKind::ApiBible { bible_id: "abc-01".into() },
        copyright: String::new(),
    };
    let source = ApiBible::with_base(&format!("{base}/v1"), &format!("{base}/f3"), "abc-01", "device42");
    lib.add_online_with_source(cfg, Arc::new(source));
    let jn3 = ChapterIdx::new(BookId(42), 3).unwrap();
    let Chapter::Ready(t) = wait_ready(&lib, "kjvapi", jn3, &rx) else { panic!("not ready") };
    let v = t.find(VerseRef::new(BookId(42), 3, 16)).unwrap();
    assert_eq!(v.text, "For God so loved the world, that he gave his only begotten Son");
    assert_eq!(lib.get("kjvapi").unwrap().copyright(), "PUBLIC DOMAIN except in the United Kingdom");
    let req = log.lock().unwrap().iter().find(|r| r.path.contains("JHN.3")).cloned().unwrap();
    assert!(req.headers.contains(&("api-key".into(), "K2".into())));
    assert!(req.path.contains("content-type=json"));

    // Displaying the chapter reports it to FUMS once per session.
    let online = lib.get("kjvapi").unwrap().online().unwrap().clone();
    online.displayed(jn3);
    online.displayed(jn3);
    std::thread::sleep(Duration::from_millis(500));
    let fums: Vec<Req> = log.lock().unwrap().iter().filter(|r| r.path.starts_with("/f3")).cloned().collect();
    assert_eq!(fums.len(), 1);
    assert!(fums[0].path.contains("t=tok123") && fums[0].path.contains("dId=device42"));

    // A rejected key surfaces as a readable error.
    let cfg = OnlineConfig {
        id: "bad".into(),
        abbrev: "BAD".into(),
        name: "Bad".into(),
        provider: ProviderKind::ApiBible { bible_id: "bad".into() },
        copyright: String::new(),
    };
    let source = ApiBible::with_base(&format!("{base}/v1"), &format!("{base}/f3"), "bad", "d");
    lib.add_online_with_source(cfg, Arc::new(source));
    match wait_ready(&lib, "bad", jn3, &rx) {
        Chapter::Failed(m) => assert!(m.contains("rejected"), "{m}"),
        _ => panic!("expected failure"),
    }
    lib.set_api_key("api-bible-key", "").unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

/// An NLT API passage for `Book.ch.start-end`, the way the API answers: ranges past the end of
/// the chapter are clamped (a start past the end repeats the last verse). `verses` is the
/// chapter's length in the NLT. Public-domain filler words.
fn nlt_page(reference: &str, verses: u32) -> String {
    let range = reference.rsplit('.').next().unwrap_or("");
    let (start, end) = range.split_once('-').unwrap_or((range, range));
    let end = end.parse::<u32>().unwrap().min(verses);
    let start = start.parse::<u32>().unwrap().min(end);
    let mut html = String::from(
        r#"<!DOCTYPE html><html lang="en-US"><head><title>NLT API</title></head><body><div id="bibletext"><section><h2 class="bk_ch_vs_header">Passage, NLT</h2>"#,
    );
    for v in start..=end {
        html.push_str(&format!(
            r#"<verse_export orig="x_{v}" vn="{v}"><p class="body"><span class="vn">{v}</span>And it came to pass, verse {v}.</p></verse_export>"#
        ));
    }
    html.push_str("</section></div></body></html>");
    html
}

fn query<'a>(path: &'a str, name: &str) -> Option<&'a str> {
    path.split_once('?')?.1.split('&').find_map(|kv| kv.strip_prefix(name)?.strip_prefix('='))
}

#[test]
fn nlt_works_without_a_key_in_ranges_the_terms_allow() {
    isolate_keychain();
    let (base, log) = serve(Arc::new(|path: &str| {
        let Some(r) = query(path, "ref").filter(|_| path.starts_with("/api/passages")) else {
            return (404, String::new());
        };
        let verses = if r.starts_with("Ps.119.") {
            176
        } else if r.starts_with("3Jn.1.") {
            15 // one more than the KJV
        } else {
            36
        };
        (200, nlt_page(r, verses))
    }));
    let dir = temp_dir("nlt");
    std::fs::create_dir_all(&dir).unwrap();
    let (notify, rx) = notifier();
    let mut lib = Library::open(&dir, notify);
    let cfg = OnlineConfig {
        id: "nlt".into(),
        abbrev: "NLT".into(),
        name: "New Living Translation".into(),
        provider: ProviderKind::Nlt,
        copyright: String::new(),
    };
    let budget = dir.join("nlt-requests");
    let nlt = || Nlt::with_base(&base, Some(budget.clone())).with_interval(Duration::ZERO);
    lib.add_online_with_source(cfg.clone(), Arc::new(nlt()));

    // No key: chapters load anyway, at most 50 verses a request (`key=TEST` is anonymous access).
    let ps119 = ChapterIdx::new(BookId::PSALMS, 119).unwrap();
    let Chapter::Ready(t) = wait_ready(&lib, "nlt", ps119, &rx) else { panic!("not ready") };
    assert_eq!(t.verses_in(ps119).len(), 176);
    let refs: Vec<String> =
        log.lock().unwrap().iter().map(|r| query(&r.path, "ref").unwrap().to_string()).collect();
    assert_eq!(refs, ["Ps.119.1-50", "Ps.119.51-100", "Ps.119.101-150", "Ps.119.151-200"]);
    assert!(log.lock().unwrap().iter().all(|r| query(&r.path, "key") == Some("TEST")));
    assert!(lib.get("nlt").unwrap().copyright().contains("Tyndale House Foundation"));
    // The NLT numbers 15 verses in 3 John, the KJV 14: the last one isn't lost.
    let jn3 = ChapterIdx::new(BookId(63), 1).unwrap();
    let Chapter::Ready(t) = wait_ready(&lib, "nlt", jn3, &rx) else { panic!("not ready") };
    assert_eq!(t.verses_in(jn3).last().unwrap().number, 15);
    // Every request counted toward the day's 500.
    assert_eq!(std::fs::read_to_string(&budget).unwrap().split(' ').nth(1), Some("5"));

    // With a free key: 500 verses a request, so one request per chapter.
    log.lock().unwrap().clear();
    let (notify, rx) = notifier();
    let key_dir = temp_dir("nlt-key");
    let mut lib = Library::open(&key_dir, notify);
    lib.set_api_key("nlt-api-key", "K 1").unwrap();
    lib.add_online_with_source(cfg.clone(), Arc::new(nlt()));
    assert!(matches!(wait_ready(&lib, "nlt", ps119, &rx), Chapter::Ready(_)));
    let reqs = log.lock().unwrap().clone();
    assert_eq!(reqs.len(), 1);
    assert_eq!(query(&reqs[0].path, "ref"), Some("Ps.119.1-500"));
    assert_eq!(query(&reqs[0].path, "key"), Some("K%201"));
    lib.set_api_key("nlt-api-key", "").unwrap();

    // Once the day's requests are used up, no more are made and the reason is shown.
    let today =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() / 86_400;
    std::fs::write(&budget, format!("{today} 500")).unwrap();
    log.lock().unwrap().clear();
    let (notify, rx) = notifier();
    let spent_dir = temp_dir("nlt-spent");
    let mut lib = Library::open(&spent_dir, notify);
    lib.add_online_with_source(cfg, Arc::new(nlt()));
    let jn3 = ChapterIdx::new(BookId(42), 3).unwrap();
    match wait_ready(&lib, "nlt", jn3, &rx) {
        Chapter::Failed(m) => {
            assert!(m.starts_with("NLT API: the daily limit of 500"), "{m}");
            assert!(m.ends_with(". A free API key from api.nlt.to raises it to 5,000"), "{m}");
        }
        _ => panic!("expected the daily limit"),
    }
    assert!(log.lock().unwrap().is_empty());
    for d in [dir, key_dir, spent_dir] {
        let _ = std::fs::remove_dir_all(d);
    }
}

const NET_BODY: &str = r#"[{"bookname":"John","chapter":"3","verse":"16","text":"<p class=\"bodytext\">For God so loved the world,<n id=\"1\" \/> that he gave his only begotten Son. "}]"#;

#[test]
fn net_fetches_without_a_key() {
    isolate_keychain();
    let (base, log) = serve(Arc::new(|path: &str| {
        if path.starts_with("/api/?passage=John+3&") {
            (200, NET_BODY.to_string())
        } else {
            (404, "[]".into())
        }
    }));
    let dir = temp_dir("net");
    let (notify, rx) = notifier();
    let mut lib = Library::open(&dir, notify);
    let cfg = OnlineConfig {
        id: "net".into(),
        abbrev: "NET".into(),
        name: "NET Bible".into(),
        provider: ProviderKind::Net,
        copyright: String::new(),
    };
    lib.add_online_with_source(cfg, Arc::new(Net::with_base(&format!("{base}/api/"))));
    let jn3 = ChapterIdx::new(BookId(42), 3).unwrap();
    let Chapter::Ready(t) = wait_ready(&lib, "net", jn3, &rx) else { panic!("not ready") };
    assert_eq!(
        t.find(VerseRef::new(BookId(42), 3, 16)).unwrap().text,
        "For God so loved the world, that he gave his only begotten Son."
    );
    let online = lib.get("net").unwrap().online().unwrap().clone();
    assert!(online.notice().contains("Biblical Studies Press"));
    assert!(!online.needs_key());
    assert!(log.lock().unwrap()[0].path.contains("type=json"));
    let _ = std::fs::remove_dir_all(&dir);
}
