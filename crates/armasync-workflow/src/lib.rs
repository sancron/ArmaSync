// SPDX-License-Identifier: AGPL-3.0-only
//! Workflows shared by the command line and the GUI (MIG-031).
//!
//! Both frontends need the same wiring: the repository's published metadata is read once, the sync use cases of
//! `armasync-application` plan and apply against the local destination adapter, and the protected and unsupported
//! paths of the metadata always become part of the plan options. That wiring lives here so that no frontend
//! carries its own copy of the rules:
//!
//! * [`parse_managed_roots`]: extra files may only be deleted with an explicit, safe set of folders;
//! * [`prepare`]: read the metadata, build the plan options (protected paths included), plan **read-only**;
//! * [`execute`]: take the exclusive destination lock, re-validate and apply the plan (streamed, verified, staged,
//!   atomic; see ADR-0007);
//! * [`platform_status`]: which platform the synchronisation has actually been verified on.
//!
//! The crate has no GUI and no terminal dependency and contains no sync algorithm of its own.

use std::path::{Path, PathBuf};

use armasync_adapters::Sha256Hasher;
use armasync_adapters::http::{HttpConfig, HttpRepository, HttpSnapshot};
use armasync_adapters::localfs::LocalDirectory;
use armasync_adapters::sync_meta::SyncMetadata;
use armasync_application::sync::{ApplyReport, SyncContext, apply, plan_cancellable};
use armasync_core::{
    CancelToken, Error, ErrorKind, EventSink, OperationId, PlanOptions, RelativePath, SyncPlan,
};

/// What to synchronise.
#[derive(Debug, Clone)]
pub struct SyncRequest {
    /// Address of the repository (`http` or `https`, no credentials).
    pub repository_url: String,
    /// The destination folder; it must already exist.
    pub destination: PathBuf,
    /// Delete local files the repository does not contain (only inside `managed_roots`).
    pub delete_extra: bool,
    /// Folders (relative to the destination) where extra files may be deleted.
    pub managed_roots: Vec<RelativePath>,
    /// HTTP client settings (safe defaults).
    pub http: HttpConfig,
}

/// Validates the folder choices for deleting extra files.
///
/// # Errors
/// [`ErrorKind::Usage`] if deletion is requested without a folder, a folder is given without deletion, or a folder
/// is not a safe relative path.
pub fn parse_managed_roots(
    texts: &[String],
    delete_extra: bool,
) -> Result<Vec<RelativePath>, Error> {
    if delete_extra && texts.is_empty() {
        return Err(Error::new(
            ErrorKind::Usage,
            "deleting extra files needs at least one folder naming where they may be deleted",
        ));
    }
    if !delete_extra && !texts.is_empty() {
        return Err(Error::new(
            ErrorKind::Usage,
            "folders for deletion only make sense together with the option to delete extra files",
        ));
    }
    texts
        .iter()
        .map(|text| {
            RelativePath::parse(text.trim()).map_err(|e| {
                Error::new(
                    ErrorKind::Usage,
                    format!(
                        "'{}' is not a safe folder relative to the destination ({e})",
                        text.trim()
                    ),
                )
            })
        })
        .collect()
}

/// A plan together with everything needed to apply it.
#[derive(Debug)]
pub struct PreparedSync {
    source: HttpSnapshot,
    options: PlanOptions,
    plan: SyncPlan,
    destination: PathBuf,
}

impl PreparedSync {
    /// The plan (nothing has been changed yet).
    #[must_use]
    pub fn plan(&self) -> &SyncPlan {
        &self.plan
    }

    /// The published metadata (manifest, warnings, protected and unsupported paths).
    #[must_use]
    pub fn metadata(&self) -> &SyncMetadata {
        self.source.metadata()
    }

    /// The plan options in force (protected and unsupported paths always included).
    #[must_use]
    pub fn options(&self) -> &PlanOptions {
        &self.options
    }

    /// The destination folder.
    #[must_use]
    pub fn destination(&self) -> &Path {
        &self.destination
    }
}

