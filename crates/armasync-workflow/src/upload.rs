// SPDX-License-Identifier: AGPL-3.0-only
//! Upload wiring shared by the CLI and the GUI (MIG-042, step 4; ADR-0014). The use cases live in `armasync-application`;
//! this module only validates the request, builds the adapters and calls the use cases. No business rule is duplicated
//! in a frontend.

use std::fmt;
use std::path::PathBuf;

use armasync_adapters::Sha256Hasher;
use armasync_adapters::http::HttpConfig;
use armasync_adapters::localfs::LocalDirectory;
use armasync_adapters::webdav::{BasicCredentials, WebDavClient};
use armasync_application::progress::UploadObserver;
use armasync_application::revision::{self, PublishReport};
use armasync_application::upload::{self, UploadPlan};
use armasync_core::exclusion::ExclusionRules;
use armasync_core::{CancelToken, Error, ErrorKind, OperationId, RelativePath, UploadEventSink};

/// What an upload needs from a frontend. The password is held in memory only for the operation; it is never printed,
/// logged or stored (ADR-0014).
#[derive(Clone)]
pub struct UploadRequest {
    /// The local folder to upload.
    pub source: PathBuf,
    /// The WebDAV folder as `https://…` (or `http://…` without credentials). No user name, password, query or fragment.
    pub target: String,
    /// The user name for Basic authentication, if any.
    pub user: Option<String>,
    /// The password for Basic authentication, if any. Only sent over HTTPS.
    pub password: Option<String>,
    /// Explicit exclusions, relative to the source (a file or a folder with all its content).
    pub exclusions: Vec<String>,
    /// Transport configuration; frontends use the default (platform trust store).
    pub http: HttpConfig,
}

impl fmt::Debug for UploadRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UploadRequest")
            .field("source", &self.source)
            .field("target", &self.target)
            .field("user", &self.user)
            .field("password", &self.password.as_ref().map(|_| "<redacted>"))
            .field("exclusions", &self.exclusions)
            .finish_non_exhaustive()
    }
}

/// The plan for `request`: what would be uploaded and what is excluded, and why. Changes nothing on the target.
///
/// # Errors
/// [`ErrorKind::Usage`] for an unsafe exclusion; [`ErrorKind::Config`] for an unusable address or credentials;
/// [`ErrorKind::FileSystem`] for a source that is not a folder; transport errors; [`ErrorKind::Cancelled`].
pub fn preview(request: &UploadRequest, cancel: &CancelToken) -> Result<UploadPlan, Error> {
    let prepared = prepare(request)?;
    upload::plan_with_rules(&prepared.source, &prepared.target, &prepared.rules, cancel)
}

/// Publishes `request` as a revision (ADR-0012). The caller must have shown the preview and obtained the confirmation.
///
/// # Errors
/// As [`preview`], plus [`ErrorKind::Integrity`] for an unverifiable publication; the previous publication stays
/// unchanged on every error.
pub fn publish(request: &UploadRequest, cancel: &CancelToken) -> Result<PublishReport, Error> {
    let prepared = prepare(request)?;
    revision::publish_with_rules(
        &prepared.source,
        &prepared.target,
        &Sha256Hasher,
        &prepared.rules,
        cancel,
    )
}

/// As [`publish`], and reports the progress to `sink` as typed events (ADR-0016). The events never contain the
/// password, the address, absolute paths or error texts.
///
/// # Errors
/// As [`publish`].
pub fn publish_observed(
    request: &UploadRequest,
    cancel: &CancelToken,
    operation: OperationId,
    sink: Box<dyn UploadEventSink>,
) -> Result<PublishReport, Error> {
    let observer = UploadObserver::new(operation, sink);
    // A refusal before the publication starts is an operation too: it ends with a `Failed` event (ADR-0016).
    let result = prepare(request).and_then(|prepared| {
        revision::publish_observed(
            &prepared.source,
            &prepared.target,
            &Sha256Hasher,
            &prepared.rules,
            cancel,
            &observer,
        )
    });
    observer.end(&result);
    result
}

/// Parses the exclusions of a request. Each must be a safe relative path (ADR-0013).
///
/// # Errors
/// [`ErrorKind::Usage`] naming the first unsafe exclusion.
pub fn parse_exclusions(raw: &[String]) -> Result<ExclusionRules, Error> {
    let mut paths = Vec::with_capacity(raw.len());
    for item in raw {
        let path = RelativePath::parse(item).map_err(|_| {
            Error::new(
                ErrorKind::Usage,
                format!("the exclusion '{item}' is not a safe path relative to the source folder"),
            )
        })?;
        paths.push(path);
    }
    Ok(ExclusionRules::new(paths))
}

/// Refuses addresses that carry credentials or other parts: credentials belong in `user` and `password`.
///
/// # Errors
/// [`ErrorKind::Config`] for a user name, a password, a query or a fragment in the address.
pub fn check_target(target: &str) -> Result<(), Error> {
    let rest = target.split_once("://").map_or("", |(_, rest)| rest);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.contains('@') || target.contains('?') || target.contains('#') {
        return Err(Error::new(
            ErrorKind::Config,
            "the address must not contain a user name, a password, a query or a fragment; use the user and password options instead",
        ));
    }
    Ok(())
}

struct Prepared {
    source: LocalDirectory,
    target: WebDavClient,
    rules: ExclusionRules,
}

fn prepare(request: &UploadRequest) -> Result<Prepared, Error> {
    let rules = parse_exclusions(&request.exclusions)?;
    check_target(&request.target)?;
    let credentials = match (&request.user, &request.password) {
        (None, None) => None,
        (Some(user), Some(password)) => Some(BasicCredentials::new(user, password)?),
        (Some(_), None) => {
            return Err(Error::new(
                ErrorKind::Config,
                "a user name was given without a password; provide the password through the password variable",
            ));
        }
        (None, Some(_)) => {
            return Err(Error::new(
                ErrorKind::Config,
                "a password was given without a user name",
            ));
        }
    };
    let source = LocalDirectory::new(&request.source)?;
    let target = WebDavClient::new(&request.target, request.http.clone(), credentials)?;
    Ok(Prepared {
        source,
        target,
        rules,
    })
}
