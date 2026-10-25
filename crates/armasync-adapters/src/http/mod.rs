// SPDX-License-Identifier: AGPL-3.0-only
//! HTTP(S) access to an existing repository (MIG-030B, ADR-0006/ADR-0007).
//!
//! [`HttpRepository`] implements the core's [`RepositorySource`]: the manifest comes from the published
//! `.a3s/sync` and `.a3s/serverinfo`, files are streamed in chunks (never held in memory) and checked against the
//! announced size. The transport is `ureq` with rustls.
//!
//! Safety rules enforced here:
//! * **TLS is always verified** against the operating system's trust store (or, only if explicitly injected, the
//!   given roots). There is no option to switch certificate or host-name verification off.
//! * Redirects are followed by hand, at most [`MAX_REDIRECTS`], and only to the same origin or an upgrade to
//!   `https`; a downgrade to `http`, another scheme or a relative location is refused.
//! * No credentials are sent (authentication is a later task); no secret is ever part of an error.
//! * Time is bounded: connect, response header, and a floor transfer rate for bodies; sizes are bounded: metadata
//!   by the parser limits, files by the announced size (extra data is refused), `read` by [`MAX_READ_BYTES`].
//! * Every failure maps to a core [`ErrorKind`]: network, authentication and trust problems are `Network`; a body
//!   that differs from the announcement is `Integrity`; cancellation is `Cancelled`.
//!
//! Resume (MIG-032R, ADR-0009): a request with a stored **strong** validator sends `Range: bytes=<offset>-` and
//! `If-Range`. Only a `206` whose `Content-Range` covers exactly the rest of the announced file **and** whose `ETag`
//! equals the stored validator continues the transfer; every other answer restarts at zero or is refused as
//! `Integrity` ("resume refused"). A `416` is a refused resume as well.
//!
//! Not done here (later tasks): authentication, explicit proxy settings (the proxy of the environment is honoured),
//! parallel transfers, compressed (`.zip`) files.

use std::io::Read;
use std::time::Duration;

use armasync_core::{
    Error, ErrorKind, Manifest, Opened, RelativePath, RepositorySource, is_strong_validator,
};
use ureq::Agent;
use ureq::tls::{Certificate, RootCerts, TlsConfig};

use crate::sync_meta::{SyncMetadata, read_sync_metadata};

/// Most redirects followed for one request.
pub const MAX_REDIRECTS: usize = 5;
/// Largest file [`RepositorySource::read`] returns (it holds the file in memory; large files use `fetch`).
pub const MAX_READ_BYTES: u64 = 16 * 1024 * 1024;
/// Largest accepted metadata file (compressed), equal to the parser limit.
const MAX_METADATA_BYTES: u64 = 32 * 1024 * 1024;

/// Settings of the HTTP client. The defaults are safe for the internet.
#[derive(Debug, Clone)]
pub struct HttpConfig {
    /// Maximum time to resolve, connect and complete the TLS handshake.
    pub connect_timeout: Duration,
    /// Maximum time to wait for the response header after the request was sent.
    pub response_timeout: Duration,
    /// Slowest acceptable body transfer in bytes per second; the body of a file must arrive within
    /// `max(min_body_timeout, size / this rate)`. A stalled connection therefore ends the transfer with a network
    /// error instead of hanging.
    pub min_bytes_per_second: u64,
    /// Shortest time allowed for receiving a body (see `min_bytes_per_second`).
    pub min_body_timeout: Duration,
    /// **Replaces** the system trust store with exactly these DER-encoded root certificates for this client. For
    /// tests and controlled environments only; `None` (default) uses the operating system's trust store.
    /// Verification itself is never disabled.
    pub trusted_roots_der: Option<Vec<Vec<u8>>>,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(15),
            response_timeout: Duration::from_secs(30),
            min_bytes_per_second: 32 * 1024,
            min_body_timeout: Duration::from_secs(60),
            trusted_roots_der: None,
        }
    }
}

/// A repository reached over HTTP or HTTPS.
#[derive(Debug, Clone)]
pub struct HttpRepository {
    base: String,
    agent: Agent,
    config: HttpConfig,
}

pub(crate) fn network(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::Network, message)
}

