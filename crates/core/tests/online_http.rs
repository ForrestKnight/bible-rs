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
use bible_core::online::{OnlineConfig, ProviderKind};

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
    let _ = std::fs::remove_dir_all(&dir);
}
