// SPDX-License-Identifier: AGPL-3.0-only
//! ArmaSync command line interface.
//!
//! Contract: `docs/migration/CLI_CONTRACT.md`. Structured results go to stdout, diagnostics to stderr, and the
//! process never waits for input (`--no-input` is accepted for forward compatibility; there is no prompt yet).
//! `version`, `import` and `repository import` exist so far; synchronisation commands are added with the use cases in later tasks.

use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

mod import_cmd;
mod list_cmd;

/// Version of the JSON envelope schema.
pub const JSON_SCHEMA_VERSION: u32 = 1;

/// Process exit codes (CLI contract, "Vorgeschlagene Exitcodes").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitStatus {
    /// Successful.
    Success,
    /// Argument or usage error.
    Usage,
    /// Configuration, missing consent or credentials.
    Config,
    /// Network, authentication or trust failure.
    Network,
    /// File system, permissions or disk space.
    FileSystem,
    /// Integrity or verification failure.
    Integrity,
    /// Format or migration failure.
    Format,
    /// Unsupported platform or transport capability.
    Unsupported,
    /// Explicit cancellation (Ctrl+C).
    Cancelled,
}

impl ExitStatus {
    /// Numeric process exit code.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::Usage => 2,
            Self::Config => 3,
            Self::Network => 4,
            Self::FileSystem => 5,
            Self::Integrity => 6,
            Self::Format => 7,
            Self::Unsupported => 8,
            Self::Cancelled => 130,
        }
    }
}

/// ArmaSync command line.
#[derive(Debug, Parser)]
#[command(
    name = "armasync",
    version,
    about = "ArmaSync by Lynxnode.it - command line interface",
    arg_required_else_help = true
)]
struct Cli {
    /// Emit a versioned JSON result on stdout instead of text.
    #[arg(long, global = true)]
    json: bool,
    /// Never wait for input; fail with a clear error instead.
    #[arg(long, global = true)]
    no_input: bool,
    /// Suppress non-essential text output.
    #[arg(long, global = true)]
    quiet: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show product, version and platform.
    Version,
    /// Import repositories and profiles from a neutral import file (preview unless --apply is given).
    Import(ImportArgs),
    /// Work with repositories.
    Repository(RepositoryArgs),
    /// Work with profiles.
    Profile(ProfileArgs),
}

#[derive(Debug, Args)]
struct RepositoryArgs {
    #[command(subcommand)]
    command: RepositoryCommand,
}

#[derive(Debug, Args)]
struct ProfileArgs {
    #[command(subcommand)]
    command: ProfileCommand,
}

#[derive(Debug, Subcommand)]
enum ProfileCommand {
    /// List the profiles in the library (read-only).
    List(ListArgs),
}

#[derive(Debug, Args)]
struct ListArgs {
    /// Config directory holding `library.json` (default: the platform's ArmaSync config directory).
    #[arg(long, value_name = "DIR")]
    config: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum RepositoryCommand {
    /// List the repositories in the library (read-only; never shows credentials).
    List(ListArgs),
    /// Import a repository from the settings its server publishes (`.a3s/autoconfig`). Reads only a local file
    /// (no download), only the name and address, never passwords; preview unless --apply is given. This does not
    /// replace your own repository files and imports no settings or addon lists.
    Import(ImportArgs),
}

#[derive(Debug, Args)]
struct ImportArgs {
    /// The file to import: a neutral import file (`armasync.import`) for `import`, a repository's
    /// `.a3s/autoconfig` file for `repository import`.
    file: PathBuf,
    /// Write the changes. Without this flag the command only shows what would happen.
    #[arg(long)]
    apply: bool,
    /// What to do when an imported item collides with an existing one.
    #[arg(long, value_name = "POLICY", default_value = "keep-both",
          value_parser = ["keep-both", "fail", "replace"])]
    on_conflict: String,
    /// Config directory holding `library.json` (default: the platform's ArmaSync config directory).
    #[arg(long, value_name = "DIR")]
    config: Option<PathBuf>,
}

/// Runs the CLI with the given arguments and returns the exit status.
pub fn run<I, T>(args: I, stdout: &mut dyn Write, stderr: &mut dyn Write) -> ExitStatus
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(error) => {
            // A closed pipe must not turn into a panic; the exit status is what matters.
            if error.use_stderr() {
                let _ = write!(stderr, "{}", error.render());
                return ExitStatus::Usage;
            }
            let _ = write!(stdout, "{}", error.render());
            return ExitStatus::Success;
        }
    };
    // No prompts exist yet, so `--no-input` changes nothing today.
    let _ = cli.no_input;
    match cli.command {
        Command::Version => version(cli.json, cli.quiet, stdout),
        Command::Import(args) => run_import(
            import_cmd::SourceKind::Neutral,
            args,
            cli.json,
            cli.quiet,
            stdout,
            stderr,
        ),
        Command::Repository(RepositoryArgs {
            command: RepositoryCommand::List(args),
        }) => list_cmd::run(
            list_cmd::Kind::Repositories,
            args.config.as_ref(),
            cli.json,
            cli.quiet,
            stdout,
            stderr,
        ),
        Command::Profile(ProfileArgs {
            command: ProfileCommand::List(args),
        }) => list_cmd::run(
            list_cmd::Kind::Profiles,
            args.config.as_ref(),
            cli.json,
            cli.quiet,
            stdout,
            stderr,
        ),
        Command::Repository(RepositoryArgs {
            command: RepositoryCommand::Import(args),
        }) => run_import(
            import_cmd::SourceKind::Autoconfig,
            args,
            cli.json,
            cli.quiet,
            stdout,
            stderr,
        ),
    }
}