fn map_ureq(error: &ureq::Error) -> Error {
    use ureq::Error as E;
    match error {
        E::Timeout(_) => network("the server did not respond in time"),
        E::HostNotFound => network("the server name could not be resolved"),
        E::ConnectionFailed => network("the connection to the server failed"),
        E::Io(io) => {
            let text = io.to_string();
            let lower = text.to_ascii_lowercase();
            if lower.contains("certificate") || lower.contains("tls") || lower.contains("handshake")
            {
                network(format!("secure connection failed: {text}"))
            } else if io.kind() == std::io::ErrorKind::UnexpectedEof {
                network("the connection was closed early")
            } else {
                network(format!("network error: {text}"))
            }
        }
        other => network(format!("network error: {other}")),
    }
}

/// Percent-encodes one path segment: unreserved characters and `@` stay, everything else is encoded.
pub(crate) fn encode_segment(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'@') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

pub(crate) fn origin_and_path(url: &str) -> Option<(&str, &str)> {
    let rest_start = url.find("://")? + 3;
    let path_start = url[rest_start..]
        .find('/')
        .map_or(url.len(), |i| rest_start + i);
    Some((&url[..path_start], &url[path_start..]))
}

/// Decides whether a redirect from `from` to `location` is acceptable and returns the absolute target URL.
///
/// Accepted: an absolute `http`/`https` URL on the same origin or an upgrade of the same host from `http` to
/// `https`, and an absolute path on the same origin. Refused: downgrades from `https` to `http`, other schemes, relative references, and anything
/// that carries credentials.
pub fn redirect_target(from: &str, location: &str) -> Result<String, Error> {
    let refused = |why: &str| network(format!("a redirect was refused: {why}"));
    let (origin, _) = origin_and_path(from).ok_or_else(|| refused("unusable address"))?;
    let target = if location.starts_with('/') && !location.starts_with("//") {
        format!("{origin}{location}")
    } else if location.starts_with("http://") || location.starts_with("https://") {
        location.to_owned()
    } else {
        return Err(refused("unsupported target"));
    };
    let (target_origin, _) = origin_and_path(&target).ok_or_else(|| refused("unusable target"))?;
    let authority = target_origin.split("://").nth(1).unwrap_or_default();
    if authority.contains('@') || target.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(refused("the target is not acceptable"));
    }
    let same_origin = origin.eq_ignore_ascii_case(target_origin);
    let host_of = |authority: &str| {
        authority
            .rsplit_once(':')
            .map_or(authority, |(h, _)| h)
            .to_ascii_lowercase()
    };
    let upgrade = from.starts_with("http://")
        && target.starts_with("https://")
        && host_of(origin.split("://").nth(1).unwrap_or_default()) == host_of(authority);
    if !(same_origin || upgrade) {
        return Err(refused(
            if from.starts_with("https://") && target.starts_with("http://") {
                "downgrade to an unencrypted connection"
            } else {
                "another server"
            },
        ));
    }
    Ok(target)
}

