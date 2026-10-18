// SPDX-License-Identifier: AGPL-3.0-only
//! Versioned operation events. They carry typed data only: no passwords, tokens, private URL queries or
//! free-form text, so they are safe for logs, JSON lines and GUI binding.

use crate::error::ErrorKind;
use crate::ids::OperationId;
use crate::job::JobState;

/// Schema version of [`OperationEvent`]; bump on incompatible changes.
pub const EVENT_SCHEMA_VERSION: u32 = 1;

/// Coarse phase of an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Computing or revalidating the plan.
    Planning,
    /// Transferring and verifying files.
    Transferring,
    /// Removing files that the plan marks for deletion.
    Cleaning,
    /// The operation reached a terminal state.
    Finished,
}

/// File and byte counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Progress {
    /// Files completed.
    pub files_done: u64,
    /// Files expected in total.
    pub files_total: u64,
    /// Bytes completed.
    pub bytes_done: u64,
    /// Bytes expected in total.
    pub bytes_total: u64,
}

/// One progress or state event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationEvent {
    /// [`EVENT_SCHEMA_VERSION`] at creation time.
    pub schema_version: u32,
    /// The operation.
    pub operation: OperationId,
    /// Strictly increasing per operation, starting at 0.
    pub sequence: u64,
    /// Phase.
    pub phase: Phase,
    /// Job state at this event.
    pub state: JobState,
    /// Counters.
    pub progress: Progress,
    /// Error category for `Failed` events (and `Cancelled` for cancellations).
    pub error: Option<ErrorKind>,
}

/// Receiver of events. Implemented by frontends and test sinks; must not block for long.
pub trait EventSink {
    /// Delivers one event.
    fn emit(&mut self, event: OperationEvent);
}

#[cfg(test)]
mod tests {
    use super::{EVENT_SCHEMA_VERSION, Phase, Progress};

    #[test]
    fn schema_version_is_one() {
        assert_eq!(EVENT_SCHEMA_VERSION, 1);
        assert_eq!(Progress::default().files_done, 0);
        assert_ne!(Phase::Planning, Phase::Finished);
    }
}
