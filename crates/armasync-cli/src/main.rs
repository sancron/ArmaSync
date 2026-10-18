// SPDX-License-Identifier: AGPL-3.0-only
//! `armasync` binary: thin wrapper around [`armasync_cli::run`].

fn main() -> std::process::ExitCode {
    let status = armasync_cli::run(
        std::env::args_os(),
        &mut std::io::stdout().lock(),
        &mut std::io::stderr().lock(),
    );
    std::process::ExitCode::from(status.code())
}