impl HttpRepository {
    /// A repository at `base` (a `http://` or `https://` address without credentials, query or fragment, as
    /// validated by [`armasync_core::RepositoryUrl`]).
    ///
    /// # Errors
    /// [`ErrorKind::Unsupported`] for other schemes; [`ErrorKind::Config`] for an unusable trust configuration.
    pub fn new(base: &str, config: HttpConfig) -> Result<Self, Error> {
        if !(base.starts_with("http://") || base.starts_with("https://")) {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "this transport only handles http and https addresses",
            ));
        }
        let roots = match &config.trusted_roots_der {
            None => RootCerts::PlatformVerifier,
            Some(list) if list.is_empty() => {
                return Err(Error::new(
                    ErrorKind::Config,
                    "no trusted root certificates given",
                ));
            }
            Some(list) => {
                let certs: Vec<Certificate<'static>> = list
                    .iter()
                    .map(|der| Certificate::from_der(der).to_owned())
                    .collect();
                RootCerts::new_with_certs(&certs)
            }
        };
        let agent: Agent = Agent::config_builder()
            .tls_config(TlsConfig::builder().root_certs(roots).build())
            .timeout_connect(Some(config.connect_timeout))
            .timeout_recv_response(Some(config.response_timeout))
            .max_redirects(0)
            .max_redirects_will_error(false)
            .http_status_as_error(false)
            .accept_encoding("identity")
            .user_agent("ArmaSync")
            .build()
            .into();
        Ok(Self {
            base: base.trim_end_matches('/').to_owned(),
            agent,
            config,
        })
    }

    fn url_for(&self, relative: &str) -> String {
        let encoded: Vec<String> = relative.split('/').map(encode_segment).collect();
        format!("{}/{}", self.base, encoded.join("/"))
    }

    /// Performs a GET, following permitted redirects. Without `range` the answer must be 200 (the whole file). With
    /// `range = (offset, validator)` it asks `bytes=<offset>-` with `If-Range: <validator>`; the answer is then 200
    /// (whole file: the range was ignored or the file changed) or 206. Returns the status with the response.
    fn get(
        &self,
        url: &str,
        body_budget: Option<Duration>,
        range: Option<(u64, &str)>,
    ) -> Result<(u16, ureq::http::Response<ureq::Body>), Error> {
        let mut current = url.to_owned();
        for _ in 0..=MAX_REDIRECTS {
            let mut builder = self.agent.get(&current);
            if let Some((offset, validator)) = range {
                builder = builder
                    .header("Range", format!("bytes={offset}-"))
                    .header("If-Range", validator);
            }
            let builder = builder.config();
            let request = match body_budget {
                Some(budget) => builder.timeout_recv_body(Some(budget)).build(),
                None => builder.build(),
            };
            let response = request.call().map_err(|e| map_ureq(&e))?;
            let status = response.status().as_u16();
            match status {
                200 | 206 => {
                    if status == 206 && range.is_none() {
                        return Err(network(
                            "the server answered with unexpected partial content",
                        ));
                    }
                    let encoding = response
                        .headers()
                        .get("content-encoding")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("identity");
                    if !encoding.eq_ignore_ascii_case("identity") {
                        return Err(network(
                            "the server answered with an unsupported content encoding",
                        ));
                    }
                    return Ok((status, response));
                }
                416 if range.is_some() => {
                    return Err(Error::new(
                        ErrorKind::Integrity,
                        "resume refused: the server cannot answer the requested range",
                    ));
                }
                301 | 302 | 303 | 307 | 308 => {
                    let location = response
                        .headers()
                        .get("location")
                        .and_then(|v| v.to_str().ok())
                        .ok_or_else(|| network("a redirect without a target"))?;
                    current = redirect_target(&current, location)?;
                }
                401 | 403 => {
                    return Err(network(format!(
                        "the server refused access (status {status}); authentication is not supported yet"
                    )));
                }
                404 | 410 => return Err(network("the file was not found on the server")),
                other => return Err(network(format!("the server answered with status {other}"))),
            }
        }
        Err(network("too many redirects"))
    }

    fn body_budget(&self, size: u64) -> Duration {
        let rate = self.config.min_bytes_per_second.max(1);
        Duration::from_secs(size / rate).max(self.config.min_body_timeout)
    }

    /// Downloads a metadata file completely, within the size limit.
    fn metadata_file(&self, name: &str) -> Result<Vec<u8>, Error> {
        let (_, response) = self.get(
            &self.url_for(&format!(".a3s/{name}")),
            Some(Duration::from_secs(300)),
            None,
        )?;
        let mut body = response.into_body();
        let mut reader = body.as_reader();
        let mut data = Vec::new();
        reader
            .by_ref()
            .take(MAX_METADATA_BYTES + 1)
            .read_to_end(&mut data)
            .map_err(|_| network("the connection was lost while reading the file list"))?;
        if data.len() as u64 > MAX_METADATA_BYTES {
            return Err(Error::new(
                ErrorKind::Format,
                "the published file list is larger than accepted",
            ));
        }
        Ok(data)
    }

    /// The manifest and everything known about the published metadata (warnings, protected and unsupported paths).
    ///
    /// # Errors
    /// [`ErrorKind::Network`] for transport problems; [`ErrorKind::Format`] for metadata that is not supported or
    /// not usable.
    pub fn metadata(&self) -> Result<SyncMetadata, Error> {
        let sync = self.metadata_file("sync")?;
        let info = self.metadata_file("serverinfo")?;
        read_sync_metadata(&sync, &info)
    }
}

/// A repository together with **one** reading of its published metadata. Planning and applying work on this
/// snapshot, so a command sees one consistent revision; every file is still verified against its checksum.
#[derive(Debug, Clone)]
pub struct HttpSnapshot {
    repository: HttpRepository,
    metadata: SyncMetadata,
}

