// SPDX-License-Identifier: AGPL-3.0-only
//! WebDAV upload client (MIG-040B, ADR-0010 decision 4 and the addendum of 2026-10-25).
//!
//! Operations: [`WebDavClient::capabilities`] (`OPTIONS`, must declare WebDAV class 1),
//! [`WebDavClient::exists`] (`PROPFIND` with `Depth: 0`, decided by the status code alone),
//! [`WebDavClient::make_collection`] (`MKCOL`), [`WebDavClient::put`] (`PUT` of a whole file with a known length) and
//! [`WebDavClient::delete`] (`DELETE`). There is no listing, no XML parsing, no locking and no resume. The client is
//! not wired into the sync commands; that belongs to the build and upload task (MIG-042).
//!
//! Transport: the blocking `reqwest` client with rustls and the platform certificate verifier. `ureq`, which the
//! download path uses, cannot send `MKCOL` or `PROPFIND` over HTTP/1.1 (see the ADR addendum).
//!
//! Safety rules (ADR-0010, decision 4):
//! * Basic credentials are sent only to `https://` base addresses. Combining them with `http://` is refused before
//!   any connection is made. Anonymous access over `http://` stays possible without credentials.
//! * Credentials go only to the origin (scheme, host, port) of the base address. This is checked for every request,
//!   including every redirect. A redirect to another origin is refused without connecting to the target.
//! * Redirects are never followed by the HTTP client itself (`Policy::none`). Write methods (`PUT`, `MKCOL`,
//!   `DELETE`) never follow redirects. `OPTIONS` and `PROPFIND` follow same-origin redirects only.
//! * Certificate and host-name verification is never disabled. Tests inject their own trust root through
//!   `HttpConfig::trusted_roots_der`, which replaces the platform roots for that client only.
//! * No secret appears in an error message or in `Debug` output.
//! * `DELETE` removes a folder together with its content. Callers must pass only paths they manage.

use std::error::Error as StdError;
use std::fmt;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

use armasync_core::{CancelToken, Error, ErrorKind, OpenedSource, RelativePath, RemoteTarget};
use reqwest::blocking::{Body, Client, Response};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, LOCATION};
use reqwest::{Certificate, Method, redirect::Policy};

use crate::http::{
    HttpConfig, MAX_REDIRECTS, encode_segment, network, origin_and_path, redirect_target,
};

/// Body of the minimal `PROPFIND` used for existence checks. The answer's body is not read.
const PROPFIND_BODY: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?><propfind xmlns="DAV:"><prop><resourcetype/></prop></propfind>"#;

/// A user name and password for HTTP Basic authentication. The password is never printed.
#[derive(Clone, PartialEq, Eq)]
pub struct BasicCredentials {
    user: String,
    password: String,
}

impl BasicCredentials {
    /// Credentials for one account.
    ///
    /// # Errors
    /// [`ErrorKind::Config`] for an empty user name or password, or a user name with a colon or a control character
    /// (RFC 7617 does not allow them).
    pub fn new(user: impl Into<String>, password: impl Into<String>) -> Result<Self, Error> {
        let (user, password) = (user.into(), password.into());
        if user.is_empty()
            || password.is_empty()
            || user.contains(':')
            || user.chars().any(char::is_control)
        {
            return Err(Error::new(
                ErrorKind::Config,
                "the user name or the password is missing or not allowed",
            ));
        }
        Ok(Self { user, password })
    }

    fn authorization(&self) -> String {
        let pair = format!("{}:{}", self.user, self.password);
        format!("Basic {}", base64_encode(pair.as_bytes()))
    }
}

impl fmt::Debug for BasicCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BasicCredentials")
            .field("password", &"<redacted>")
            .finish_non_exhaustive()
    }
}

/// What the server declared about itself on `OPTIONS`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebDavCapabilities {
    /// Methods listed in `Allow`, upper case, in server order.
    pub allow: Vec<String>,
    /// Whether the server declares WebDAV class 2 (locking) in `DAV`.
    pub locking: bool,
}

/// A WebDAV client for the folder tree below `base`.
#[derive(Clone)]
pub struct WebDavClient {
    base: String,
    origin: String,
    client: Client,
    config: HttpConfig,
    credentials: Option<BasicCredentials>,
}

impl fmt::Debug for WebDavClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebDavClient")
            .field("base", &self.base)
            .field("credentials", &self.credentials)
            .finish_non_exhaustive()
    }
}