/// Reads the repository's metadata once and plans the synchronisation. Changes nothing and opens the destination
/// read-only.
///
/// # Errors
/// [`ErrorKind::Unsupported`] for other schemes; [`ErrorKind::Network`] for transport and TLS problems;
/// [`ErrorKind::Format`] for unusable metadata; [`ErrorKind::FileSystem`] if the destination is unusable;
/// [`ErrorKind::Cancelled`].
pub fn prepare(request: &SyncRequest, cancel: &CancelToken) -> Result<PreparedSync, Error> {
    // The rule holds for every frontend: no deletion without an explicit, non-empty choice of folders.
    if request.delete_extra && request.managed_roots.is_empty() {
        return Err(Error::new(
            ErrorKind::Usage,
            "deleting extra files needs at least one folder naming where they may be deleted",
        ));
    }
    let cancelled = || Error::new(ErrorKind::Cancelled, "cancelled; nothing was changed");
    if cancel.is_cancelled() {
        return Err(cancelled());
    }
    let repository = HttpRepository::new(&request.repository_url, request.http.clone())?;
    let source = repository.snapshot()?;
    if cancel.is_cancelled() {
        return Err(cancelled());
    }
    // The protected and unsupported paths of the metadata are always part of the options.
    let options = source
        .metadata()
        .plan_options(request.delete_extra, request.managed_roots.clone());
    let store = LocalDirectory::new(&request.destination)?;
    let plan = plan_cancellable(&source, &store, &options, cancel)?;
    Ok(PreparedSync {
        source,
        options,
        plan,
        destination: request.destination.clone(),
    })
}

/// Applies a prepared plan. Takes the exclusive lock of the destination (a second run fails with
/// [`ErrorKind::Config`]), re-validates the plan against the current local state and transfers, verifies and
/// installs each file; any failure leaves existing files unchanged.
///
/// # Errors
/// As [`prepare`], plus [`ErrorKind::Integrity`] (stale plan, wrong size or checksum) and [`ErrorKind::Config`]
/// (destination in use).
pub fn execute(
    prepared: &PreparedSync,
    operation: OperationId,
    sink: &mut dyn EventSink,
    cancel: &CancelToken,
) -> Result<ApplyReport, Error> {
    let mut store = LocalDirectory::open_exclusive(&prepared.destination)?;
    let hasher = Sha256Hasher;
    let mut context = SyncContext {
        source: &prepared.source,
        store: &mut store,
        hasher: &hasher,
        sink,
        cancel,
    };
    apply(operation, &prepared.plan, &prepared.options, &mut context)
}

/// On which platforms the synchronisation has been verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformStatus {
    /// Verified; `detail` says how far (shown to users).
    Verified {
        /// What exactly was verified and what was not.
        detail: &'static str,
    },
    /// Not released: changing the destination is refused unless explicitly overridden.
    NotReleased {
        /// Why.
        reason: &'static str,
    },
}

impl PlatformStatus {
    /// Whether changing the destination is allowed without an override.
    #[must_use]
    pub const fn run_allowed(self) -> bool {
        matches!(self, Self::Verified { .. })
    }

    /// The text for users.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::Verified { detail } => detail,
            Self::NotReleased { reason } => reason,
        }
    }
}

/// The status for an operating system name (`std::env::consts::OS`).
#[must_use]
pub fn platform_status_for(os: &str) -> PlatformStatus {
    match os {
        "windows" => PlatformStatus::Verified {
            detail: "Windows: der Ablauf ist mit einem lokalen Testserver geprüft (HTTP und HTTPS). Echte Repository-Server und die Bedienung mit Screenreadern sind noch nicht geprüft.",
        },
        "linux" => PlatformStatus::NotReleased {
            reason: "Linux: die Synchronisation ist noch nicht freigegeben. Die Absicherung der Pfadgrenze gegen gleichzeitig laufende Prozesse fehlt unter Linux, und der Ablauf wurde dort noch nicht geprüft. Der Plan kann angesehen werden; es wird nichts verändert.",
        },
        _ => PlatformStatus::NotReleased {
            reason: "Auf diesem System ist die Synchronisation nicht freigegeben und nicht geprüft. Der Plan kann angesehen werden; es wird nichts verändert.",
        },
    }
}

/// The status of the system this program runs on.
#[must_use]
pub fn platform_status() -> PlatformStatus {
    platform_status_for(std::env::consts::OS)
}

/// Refuses to change anything on a platform that is not released, unless the caller explicitly overrides (the
/// command line offers `--allow-unreleased-platform` **only in validation builds** made with the feature
/// `unreleased-platform-override`; a normal or release build always passes `false`).
///
/// # Errors
/// [`ErrorKind::Unsupported`].
pub fn require_run_allowed(status: PlatformStatus, allow_unreleased: bool) -> Result<(), Error> {
    if status.run_allowed() || allow_unreleased {
        Ok(())
    } else {
        Err(Error::new(ErrorKind::Unsupported, status.message()))
    }
}
