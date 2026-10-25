// SPDX-License-Identifier: AGPL-3.0-only
//! Versioned events of an upload publication (MIG-042, step 4c; ADR-0016).
//!
//! The events describe the revision files, their bytes and the end of the operation. They carry typed data only: no
//! passwords, no addresses, no absolute local paths and no error text. Only the error category and relative paths of
//! revision files appear in them.

use crate::error::ErrorKind;
use crate::ids::OperationId;
use crate::path::RelativePath;

/// Schema version of [`UploadEvent`]; bump on incompatible changes (ADR-0016).
pub const UPLOAD_EVENT_SCHEMA_VERSION: u32 = 1;

/// Counters of an upload publication. See ADR-0016 for their exact meaning.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UploadProgress {
    /// Revision files completed.
    pub files_done: u64,
    /// Revision files that are uploaded in this operation.
    pub files_total: u64,
    /// Bytes of completed files plus bytes sent of the running file.
    pub bytes_done: u64,
    /// Bytes of the files that are uploaded in this operation.
    pub bytes_total: u64,
}

/// What happened, with the data that belongs to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadEventKind {
    /// The operation started; the totals are known from the plan.
    Started,
    /// A revision file starts; `size` is the size the plan announced.
    FileStarted {
        /// Path of the file within the revision.
        path: RelativePath,
        /// Size in bytes.
        size: u64,
    },
    /// Bytes of the running file were read for the upload; `bytes_done` counts this file only.
    FileBytes {
        /// Path of the file within the revision.
        path: RelativePath,
        /// Bytes of this file so far.
        bytes_done: u64,
    },
    /// A revision file was uploaded completely.
    FileFinished {
        /// Path of the file within the revision.
        path: RelativePath,
    },
    /// The publication finished successfully.
    Finished,
    /// The publication was cancelled through its cancellation token.
    Cancelled,
    /// The publication failed with this category; the text of the error is not part of the event.
    Failed {
        /// Error category.
        kind: ErrorKind,
    },
}

impl UploadEventKind {
    /// The stable event name for logs and JSON lines (ADR-0016).
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Started => "upload.started",
            Self::FileStarted { .. } => "upload.file_started",
            Self::FileBytes { .. } => "upload.file_bytes",
            Self::FileFinished { .. } => "upload.file_finished",
            Self::Finished => "upload.finished",
            Self::Cancelled => "upload.cancelled",
            Self::Failed { .. } => "upload.failed",
        }
    }

    /// Whether this event ends the operation. Exactly one terminal event comes last.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Finished | Self::Cancelled | Self::Failed { .. })
    }
}

/// One event of an upload publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadEvent {
    /// [`UPLOAD_EVENT_SCHEMA_VERSION`] at creation time.
    pub schema_version: u32,
    /// The operation this event belongs to.
    pub operation: OperationId,
    /// Starts at 0 and grows by exactly 1 per event of the operation.
    pub sequence: u64,
    /// What happened.
    pub kind: UploadEventKind,
    /// Counters at this event.
    pub progress: UploadProgress,
}

/// Receiver of upload events. It is owned by the operation and called from the operation's thread; it must not block
/// for long (ADR-0016).
pub trait UploadEventSink: Send {
    /// Delivers one event.
    fn emit(&mut self, event: UploadEvent);
}

#[cfg(test)]
mod tests {
    use super::{UPLOAD_EVENT_SCHEMA_VERSION, UploadEventKind};
    use crate::error::ErrorKind;
    use crate::path::RelativePath;

    #[test]
    fn names_are_stable_and_terminal_kinds_are_marked() {
        let path = RelativePath::parse("a.txt").unwrap();
        assert_eq!(UPLOAD_EVENT_SCHEMA_VERSION, 1);
        assert_eq!(UploadEventKind::Started.name(), "upload.started");
        assert_eq!(
            UploadEventKind::FileStarted {
                path: path.clone(),
                size: 1
            }
            .name(),
            "upload.file_started"
        );
        assert_eq!(
            UploadEventKind::FileBytes {
                path: path.clone(),
                bytes_done: 1
            }
            .name(),
            "upload.file_bytes"
        );
        assert_eq!(
            UploadEventKind::FileFinished { path }.name(),
            "upload.file_finished"
        );
        assert_eq!(UploadEventKind::Finished.name(), "upload.finished");
        assert_eq!(UploadEventKind::Cancelled.name(), "upload.cancelled");
        assert_eq!(
            UploadEventKind::Failed {
                kind: ErrorKind::Network
            }
            .name(),
            "upload.failed"
        );
        assert!(UploadEventKind::Finished.is_terminal());
        assert!(UploadEventKind::Cancelled.is_terminal());
        assert!(!UploadEventKind::Started.is_terminal());
    }
}