impl HttpRepository {
    /// Reads the metadata once and returns the snapshot.
    ///
    /// # Errors
    /// As [`HttpRepository::metadata`].
    pub fn snapshot(&self) -> Result<HttpSnapshot, Error> {
        Ok(HttpSnapshot {
            repository: self.clone(),
            metadata: self.metadata()?,
        })
    }
}

impl HttpSnapshot {
    /// What was read: the manifest, warnings, protected and unsupported paths.
    #[must_use]
    pub fn metadata(&self) -> &SyncMetadata {
        &self.metadata
    }
}

impl RepositorySource for HttpSnapshot {
    fn manifest(&self) -> Result<Manifest, Error> {
        Ok(self.metadata.manifest.clone())
    }

    fn read(&self, path: &RelativePath) -> Result<Vec<u8>, Error> {
        self.repository.read(path)
    }

    fn identity(&self) -> &str {
        self.repository.identity()
    }

    fn open(
        &self,
        path: &RelativePath,
        expected_size: u64,
        offset: u64,
        if_range: Option<&str>,
    ) -> Result<Opened, Error> {
        self.repository.open(path, expected_size, offset, if_range)
    }
}

impl RepositorySource for HttpRepository {
    fn manifest(&self) -> Result<Manifest, Error> {
        Ok(self.metadata()?.manifest)
    }

    fn identity(&self) -> &str {
        &self.base
    }

    fn read(&self, path: &RelativePath) -> Result<Vec<u8>, Error> {
        let (_, response) = self.get(
            &self.url_for(path.as_str()),
            Some(self.body_budget(MAX_READ_BYTES)),
            None,
        )?;
        let mut body = response.into_body();
        let mut reader = body.as_reader();
        let mut data = Vec::new();
        reader
            .by_ref()
            .take(MAX_READ_BYTES + 1)
            .read_to_end(&mut data)
            .map_err(|_| network("the connection was lost during the transfer"))?;
        if data.len() as u64 > MAX_READ_BYTES {
            return Err(Error::new(
                ErrorKind::Integrity,
                "the file is too large to be read into memory; it must be streamed",
            ));
        }
        Ok(data)
    }

    fn open(
        &self,
        path: &RelativePath,
        expected_size: u64,
        offset: u64,
        if_range: Option<&str>,
    ) -> Result<Opened, Error> {
        let url = self.url_for(path.as_str());
        let remaining = expected_size.saturating_sub(offset);
        let range = match if_range {
            Some(validator) if offset > 0 && is_strong_validator(validator) => {
                Some((offset, validator))
            }
            _ => None,
        };
        let (status, response) = self.get(&url, Some(self.body_budget(remaining)), range)?;
        let validator = response
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .filter(|v| is_strong_validator(v))
            .map(str::to_owned);
        let content_length = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok());
        let start = match range {
            Some((offset, wanted)) if status == 206 => {
                let content_range = response
                    .headers()
                    .get("content-range")
                    .and_then(|v| v.to_str().ok())
                    .and_then(parse_content_range);
                let last = expected_size.saturating_sub(1);
                let exact = content_range == Some((offset, last, expected_size))
                    && validator.as_deref() == Some(wanted)
                    && content_length.is_none_or(|len| len == remaining);
                if !exact {
                    return Err(refused_resume());
                }
                offset
            }
            _ => {
                if content_length.is_some_and(|len| len != expected_size) {
                    return Err(Error::new(
                        ErrorKind::Integrity,
                        "the server announces a different size than the file list",
                    ));
                }
                0
            }
        };
        Ok(Opened {
            start,
            validator,
            body: Box::new(response.into_body().into_reader()),
        })
    }
}

fn refused_resume() -> Error {
    Error::new(
        ErrorKind::Integrity,
        "resume refused: the answer does not match the requested range or validator",
    )
}

/// Parses `bytes <first>-<last>/<total>` into its three numbers.
fn parse_content_range(value: &str) -> Option<(u64, u64, u64)> {
    let rest = value.strip_prefix("bytes ")?;
    let (span, total) = rest.split_once('/')?;
    let (first, last) = span.split_once('-')?;
    Some((first.parse().ok()?, last.parse().ok()?, total.parse().ok()?))
}