impl WebDavClient {
    /// A client for `base` (an `http://` or `https://` address without credentials, query or fragment).
    ///
    /// # Errors
    /// [`ErrorKind::Unsupported`] for other schemes; [`ErrorKind::Config`] when credentials are combined with
    /// `http://` (no connection is made in that case) or the trust configuration is unusable.
    pub fn new(
        base: &str,
        config: HttpConfig,
        credentials: Option<BasicCredentials>,
    ) -> Result<Self, Error> {
        if !(base.starts_with("http://") || base.starts_with("https://")) {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "this transport only handles http and https addresses",
            ));
        }
        if credentials.is_some() && !base.starts_with("https://") {
            return Err(Error::new(
                ErrorKind::Config,
                "credentials are only sent over HTTPS; use an https:// address",
            ));
        }
        let origin = origin_and_path(base)
            .map(|(origin, _)| origin.to_owned())
            .ok_or_else(|| Error::new(ErrorKind::Config, "the address cannot be used"))?;
        let client = build_client(&config)?;
        Ok(Self {
            base: base.trim_end_matches('/').to_owned(),
            origin,
            client,
            config,
            credentials,
        })
    }

    /// The address of a path below the base.
    fn url_for(&self, path: &RelativePath) -> String {
        let encoded: Vec<String> = path.as_str().split('/').map(encode_segment).collect();
        format!("{}/{}", self.base, encoded.join("/"))
    }

    /// The `Authorization` value for `url`, or `None` when the request must not carry credentials.
    fn authorization_for(&self, url: &str) -> Option<String> {
        let credentials = self.credentials.as_ref()?;
        let same_origin = origin_and_path(url)
            .is_some_and(|(origin, _)| origin.eq_ignore_ascii_case(&self.origin));
        same_origin.then(|| credentials.authorization())
    }

    /// The total time allowed for a body of `size` bytes: at least the configured minimum, otherwise the size divided
    /// by the slowest acceptable rate.
    fn body_budget(&self, size: u64) -> Duration {
        let rate = self.config.min_bytes_per_second.max(1);
        Duration::from_secs(size / rate).max(self.config.min_body_timeout)
    }

    /// Sends a request with a fixed body. Redirects are followed only when `follow` is set, and then only to the
    /// same origin; every hop is checked for credentials on its own.
    fn send_bodyless(
        &self,
        method: Method,
        url: &str,
        headers: &[(&str, &str)],
        body: &'static [u8],
        follow: bool,
    ) -> Result<Response, Error> {
        let mut current = url.to_owned();
        for _ in 0..=MAX_REDIRECTS {
            let mut request = self
                .client
                .request(method.clone(), current.as_str())
                .timeout(self.config.response_timeout)
                .body(body);
            for (name, value) in headers.iter().copied() {
                request = request.header(name, value);
            }
            if let Some(value) = self.authorization_for(&current) {
                request = request.header(AUTHORIZATION, value);
            }
            let response = request.send().map_err(|e| map_reqwest(&e))?;
            let status = response.status().as_u16();
            if !matches!(status, 301 | 302 | 303 | 307 | 308) {
                return Ok(response);
            }
            if !follow {
                return Err(redirected_write());
            }
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| network("a redirect without a target"))?;
            current = redirect_target(&current, location)?;
        }
        Err(network("too many redirects"))
    }

    /// The WebDAV capabilities of the server. The server must declare WebDAV class 1.
    ///
    /// # Errors
    /// [`ErrorKind::Network`] for transport and authentication problems; [`ErrorKind::Unsupported`] when the server
    /// declares no WebDAV class 1.
    pub fn capabilities(&self) -> Result<WebDavCapabilities, Error> {
        let response = self.send_bodyless(Method::OPTIONS, &self.base, &[], b"", true)?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(status_error(status));
        }
        let header = |name: &str| {
            response
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_owned()
        };
        let dav: Vec<String> = header("dav")
            .split(',')
            .map(|t| t.trim().to_owned())
            .collect();
        if !dav.iter().any(|t| t == "1") {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "the server does not declare WebDAV class 1",
            ));
        }
        let allow = header("allow")
            .split(',')
            .map(|m| m.trim().to_ascii_uppercase())
            .filter(|m| !m.is_empty())
            .collect();
        Ok(WebDavCapabilities {
            allow,
            locking: dav.iter().any(|t| t == "2"),
        })
    }

    /// Whether something exists at `path`. Decided by the status code only: `207` or `200` means yes, `404` means no.
    ///
    /// # Errors
    /// As [`WebDavClient::capabilities`].
    pub fn exists(&self, path: &RelativePath) -> Result<bool, Error> {
        let url = self.url_for(path);
        let response = self.send_bodyless(
            custom_method(b"PROPFIND")?,
            &url,
            &[("Depth", "0"), ("Content-Type", "application/xml")],
            PROPFIND_BODY,
            true,
        )?;
        match response.status().as_u16() {
            200 | 207 => Ok(true),
            404 => Ok(false),
            other => Err(status_error(other)),
        }
    }

    /// Creates one folder. An existing folder at `path` is accepted.
    ///
    /// # Errors
    /// [`ErrorKind::FileSystem`] when the parent folder is missing or the server refuses the folder; otherwise as
    /// [`WebDavClient::capabilities`].
    pub fn make_collection(&self, path: &RelativePath) -> Result<(), Error> {
        let url = self.url_for(path);
        let method = custom_method(b"MKCOL")?;
        let response = self.send_bodyless(method, &url, &[], b"", false)?;
        let status = response.status().as_u16();
        if status == 201 {
            return Ok(());
        }
        if status == 405 && self.exists(path)? {
            return Ok(());
        }
        Err(status_error(status))
    }

    /// Uploads the whole content of `file` to `path`, replacing an existing file. The upload always starts at offset
    /// 0 and its length is taken from the file, so the file must not change during the upload. The parent folder must
    /// exist on the server.
    ///
    /// # Errors
    /// [`ErrorKind::FileSystem`] for a missing parent folder, a locked target, insufficient space, a file the server
    /// refuses as too large and an unreadable local file; [`ErrorKind::Network`] otherwise.
    pub fn put(&self, path: &RelativePath, file: &mut File) -> Result<(), Error> {
        file.seek(SeekFrom::Start(0)).map_err(local_read_error)?;
        let length = file.metadata().map_err(local_read_error)?.len();
        let copy = file.try_clone().map_err(local_read_error)?;
        self.send_upload(path, Box::new(copy), length, &CancelToken::new())
    }

    /// Streams `content` (exactly `length` bytes) to `path`. The stream stops at the next read once `cancel` is set,
    /// and a failure after cancellation is reported as [`ErrorKind::Cancelled`].
    fn send_upload(
        &self,
        path: &RelativePath,
        content: Box<dyn Read + Send>,
        length: u64,
        cancel: &CancelToken,
    ) -> Result<(), Error> {
        let url = self.url_for(path);
        let stream = CancelReader {
            inner: content,
            cancel: cancel.clone(),
        };
        let mut request = self
            .client
            .put(url.as_str())
            .timeout(self.body_budget(length))
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(Body::sized(stream, length));
        if let Some(value) = self.authorization_for(&url) {
            request = request.header(AUTHORIZATION, value);
        }
        let response = match request.send() {
            Ok(response) => response,
            Err(_) if cancel.is_cancelled() => {
                return Err(Error::new(ErrorKind::Cancelled, "the upload was cancelled"));
            }
            Err(error) => return Err(map_reqwest(&error)),
        };
        match response.status().as_u16() {
            200 | 201 | 204 => Ok(()),
            other => Err(status_error(other)),
        }
    }

    /// Removes the resource at `path`. Returns `false` when nothing was there.
    ///
    /// # Errors
    /// As [`WebDavClient::put`], without the space and size cases.
    pub fn delete(&self, path: &RelativePath) -> Result<bool, Error> {
        let url = self.url_for(path);
        let response = self.send_bodyless(Method::DELETE, &url, &[], b"", false)?;
        match response.status().as_u16() {
            200 | 202 | 204 => Ok(true),
            404 => Ok(false),
            other => Err(status_error(other)),
        }
    }
}

