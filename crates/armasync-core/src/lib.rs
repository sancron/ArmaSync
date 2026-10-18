// SPDX-License-Identifier: AGPL-3.0-only
//! ArmaSync core: product identity and the shared contracts (paths, manifests, plans, errors, events, jobs, ports).
//!
//! The core has no GUI, operating-system or network dependencies and only uses the standard library
//! (ADR-0001, ADR-0004). Frontends and adapters depend on it, never the other way round.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod error;
pub mod event;
pub mod hash;
pub mod ids;
pub mod job;
pub mod manifest;
pub mod path;
pub mod plan;
pub mod port;
pub mod product;

pub use error::{Error, ErrorKind};
pub use event::{EVENT_SCHEMA_VERSION, EventSink, OperationEvent, Phase, Progress};
pub use hash::{ContentHash, HashAlgorithm, HashError};
pub use ids::{IdError, OperationId, RepositoryId};
pub use job::{InvalidTransition, JobState};
pub use manifest::{FileEntry, Manifest, ManifestError};
pub use path::{PathError, RelativePath};
pub use plan::{LocalFile, PlanOptions, PlanSummary, PlanWarning, SyncPlan, SyncStep, plan_sync};
pub use port::{CancelToken, Hasher, LocalStore, RepositorySource};
