// SPDX-License-Identifier: AGPL-3.0-only
//! Tests of the WebDAV client (OPTIONS, PROPFIND, MKCOL, PUT, DELETE) against local loopback servers: plain HTTP and
//! TLS with a freshly generated, self-signed test certificate. Synthetic credentials only; no real server and no real
//! data.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use armasync_adapters::http::HttpConfig;
use armasync_adapters::webdav::{BasicCredentials, WebDavClient};
use armasync_core::{ErrorKind, RelativePath};

const USER: &str = "Aladdin";
const PASSWORD: &str = "open sesame";
/// `Basic` plus base64("Aladdin:open sesame"), the example of RFC 7617.
const EXPECTED_AUTH: &str = "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==";

#[derive(Default)]
struct State {
    files: BTreeMap<String, Vec<u8>>,
    dirs: BTreeSet<String>,
    seen: Vec<Seen>,
}

#[derive(Clone, Debug)]
struct Seen {
    method: String,
    target: String,
    authorization: Option<String>,
}

#[derive(Clone)]
struct Options {
    require_auth: bool,
    /// Absolute target of the `/dav/redirect/cross` route.
    cross_target: String,
}

struct Server {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    state: Arc<Mutex<State>>,
    cert_der: Vec<u8>,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
    }
}

impl Server {
    fn seen(&self) -> Vec<Seen> {
        self.state.lock().unwrap().seen.clone()
    }

    fn file(&self, name: &str) -> Option<Vec<u8>> {
        self.state.lock().unwrap().files.get(name).cloned()
    }

    fn folder(&self, name: &str) {
        self.state.lock().unwrap().dirs.insert(name.to_owned());
    }

    fn https(&self) -> String {
        format!("https://{}/dav", self.addr)
    }

    fn http(&self) -> String {
        format!("http://{}/dav", self.addr)
    }

    fn trust(&self) -> HttpConfig {
        HttpConfig {
            trusted_roots_der: Some(vec![self.cert_der.clone()]),
            ..HttpConfig::default()
        }
    }
}

fn tls_config() -> (Arc<rustls::ServerConfig>, Vec<u8>) {
    use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
    let certified = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_owned()]).unwrap();
    let cert_der = certified.cert.der().to_vec();
    let key = PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(
        vec![CertificateDer::from(cert_der.clone())],
        PrivateKeyDer::from(key),
    )
    .unwrap();
    (Arc::new(config), cert_der)
}

/// Starts a WebDAV-like server on loopback. `require_auth` demands [`EXPECTED_AUTH`] on every request.
fn start(tls: bool, require_auth: bool, cross_target: &str) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let (config, cert_der) = if tls {
        let (config, cert) = tls_config();
        (Some(config), cert)
    } else {
        (None, Vec::new())
    };
    let stop = Arc::new(AtomicBool::new(false));
    let state = Arc::new(Mutex::new(State::default()));
    let options = Options {
        require_auth,
        cross_target: cross_target.to_owned(),
    };
    let (stop2, state2) = (stop.clone(), state.clone());
    thread::spawn(move || {
        for incoming in listener.incoming() {
            if stop2.load(Ordering::SeqCst) {
                break;
            }
            let Ok(mut stream) = incoming else { continue };
            let (config, state, options) = (config.clone(), state2.clone(), options.clone());
            thread::spawn(move || match config {
                None => handle(&mut stream, &state, &options),
                Some(config) => {
                    let Ok(connection) = rustls::ServerConnection::new(config) else {
                        return;
                    };
                    let mut tls = rustls::StreamOwned::new(connection, stream);
                    handle(&mut tls, &state, &options);
                    tls.conn.send_close_notify();
                    let _ = tls.flush();
                }
            });
        }
    });
    Server {
        addr,
        stop,
        state,
        cert_der,
    }
}

struct Request {
    method: String,
    target: String,
    authorization: Option<String>,
    body: Vec<u8>,
}

fn read_request<S: Read + Write>(stream: &mut S) -> Option<Request> {
    let mut data = Vec::new();
    let mut buf = [0u8; 4096];
    let head_end = loop {
        if let Some(pos) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos + 4;
        }
        let n = stream.read(&mut buf).ok()?;
        if n == 0 {
            return None;
        }
        data.extend_from_slice(&buf[..n]);
    };
    let head = String::from_utf8_lossy(&data[..head_end]).into_owned();
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let method = first.next()?.to_owned();
    let target = first.next()?.to_owned();
    let headers: Vec<(String, String)> = lines
        .take_while(|l| !l.is_empty())
        .filter_map(|l| l.split_once(": "))
        .map(|(k, v)| (k.to_ascii_lowercase(), v.to_owned()))
        .collect();
    let header = |name: &str| {
        headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
    };
    let length: usize = header("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    if header("expect").is_some_and(|v| v.eq_ignore_ascii_case("100-continue")) {
        stream.write_all(b"HTTP/1.1 100 Continue\r\n\r\n").ok()?;
    }
    let mut body = data[head_end..].to_vec();
    while body.len() < length {
        let n = stream.read(&mut buf).ok()?;
        if n == 0 {
            return None;
        }
        body.extend_from_slice(&buf[..n]);
    }
    body.truncate(length);
    Some(Request {
        method,
        target,
        authorization: header("authorization"),
        body,
    })
}