/// The upload side of the folder tree (ADR-0011). The methods keep their WebDAV semantics and their safety rules.
impl RemoteTarget for WebDavClient {
    fn exists(&self, path: &RelativePath) -> Result<bool, Error> {
        WebDavClient::exists(self, path)
    }

    fn make_folder(&self, path: &RelativePath) -> Result<(), Error> {
        self.make_collection(path)
    }

    fn upload(
        &self,
        path: &RelativePath,
        source: OpenedSource,
        cancel: &CancelToken,
    ) -> Result<(), Error> {
        self.send_upload(path, source.content, source.size, cancel)
    }

    fn fetch(&self, path: &RelativePath, limit: u64) -> Result<Option<Vec<u8>>, Error> {
        let url = self.url_for(path);
        let response = self.send_bodyless(Method::GET, &url, &[], b"", true)?;
        match response.status().as_u16() {
            200 => {}
            404 => return Ok(None),
            other => return Err(status_error(other)),
        }
        let mut data = Vec::new();
        response
            .take(limit + 1)
            .read_to_end(&mut data)
            .map_err(|_| network("the connection was lost while reading a file"))?;
        if data.len() as u64 > limit {
            return Err(Error::new(
                ErrorKind::Format,
                "a file on the server is larger than accepted",
            ));
        }
        Ok(Some(data))
    }
}

