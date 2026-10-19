// SPDX-License-Identifier: AGPL-3.0-only
//! `armasync` binary: thin wrapper around [`armasync_cli::run`].

use armasync_cli::interrupt::{InterruptAction, InterruptState};
use armasync_core::CancelToken;

/// Ties Ctrl+C (and Ctrl+Break on Windows) to the cancellation of the running operation; see
/// [`armasync_cli::interrupt`].
fn install_interrupt_handler(cancel: &CancelToken) {
    let state = InterruptState::new(cancel);
    // If a handler cannot be installed the program still works; it then ends on Ctrl+C like any process.
    let _ = ctrlc::set_handler(move || match state.interrupted() {
        InterruptAction::CancelRequested => {
            eprintln!(
                "Stopping after the current step (press Ctrl+C again to stop immediately)..."
            );
        }
        InterruptAction::ExitNow => std::process::exit(130),
    });
}

fn main() -> std::process::ExitCode {
    let cancel = CancelToken::new();
    install_interrupt_handler(&cancel);
    let status = armasync_cli::run_with_cancel(
        std::env::args_os(),
        &mut std::io::stdout().lock(),
        &mut std::io::stderr().lock(),
        &cancel,
    );
    std::process::ExitCode::from(status.code())
}