fn respond(stream: &mut impl Write, status: &str, headers: &[(&str, String)], body: &[u8]) {
    let mut head = format!(
        "HTTP/1.1 {status}\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn handle<S: Read + Write>(stream: &mut S, state: &Mutex<State>, options: &Options) {
    let Some(request) = read_request(stream) else {
        return;
    };
    state.lock().unwrap().seen.push(Seen {
        method: request.method.clone(),
        target: request.target.clone(),
        authorization: request.authorization.clone(),
    });
    if options.require_auth && request.authorization.as_deref() != Some(EXPECTED_AUTH) {
        let challenge = vec![("WWW-Authenticate", "Basic realm=\"test\"".to_owned())];
        return respond(stream, "401 Unauthorized", &challenge, b"");
    }
    let target = request.target.as_str();
    if let Some(rest) = target.strip_prefix("/dav/redirect/same") {
        let location = vec![("Location", format!("/dav/{}", rest.trim_start_matches('/')))];
        return respond(stream, "302 Found", &location, b"");
    }
    if target.starts_with("/dav/redirect/cross") {
        let location = vec![("Location", options.cross_target.clone())];
        return respond(stream, "302 Found", &location, b"");
    }
    let Some(rel) = target.strip_prefix("/dav") else {
        return respond(stream, "404 Not Found", &[], b"");
    };
    let rel = rel.trim_start_matches('/');
    let mut st = state.lock().unwrap();
    let parent = rel.rsplit_once('/').map_or("", |(parent, _)| parent);
    let parent_missing = !parent.is_empty() && !st.dirs.contains(parent);
    let exists = rel.is_empty() || st.dirs.contains(rel) || st.files.contains_key(rel);
    match request.method.as_str() {
        "OPTIONS" => {
            let headers = vec![
                ("DAV", "1".to_owned()),
                (
                    "Allow",
                    "OPTIONS, PROPFIND, MKCOL, PUT, DELETE, GET".to_owned(),
                ),
            ];
            respond(stream, "200 OK", &headers, b"");
        }
        "PROPFIND" => {
            if exists {
                respond(stream, "207 Multi-Status", &[], b"");
            } else {
                respond(stream, "404 Not Found", &[], b"");
            }
        }
        "MKCOL" => {
            if exists {
                respond(stream, "405 Method Not Allowed", &[], b"");
            } else if parent_missing {
                respond(stream, "409 Conflict", &[], b"");
            } else {
                st.dirs.insert(rel.to_owned());
                respond(stream, "201 Created", &[], b"");
            }
        }
        "PUT" => {
            if rel.contains("full") {
                respond(stream, "507 Insufficient Storage", &[], b"");
            } else if rel.contains("locked") {
                respond(stream, "423 Locked", &[], b"");
            } else if parent_missing || st.dirs.contains(rel) {
                respond(stream, "409 Conflict", &[], b"");
            } else {
                let existed = st.files.insert(rel.to_owned(), request.body).is_some();
                let status = if existed {
                    "204 No Content"
                } else {
                    "201 Created"
                };
                respond(stream, status, &[], b"");
            }
        }
        "DELETE" => {
            if st.files.remove(rel).is_some() {
                respond(stream, "204 No Content", &[], b"");
            } else if st.dirs.remove(rel) {
                let prefix = format!("{rel}/");
                st.files.retain(|name, _| !name.starts_with(&prefix));
                st.dirs.retain(|name| !name.starts_with(&prefix));
                respond(stream, "204 No Content", &[], b"");
            } else {
                respond(stream, "404 Not Found", &[], b"");
            }
        }
        _ => respond(stream, "405 Method Not Allowed", &[], b""),
    }
}

fn path(text: &str) -> RelativePath {
    RelativePath::parse(text).unwrap()
}

fn credentials() -> BasicCredentials {
    BasicCredentials::new(USER, PASSWORD).unwrap()
}

fn scratch_file(name: &str, bytes: &[u8]) -> PathBuf {
    let file = std::env::temp_dir().join(format!("armasync-webdav-{}-{name}", std::process::id()));
    std::fs::write(&file, bytes).unwrap();
    file
}

fn payload() -> Vec<u8> {
    (0..200_000)
        .map(|i: usize| (i.wrapping_mul(31) % 251) as u8)
        .collect()
}

// ---- schemes and credentials ----------------------------------------------------------------------------------

#[test]
fn credentials_over_plain_http_are_refused_before_any_connection() {
    let server = start(false, false, "");
    let error =
        WebDavClient::new(&server.http(), HttpConfig::default(), Some(credentials())).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Config);
    assert!(server.seen().is_empty());
}

