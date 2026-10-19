// SPDX-License-Identifier: AGPL-3.0-only
//! Handling of Ctrl+C / Ctrl+Break for the command line.
//!
//! The first interrupt asks the running operation to stop cleanly through the shared [`CancelToken`]: files that are
//! complete stay, the file in progress is discarded, existing files stay unchanged, the process ends with exit
//! status 130. A second interrupt means "now": the process ends at once (also safe, because every file is staged
//! and replaced atomically and the next run cleans up leftovers). The decision logic is separate from the operating
//! system hook so that it can be tested without sending signals.

use std::sync::atomic::{AtomicU32, Ordering};

use armasync_core::CancelToken;

/// What an interrupt led to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptAction {
    /// The first interrupt: cancellation was requested, the operation stops at the next safe point.
    CancelRequested,
    /// A further interrupt: stop immediately.
    ExitNow,
}

/// Counts interrupts and turns the first into a cancellation request.
#[derive(Debug)]
pub struct InterruptState {
    presses: AtomicU32,
    cancel: CancelToken,
}

impl InterruptState {
    /// A state that cancels `cancel` on the first interrupt.
    #[must_use]
    pub fn new(cancel: &CancelToken) -> Self {
        Self {
            presses: AtomicU32::new(0),
            cancel: cancel.clone(),
        }
    }

    /// Registers one interrupt.
    pub fn interrupted(&self) -> InterruptAction {
        if self.presses.fetch_add(1, Ordering::SeqCst) == 0 {
            self.cancel.cancel();
            InterruptAction::CancelRequested
        } else {
            InterruptAction::ExitNow
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_interrupt_cancels_cleanly_and_the_second_ends_the_process() {
        let cancel = CancelToken::new();
        let state = InterruptState::new(&cancel);
        assert!(!cancel.is_cancelled());
        assert_eq!(state.interrupted(), InterruptAction::CancelRequested);
        assert!(cancel.is_cancelled());
        assert_eq!(state.interrupted(), InterruptAction::ExitNow);
        assert_eq!(state.interrupted(), InterruptAction::ExitNow);
    }
}