fn run_import(
    kind: import_cmd::SourceKind,
    args: ImportArgs,
    json: bool,
    quiet: bool,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> ExitStatus {
    let args = import_cmd::ImportArgs {
        kind,
        file: args.file,
        apply: args.apply,
        on_conflict: armasync_adapters::import::OnConflict::from_name(&args.on_conflict)
            .unwrap_or_default(),
        config: args.config,
    };
    import_cmd::run(&args, json, quiet, stdout, stderr)
}

fn version(json: bool, quiet: bool, stdout: &mut dyn Write) -> ExitStatus {
    let info = armasync_application::version_info();
    let platform = armasync_adapters::Platform::current();
    let written = if json {
        let envelope = serde_json::json!({
            "schema_version": JSON_SCHEMA_VERSION,
            "command": "version",
            "operation_id": serde_json::Value::Null,
            "status": "ok",
            "result": {
                "name": info.name,
                "publisher": info.publisher,
                "technical_id": info.technical_id,
                "version": info.version,
                "platform": platform.name(),
                "can_launch_game": platform.can_launch_game(),
            },
        });
        writeln!(stdout, "{envelope}")
    } else if quiet {
        writeln!(stdout, "{}", info.version)
    } else {
        writeln!(
            stdout,
            "{} {} ({}, {})",
            info.name,
            info.version,
            info.publisher,
            platform.name()
        )
    };
    if written.is_ok() {
        ExitStatus::Success
    } else {
        ExitStatus::FileSystem
    }
}

#[cfg(test)]
mod tests {
    use super::{ExitStatus, run};

    fn run_args(args: &[&str]) -> (ExitStatus, String, String) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let status = run(args.iter().copied(), &mut out, &mut err);
        (
            status,
            String::from_utf8(out).unwrap_or_default(),
            String::from_utf8(err).unwrap_or_default(),
        )
    }

    #[test]
    fn exit_codes_follow_contract() {
        let expected = [
            (ExitStatus::Success, 0),
            (ExitStatus::Usage, 2),
            (ExitStatus::Config, 3),
            (ExitStatus::Network, 4),
            (ExitStatus::FileSystem, 5),
            (ExitStatus::Integrity, 6),
            (ExitStatus::Format, 7),
            (ExitStatus::Unsupported, 8),
            (ExitStatus::Cancelled, 130),
        ];
        for (status, code) in expected {
            assert_eq!(status.code(), code);
        }
    }

    #[test]
    fn version_text() {
        let (status, out, err) = run_args(&["armasync", "version"]);
        assert_eq!(status, ExitStatus::Success);
        assert!(out.starts_with("ArmaSync 0.0.0 (Lynxnode.it, "), "{out}");
        assert!(err.is_empty());
    }

    #[test]
    fn version_json_is_a_versioned_envelope_without_secrets() {
        let (status, out, err) = run_args(&["armasync", "--json", "version"]);
        assert_eq!(status, ExitStatus::Success);
        assert!(err.is_empty());
        let value: serde_json::Value = serde_json::from_str(&out).unwrap_or_default();
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["status"], "ok");
        assert_eq!(value["result"]["technical_id"], "it.lynxnode.armasync");
        assert!(value["operation_id"].is_null());
    }

    #[test]
    fn global_flags_work_after_the_subcommand() {
        let (status, out, _) = run_args(&["armasync", "version", "--json", "--no-input"]);
        assert_eq!(status, ExitStatus::Success);
        assert!(out.trim_start().starts_with('{'));
    }

    #[test]
    fn unknown_command_is_a_usage_error_on_stderr() {
        let (status, out, err) = run_args(&["armasync", "sync-everything"]);
        assert_eq!(status, ExitStatus::Usage);
        assert!(out.is_empty());
        assert!(!err.is_empty());
    }

    #[test]
    fn no_arguments_prints_help_as_usage_error() {
        let (status, _, err) = run_args(&["armasync"]);
        assert_eq!(status, ExitStatus::Usage);
        assert!(err.contains("Usage"), "{err}");
    }

    #[test]
    fn help_and_version_flags_succeed_on_stdout() {
        let (status, out, _) = run_args(&["armasync", "--help"]);
        assert_eq!(status, ExitStatus::Success);
        assert!(out.contains("version"));
        let (status, out, _) = run_args(&["armasync", "--version"]);
        assert_eq!(status, ExitStatus::Success);
        assert!(out.contains("armasync"));
    }
}