#[test]
fn other_schemes_are_unsupported() {
    let error = WebDavClient::new("ftp://127.0.0.1/dav", HttpConfig::default(), None).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Unsupported);
}

#[test]
fn credentials_must_be_complete_and_are_never_printed() {
    assert!(BasicCredentials::new("", "x").is_err());
    assert!(BasicCredentials::new("u", "").is_err());
    assert!(BasicCredentials::new("a:b", "x").is_err());
    assert!(BasicCredentials::new("a\nb", "x").is_err());
    let debug = format!(
        "{:?}",
        BasicCredentials::new("user", "topsecret-value").unwrap()
    );
    assert!(!debug.contains("topsecret-value"), "{debug}");
}

#[test]
fn the_client_debug_output_never_shows_the_password() {
    let client = WebDavClient::new(
        "https://127.0.0.1:9/dav",
        HttpConfig::default(),
        Some(BasicCredentials::new("user", "topsecret-value").unwrap()),
    )
    .unwrap();
    let debug = format!("{client:?}");
    assert!(!debug.contains("topsecret-value"), "{debug}");
}

#[test]
fn an_untrusted_certificate_is_rejected_with_the_default_trust_store() {
    let server = start(true, true, "");
    let client =
        WebDavClient::new(&server.https(), HttpConfig::default(), Some(credentials())).unwrap();
    let error = client.capabilities().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network);
    assert!(
        error.message().contains("secure connection failed"),
        "{error}"
    );
}

// ---- capabilities and anonymous access ------------------------------------------------------------------------

#[test]
fn options_declare_class_1_anonymously_over_plain_http() {
    let server = start(false, false, "");
    let client = WebDavClient::new(&server.http(), HttpConfig::default(), None).unwrap();
    let capabilities = client.capabilities().unwrap();
    assert!(!capabilities.locking);
    assert!(capabilities.allow.iter().any(|m| m == "PUT"));
    assert!(server.seen().iter().all(|s| s.authorization.is_none()));
}

#[test]
fn anonymous_folder_creation_and_upload_over_plain_http_send_no_authorization() {
    let server = start(false, false, "");
    let client = WebDavClient::new(&server.http(), HttpConfig::default(), None).unwrap();
    client.make_collection(&path("docs")).unwrap();
    let mut file = File::open(scratch_file("anon.txt", b"hello")).unwrap();
    client.put(&path("docs/hello.txt"), &mut file).unwrap();
    assert_eq!(server.file("docs/hello.txt").unwrap(), b"hello");
    let seen = server.seen();
    assert!(seen.iter().any(|s| s.method == "MKCOL"), "{seen:?}");
    assert!(seen.iter().all(|s| s.authorization.is_none()));
}

// ---- authenticated round trip over TLS ------------------------------------------------------------------------

#[test]
fn basic_auth_round_trip_over_https_carries_the_header_on_every_request() {
    let server = start(true, true, "");
    let client = WebDavClient::new(&server.https(), server.trust(), Some(credentials())).unwrap();
    client.capabilities().unwrap();
    client.make_collection(&path("docs")).unwrap();
    assert!(client.exists(&path("docs")).unwrap());
    let content = payload();
    let mut file = File::open(scratch_file("payload.bin", &content)).unwrap();
    client.put(&path("docs/payload.bin"), &mut file).unwrap();
    assert_eq!(server.file("docs/payload.bin").unwrap(), content);
    assert!(client.exists(&path("docs/payload.bin")).unwrap());
    assert!(client.delete(&path("docs/payload.bin")).unwrap());
    assert!(!client.delete(&path("docs/payload.bin")).unwrap());
    assert!(!client.exists(&path("docs/payload.bin")).unwrap());
    let seen = server.seen();
    let methods: Vec<&str> = seen.iter().map(|s| s.method.as_str()).collect();
    assert_eq!(
        methods,
        [
            "OPTIONS", "MKCOL", "PROPFIND", "PUT", "PROPFIND", "DELETE", "DELETE", "PROPFIND"
        ],
        "{seen:?}"
    );
    assert!(
        seen.iter()
            .all(|s| s.authorization.as_deref() == Some(EXPECTED_AUTH)),
        "{seen:?}"
    );
}

