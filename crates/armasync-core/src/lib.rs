// SPDX-License-Identifier: AGPL-3.0-only
//! ArmaSync core: product identity and the shared contracts (paths, manifests, plans, errors, events, jobs, ports).
//!
//! The core has no GUI, operating-system or network dependencies and only uses the standard library
//! (ADR-0001, ADR-0004). Frontends and adapters depend on it, never the other way round.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod credential;
pub mod error;
pub mod event;
pub mod hash;
pub mod ids;
pub mod job;
pub mod manifest;
pub mod model;
pub mod path;
pub mod plan;
pub mod port;
pub mod product;

pub use credential::{Credential, CredentialStore, Secret};
pub use error::{Error, ErrorKind};
pub use event::{EVENT_SCHEMA_VERSION, EventSink, OperationEvent, Phase, Progress};
pub use hash::{ContentHash, HashAlgorithm, HashError};
pub use ids::{CredentialRef, IdError, OperationId, ProfileId, RepositoryId};
pub use job::{InvalidTransition, JobState};
pub use manifest::{FileEntry, Manifest, ManifestError};
pub use model::{
    DisplayName, Language, Library, ModelError, Profile, Repository, RepositoryUrl, Settings,
    Theme, UiScale,
};
pub use path::{PathError, RelativePath};
pub use plan::{LocalFile, PlanOptions, PlanSummary, PlanWarning, SyncPlan, SyncStep, plan_sync};
pub use port::{CancelToken, Hasher, LocalStore, RepositorySource};
