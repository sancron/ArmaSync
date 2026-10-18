// SPDX-License-Identifier: AGPL-3.0-only
//! Job lifecycle: `Queued -> Running -> {Succeeded, Failed, Cancelled}`.
//!
//! Cancelling is its own terminal state, never a success. A queued job may also be cancelled before it starts.
//! Pause/resume is intentionally absent until resume state is really persisted.

use std::fmt;

/// State of a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobState {
    /// Accepted, not started.
    Queued,
    /// In progress.
    Running,
    /// Finished successfully.
    Succeeded,
    /// Finished with an error.
    Failed,
    /// Stopped on request.
    Cancelled,
}

/// A rejected state change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidTransition {
    /// State before.
    pub from: JobState,
    /// Requested state.
    pub to: JobState,
}

impl fmt::Display for InvalidTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid job transition {:?} -> {:?}", self.from, self.to)
    }
}

impl std::error::Error for InvalidTransition {}

impl JobState {
    /// Whether no further transition is possible.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }

    /// Whether `self -> next` is allowed.
    #[must_use]
    pub const fn can_become(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Queued, Self::Running | Self::Cancelled)
                | (
                    Self::Running,
                    Self::Succeeded | Self::Failed | Self::Cancelled
                )
        )
    }

    /// Performs the transition.
    ///
    /// # Errors
    /// [`InvalidTransition`] if it is not allowed.
    pub const fn transition(self, next: Self) -> Result<Self, InvalidTransition> {
        if self.can_become(next) {
            Ok(next)
        } else {
            Err(InvalidTransition {
                from: self,
                to: next,
            })
        }
    }

    /// Stable lower-case name for events and JSON.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::JobState::{Cancelled, Failed, Queued, Running, Succeeded};

    #[test]
    fn happy_and_cancel_paths() {
        assert_eq!(Queued.transition(Running).unwrap(), Running);
        assert_eq!(Running.transition(Succeeded).unwrap(), Succeeded);
        assert_eq!(Running.transition(Failed).unwrap(), Failed);
        assert_eq!(Running.transition(Cancelled).unwrap(), Cancelled);
        assert_eq!(Queued.transition(Cancelled).unwrap(), Cancelled);
    }

    #[test]
    fn terminal_states_are_final_and_cancel_is_not_success() {
        for terminal in [Succeeded, Failed, Cancelled] {
            assert!(terminal.is_terminal());
            for next in [Queued, Running, Succeeded, Failed, Cancelled] {
                assert!(
                    terminal.transition(next).is_err(),
                    "{terminal:?} -> {next:?}"
                );
            }
        }
        assert!(Queued.transition(Succeeded).is_err());
        assert!(Queued.transition(Failed).is_err());
        assert!(Running.transition(Queued).is_err());
        assert!(!Queued.is_terminal() && !Running.is_terminal());
        assert_ne!(Cancelled, Succeeded);
    }
}