#[test]
fn a_wrong_password_is_refused_without_echoing_it() {
    let server = start(true, true, "");
    let wrong = BasicCredentials::new(USER, "wrong-secret-value").unwrap();
    let client = WebDavClient::new(&server.https(), server.trust(), Some(wrong)).unwrap();
    let error = client.capabilities().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network);
    assert!(
        error.message().contains("check the user name and password"),
        "{error}"
    );
    assert!(!error.message().contains("wrong-secret-value"));
    assert!(!server.seen().is_empty());
}

// ---- folders ------------------------------------------------------------------------------------------------

#[test]
fn make_collection_creates_a_folder_and_accepts_an_existing_one() {
    let server = start(true, true, "");
    let client = WebDavClient::new(&server.https(), server.trust(), Some(credentials())).unwrap();
    client.make_collection(&path("docs")).unwrap();
    client.make_collection(&path("docs")).unwrap();
    assert!(client.exists(&path("docs")).unwrap());
    assert!(!client.exists(&path("other")).unwrap());
}

#[test]
fn make_collection_without_a_parent_folder_is_a_file_system_error() {
    let server = start(true, true, "");
    let client = WebDavClient::new(&server.https(), server.trust(), Some(credentials())).unwrap();
    let error = client.make_collection(&path("missing/sub")).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::FileSystem, "{error}");
}

// ---- redirects ------------------------------------------------------------------------------------------------

#[test]
fn write_requests_do_not_follow_redirects() {
    let server = start(true, true, "");
    server.folder("docs");
    let client = WebDavClient::new(&server.https(), server.trust(), Some(credentials())).unwrap();
    let mut file = File::open(scratch_file("redirected.bin", b"data")).unwrap();
    let error = client
        .put(&path("redirect/same/docs/x.bin"), &mut file)
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network);
    assert!(error.message().contains("not followed"), "{error}");
    assert!(server.file("docs/x.bin").is_none());
    let seen = server.seen();
    assert!(
        seen.iter().all(|s| s.target != "/dav/docs/x.bin"),
        "{seen:?}"
    );
}

#[test]
fn a_redirected_folder_creation_is_not_followed() {
    let server = start(true, true, "");
    let client = WebDavClient::new(&server.https(), server.trust(), Some(credentials())).unwrap();
    let error = client
        .make_collection(&path("redirect/same/newdir"))
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network);
    assert!(error.message().contains("not followed"), "{error}");
    assert!(server.seen().iter().all(|s| s.target != "/dav/newdir"));
}

#[test]
fn same_origin_redirect_keeps_the_credentials_for_read_requests() {
    let server = start(true, true, "");
    let base = format!("https://{}/dav/redirect/same", server.addr);
    let client = WebDavClient::new(&base, server.trust(), Some(credentials())).unwrap();
    client.capabilities().unwrap();
    let seen = server.seen();
    let followed = seen
        .iter()
        .find(|s| s.target == "/dav/" && s.method == "OPTIONS")
        .expect("the redirect target was requested");
    assert_eq!(followed.authorization.as_deref(), Some(EXPECTED_AUTH));
}

#[test]
fn a_cross_origin_redirect_never_receives_credentials() {
    let other = start(true, false, "");
    let target = format!("https://{}/dav/docs", other.addr);
    let server = start(true, true, &target);
    let base = format!("https://{}/dav/redirect/cross", server.addr);
    let client = WebDavClient::new(&base, server.trust(), Some(credentials())).unwrap();
    let error = client.exists(&path("docs")).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network);
    assert!(error.message().contains("refused"), "{error}");
    assert!(other.seen().is_empty(), "{:?}", other.seen());
    assert_eq!(server.seen().len(), 1);
}

#[test]
fn a_redirect_from_https_to_http_is_refused_and_the_plain_target_is_never_contacted() {
    let plain = start(false, false, "");
    let target = format!("http://{}/dav/docs", plain.addr);
    let server = start(true, true, &target);
    let base = format!("https://{}/dav/redirect/cross", server.addr);
    let client = WebDavClient::new(&base, server.trust(), Some(credentials())).unwrap();
    let error = client.capabilities().unwrap_err();
    assert_eq!(error.kind(), ErrorKind::Network);
    assert!(plain.seen().is_empty(), "{:?}", plain.seen());
}

// ---- failures -------------------------------------------------------------------------------------------------

#[test]
fn full_disk_locked_and_missing_parent_map_to_file_system_errors() {
    let server = start(true, true, "");
    server.folder("docs");
    let client = WebDavClient::new(&server.https(), server.trust(), Some(credentials())).unwrap();
    let mut file = File::open(scratch_file("failure.bin", b"x")).unwrap();
    for target in ["docs/full.bin", "docs/locked.bin", "missing/x.bin"] {
        let error = client.put(&path(target), &mut file).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::FileSystem, "{target}: {error}");
    }
}