/// Ends a streamed upload at the next read once cancellation was requested.
struct CancelReader {
    inner: Box<dyn Read + Send>,
    cancel: CancelToken,
}

impl Read for CancelReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.cancel.is_cancelled() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "the upload was cancelled",
            ));
        }
        self.inner.read(buf)
    }
}

/// The blocking client: redirects are never followed by the client itself; the trust configuration is the platform
/// store, unless test roots were given (then only those roots are trusted).
fn build_client(config: &HttpConfig) -> Result<Client, Error> {
    let mut builder = Client::builder()
        .redirect(Policy::none())
        .connect_timeout(config.connect_timeout)
        .user_agent("ArmaSync");
    if let Some(list) = &config.trusted_roots_der {
        if list.is_empty() {
            return Err(Error::new(
                ErrorKind::Config,
                "no trusted root certificates given",
            ));
        }
        let certs = list
            .iter()
            .map(|der| Certificate::from_der(der))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| {
                Error::new(ErrorKind::Config, "a trusted root certificate is not valid")
            })?;
        builder = builder.tls_certs_only(certs);
    }
    builder
        .build()
        .map_err(|_| Error::new(ErrorKind::Config, "the HTTP client cannot be created"))
}

/// Maps a transport error to a fixed message. The message never contains the URL or credentials.
fn map_reqwest(error: &reqwest::Error) -> Error {
    let mut chain = String::new();
    let mut source = error.source();
    while let Some(cause) = source {
        chain.push_str(&cause.to_string());
        chain.push(' ');
        source = cause.source();
    }
    let lower = chain.to_ascii_lowercase();
    if lower.contains("certificate") || lower.contains("tls") || lower.contains("handshake") {
        network("secure connection failed")
    } else if error.is_timeout() {
        network("the server did not respond in time")
    } else if error.is_connect() {
        network("the connection to the server failed")
    } else {
        network("network error")
    }
}

fn local_read_error(error: std::io::Error) -> Error {
    Error::new(
        ErrorKind::FileSystem,
        format!("the file cannot be read: {error}"),
    )
}

/// A WebDAV method token such as `PROPFIND` or `MKCOL`.
fn custom_method(name: &[u8]) -> Result<Method, Error> {
    Method::from_bytes(name)
        .map_err(|_| Error::new(ErrorKind::Config, "the HTTP method is not valid"))
}

fn redirected_write() -> Error {
    network("the server redirected a write request; the redirect was not followed")
}

/// The error for a status the operation does not accept. Messages never contain credentials.
fn status_error(status: u16) -> Error {
    match status {
        300..=399 => redirected_write(),
        401 | 403 => network(format!(
            "the server refused access (status {status}); check the user name and password"
        )),
        409 => Error::new(
            ErrorKind::FileSystem,
            "the server refused the change: the parent folder is missing",
        ),
        413 => Error::new(
            ErrorKind::FileSystem,
            "the server refused the file as too large",
        ),
        423 => Error::new(
            ErrorKind::FileSystem,
            "the file or folder is locked on the server",
        ),
        507 => Error::new(
            ErrorKind::FileSystem,
            "the server has not enough space for this file",
        ),
        other => network(format!("the server answered with status {other}")),
    }
}

/// Standard Base64 (RFC 4648) with padding, for the `Authorization` header only.
fn base64_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let byte = |i: usize| u32::from(chunk.get(i).copied().unwrap_or(0));
        let value = (byte(0) << 16) | (byte(1) << 8) | byte(2);
        for index in 0..4 {
            if index <= chunk.len() {
                let sextet = (value >> (18 - 6 * index)) & 63;
                out.push(char::from(ALPHABET[sextet as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}
