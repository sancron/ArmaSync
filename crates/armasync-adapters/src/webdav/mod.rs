// SPDX-License-Identifier: AGPL-3.0-only
//! WebDAV upload client (MIG-040B, ADR-0010, feature F07), in its first step.
//!
//! Implemented: [`WebDavClient::capabilities`] (`OPTIONS`, must declare WebDAV class 1), [`WebDavClient::put`]
//! (`PUT` of a file with a known length, into an existing folder) and [`WebDavClient::delete`] (`DELETE`).
//!
//! **Blocked, not implemented:** folder creation (`MKCOL`) and existence checks (`PROPFIND`). The HTTP stack `ureq`
//! 3.4.2 (`ureq-proto` 0.6.5) rejects every method outside its list for HTTP/1.1 before anything is sent, and WebDAV
//! uses `MKCOL` and `PROPFIND` over HTTP/1.1. Solving this needs a decision on the transport (see ADR-0010,
//! addendum of 2026-10-25). The client is not wired into the sync commands; that belongs to MIG-042.
//! No listing, no XML parsing, no locking and no resume.
//!
//! Safety rules (ADR-0010, decision 4):
//! * Basic credentials are sent only to `https://` base addresses. Combining them with `http://` is refused before
//!   any connection is made. Anonymous access over `http://` stays possible without credentials.
//! * Credentials go only to the origin (scheme, host, port) of the base address. This is checked for every request,
//!   including every redirect. A redirect to another origin is refused without connecting to the target.
//! * Write methods (`PUT`, `DELETE`) never follow redirects, because their bodies could not be sent again.
//!   `OPTIONS` follows same-origin redirects only.
//! * TLS verification and timeouts are those of [`HttpRepository`]; verification is never disabled.
//! * No secret appears in an error message or in `Debug` output.
//! * `DELETE` removes a folder together with its content. Callers must pass only paths they manage.

use std::fmt;
use std::fs::File;
use std::io::{Seek, SeekFrom};

use armasync_core::{Error, ErrorKind, RelativePath};
use ureq::Body;
use ureq::http::{self, Response};

use crate::http::{
    HttpConfig, HttpRepository, MAX_REDIRECTS, encode_segment, map_ureq, network, origin_and_path,
    redirect_target,
};

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
#[derive(Debug, Clone)]
pub struct WebDavClient {
    http: HttpRepository,
    credentials: Option<BasicCredentials>,
    origin: String,
}

impl WebDavClient {
    /// A client for `base` (an `http://` or `https://` address without credentials, query or fragment).
    ///
    /// # Errors
    /// [`ErrorKind::Unsupported`] for other schemes; [`ErrorKind::Config`] when credentials are combined with
    /// `http://` (no connection is made in that case).
    pub fn new(
        base: &str,
        config: HttpConfig,
        credentials: Option<BasicCredentials>,
    ) -> Result<Self, Error> {
        let http = HttpRepository::new(base, config)?;
        if credentials.is_some() && !http.base().to_ascii_lowercase().starts_with("https://") {
            return Err(Error::new(
                ErrorKind::Config,
                "credentials are only sent over HTTPS; use an https:// address",
            ));
        }
        let origin = origin_and_path(http.base())
            .map(|(origin, _)| origin.to_owned())
            .ok_or_else(|| Error::new(ErrorKind::Config, "the address cannot be used"))?;
        Ok(Self {
            http,
            credentials,
            origin,
        })
    }

    /// The address of a path below the base.
    fn url_for(&self, path: &RelativePath) -> String {
        let encoded: Vec<String> = path.as_str().split('/').map(encode_segment).collect();
        format!("{}/{}", self.http.base(), encoded.join("/"))
    }

    /// The `Authorization` value for `url`, or `None` when the request must not carry credentials.
    fn authorization_for(&self, url: &str) -> Option<String> {
        let credentials = self.credentials.as_ref()?;
        let same_origin = origin_and_path(url)
            .is_some_and(|(origin, _)| origin.eq_ignore_ascii_case(&self.origin));
        same_origin.then(|| credentials.authorization())
    }

    /// Sends a request with a fixed body. Redirects are followed only when `follow` is set, and then only to the
    /// same origin; every hop is checked for credentials on its own.
    fn send_bodyless(
        &self,
        method: &str,
        url: &str,
        body: &'static [u8],
        follow: bool,
    ) -> Result<Response<Body>, Error> {
        let mut current = url.to_owned();
        for _ in 0..=MAX_REDIRECTS {
            let mut builder = http::Request::builder()
                .method(method)
                .uri(current.as_str());
            if let Some(value) = self.authorization_for(&current) {
                builder = builder.header("Authorization", value);
            }
            let request = builder
                .body(body)
                .map_err(|_| Error::new(ErrorKind::Config, "the address cannot be used"))?;
            let response = self.http.agent().run(request).map_err(|e| map_ureq(&e))?;
            let status = response.status().as_u16();
            if !matches!(status, 301 | 302 | 303 | 307 | 308) {
                return Ok(response);
            }
            if !follow {
                return Err(redirected_write());
            }
            let location = response
                .headers()
                .get("location")
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
        let url = self.http.base().to_owned();
        let response = self.send_bodyless("OPTIONS", &url, b"", true)?;
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

    /// Uploads the whole content of `file` to `path`, replacing an existing file. The upload always starts at offset
    /// 0 and its length is taken from the file, so the file must not change during the upload. The parent folder must
    /// already exist on the server.
    ///
    /// # Errors
    /// [`ErrorKind::FileSystem`] for a missing parent folder, a locked target, insufficient space, a file the server
    /// refuses as too large and an unreadable local file; [`ErrorKind::Network`] otherwise.
    pub fn put(&self, path: &RelativePath, file: &mut File) -> Result<(), Error> {
        file.seek(SeekFrom::Start(0)).map_err(|e| {
            Error::new(
                ErrorKind::FileSystem,
                format!("the file cannot be read: {e}"),
            )
        })?;
        let length = file
            .metadata()
            .map_err(|e| {
                Error::new(
                    ErrorKind::FileSystem,
                    format!("the file cannot be read: {e}"),
                )
            })?
            .len();
        let url = self.url_for(path);
        let mut builder = self
            .http
            .agent()
            .put(url.as_str())
            .header("Content-Type", "application/octet-stream");
        if let Some(value) = self.authorization_for(&url) {
            builder = builder.header("Authorization", value);
        }
        let budget = self.http.body_budget(length);
        let response = builder
            .config()
            .timeout_send_body(Some(budget))
            .build()
            .send(&*file)
            .map_err(|e| map_ureq(&e))?;
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
        let response = self.send_bodyless("DELETE", &url, b"", false)?;
        match response.status().as_u16() {
            200 | 202 | 204 => Ok(true),
            404 => Ok(false),
            other => Err(status_error(other)),
        }
    }
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
