// SPDX-License-Identifier: AGPL-3.0-only
//! End-to-end tests of `armasync sync plan` and `armasync sync run`: the real binary, the real HTTP adapter, the
//! real local file-system adapter and a loopback repository server that publishes the MIG-001 `small` metadata with
//! payloads regenerated from the documented generator. No public server, no user data, temporary folders only.
#![cfg(any(windows, feature = "unreleased-platform-override"))]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};

use armasync_testkit::{
    BIKEY, Behavior, PBO, Server, ZIP, bikey, hang_up_server, pbo, serve, serve_tls,
};

// ---- helpers --------------------------------------------------------------------------------------------------

struct Env {
    root: PathBuf,
    config: PathBuf,
    dest: PathBuf,
}

impl Drop for Env {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn armasync_command(args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_armasync"));
    command.args(args).stdin(Stdio::null());
    command
}

fn armasync(args: &[&str]) -> Output {
    armasync_command(args)
        .output()
        .expect("armasync binary must start")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn path(p: &Path) -> &str {
    p.to_str().unwrap()
}

/// A config directory whose library contains the repository `example` at `base` (via the real import command).
fn env_for(base: &str, needs_credentials: bool) -> Env {
    let root = std::env::temp_dir().join(format!(
        "armasync-sync-e2e-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    // Unique even if the clock is coarse: two tests may read the same timestamp (this was a real source of
    // flaky failures when many tests ran at once), so the folder is created exclusively and suffixed on a clash.
    let mut root = root;
    let base_name = root.clone();
    let mut attempt = 0u32;
    while fs::create_dir(&root).is_err() {
        attempt += 1;
        assert!(attempt < 1000, "cannot create a unique scratch folder");
        root = PathBuf::from(format!("{}-{attempt}", base_name.display()));
    }
    let (config, dest) = (root.join("config"), root.join("dest"));
    fs::create_dir_all(&dest).unwrap();
    let import = root.join("import.json");
    fs::write(
        &import,
        format!(
            r#"{{"kind":"armasync.import","schema_version":1,
            "source":{{"tool":"test","tool_version":"1","format":"synthetic"}},
            "repositories":[{{"key":"Example","name":"Example","url":"{base}","needs_credentials":{needs_credentials}}}],
            "profiles":[]}}"#
        ),
    )
    .unwrap();
    let out = armasync(&[
        "import",
        path(&import),
        "--apply",
        "--config",
        path(&config),
    ]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    Env { root, config, dest }
}

fn http_env(server: &Server) -> Env {
    env_for(&format!("http://{}/repo", server.addr), false)
}

/// The validation override exists only in builds with the `unreleased-platform-override` feature (never in a
/// normal build). On Windows the synchronisation is released and needs no override; elsewhere these tests run only
/// when the feature is on (see the `cfg` at the top of this file).
const UNRELEASED: &str = "--allow-unreleased-platform";

/// Appends the override flag to a `sync run` command line if (and only if) this build has the feature.
fn ov<'a>(args: &[&'a str]) -> Vec<&'a str> {
    let mut all = args.to_vec();
    if cfg!(feature = "unreleased-platform-override") {
        all.push(UNRELEASED);
    }
    all
}

fn sync(env: &Env, sub: &str, extra: &[&str]) -> Output {
    let mut args = vec![
        "sync",
        sub,
        "example",
        "--destination",
        path(&env.dest),
        "--config",
        path(&env.config),
    ];
    args.extend_from_slice(extra);
    if sub == "run" {
        if cfg!(feature = "unreleased-platform-override") {
            args.push(UNRELEASED);
        }
    }
    armasync(&args)
}

fn sync_json(env: &Env, sub: &str, extra: &[&str]) -> (Output, serde_json::Value) {
    let mut args = vec![
        "--json",
        "sync",
        sub,
        "example",
        "--destination",
        path(&env.dest),
        "--config",
        path(&env.config),
    ];
    args.extend_from_slice(extra);
    if sub == "run" {
        if cfg!(feature = "unreleased-platform-override") {
            args.push(UNRELEASED);
        }
    }
    let out = armasync(&args);
    let value = serde_json::from_str(&text(&out.stdout)).unwrap_or(serde_json::Value::Null);
    (out, value)
}

/// All files below `dir` (relative, sorted), ignoring the adapter's lock file.
fn files(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
            if name == ".armasync-lock" {
                continue;
            }
            if entry.path().is_dir() {
                walk(&entry.path(), &format!("{name}/"), out);
            } else {
                out.push(name);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, "", &mut out);
    out.sort();
    out
}

fn read(env: &Env, relative: &str) -> Vec<u8> {
    fs::read(env.dest.join(relative)).unwrap()
}

fn no_temporary_files(env: &Env) {
    for name in files(&env.dest) {
        assert!(
            !name.contains(".armasync-"),
            "temporary file left behind: {name}"
        );
    }
}

// ---- tests ----------------------------------------------------------------------------------------------------

#[test]
fn plan_shows_the_work_and_changes_nothing() {
    let server = serve();
    let env = http_env(&server);
    let out = sync(&env, "plan", &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let stdout = text(&out.stdout);
    assert!(stdout.contains("2 to download"), "{stdout}");
    assert!(stdout.contains(&format!("download {PBO}")), "{stdout}");
    assert!(
        stdout.contains("compressed file not supported yet"),
        "{stdout}"
    );
    assert!(
        stdout.contains("Plan only: nothing was changed"),
        "{stdout}"
    );
    assert!(
        fs::read_dir(&env.dest).unwrap().next().is_none(),
        "plan must not even create the lock file"
    );
    assert_eq!(
        server.requested("example_core.pbo"),
        0,
        "plan transfers no file"
    );
}

#[test]
fn run_downloads_verified_files_and_a_second_run_changes_nothing() {
    let server = serve();
    let env = http_env(&server);
    let (out, value) = sync_json(&env, "run", &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(value["command"], "sync-run");
    assert_eq!(value["status"], "ok");
    assert_eq!(value["result"]["mode"], "applied");
    assert_eq!(value["result"]["changed"], true);
    assert_eq!(value["result"]["report"]["downloaded"], 2);
    assert_eq!(value["result"]["revision"], "3");
    assert_eq!(read(&env, PBO), pbo());
    assert_eq!(read(&env, BIKEY), bikey());
    assert_eq!(files(&env.dest), [PBO, BIKEY]);
    assert!(
        !files(&env.dest).iter().any(|f| f.contains("zip")),
        "the compressed file is not fetched"
    );
    assert_eq!(server.requested(".zip"), 0);
    no_temporary_files(&env);

    let (out, value) = sync_json(&env, "run", &[]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(value["result"]["changed"], false);
    assert_eq!(value["result"]["summary"]["keep_files"], 2);
    assert_eq!(
        server.requested("example_core.pbo"),
        1,
        "an unchanged file is not downloaded again"
    );
    // the requests carry no credentials
    for (_, headers) in server.requests.lock().unwrap().iter() {
        assert!(
            headers
                .iter()
                .all(|(k, _)| k != "authorization" && k != "cookie")
        );
    }
}

#[test]
fn text_output_and_quiet_mode() {
    let server = serve();
    let env = http_env(&server);
    let out = sync(&env, "run", &[]);
    let stdout = text(&out.stdout);
    assert!(
        stdout.contains("Done: 2 downloaded, 0 replaced, 0 deleted"),
        "{stdout}"
    );
    let env2 = http_env(&server);
    let out = armasync(&ov(&[
        "--quiet",
        "sync",
        "run",
        "example",
        "--destination",
        path(&env2.dest),
        "--config",
        path(&env2.config),
    ]));
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty());
    assert_eq!(read(&env2, PBO), pbo());
}

#[test]
fn dry_run_changes_nothing() {
    let server = serve();
    let env = http_env(&server);
    let (out, value) = sync_json(&env, "run", &["--dry-run"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(value["result"]["mode"], "dry-run");
    assert_eq!(value["result"]["changed"], false);
    assert!(fs::read_dir(&env.dest).unwrap().next().is_none());
    assert_eq!(server.requested("example_core.pbo"), 0);
}

#[test]
fn changed_files_are_replaced_and_extras_survive_unless_deletion_is_chosen() {
    let server = serve();
    let env = http_env(&server);
    fs::create_dir_all(env.dest.join("@example_core/addons")).unwrap();
    fs::write(env.dest.join(PBO), b"an old, different version").unwrap();
    fs::write(env.dest.join("@example_core/addons/extra.pbo"), b"extra").unwrap();
    fs::write(
        env.dest.join(ZIP),
        b"local unpacked copy of a compressed repository file",
    )
    .unwrap();
    fs::create_dir_all(env.dest.join("@second")).unwrap();
    fs::write(env.dest.join("@second/mine.txt"), b"mine").unwrap();
    // default: replace + download, delete nothing
    let (out, value) = sync_json(&env, "run", &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(value["result"]["report"]["replaced"], 1);
    assert_eq!(value["result"]["report"]["deleted"], 0);
    assert_eq!(read(&env, PBO), pbo());
    assert!(env.dest.join("@example_core/addons/extra.pbo").exists());
    // deletion only with the flag AND an explicit folder; protected paths survive
    let (out, value) = sync_json(
        &env,
        "run",
        &[
            "--delete-extra",
            "--managed-root",
            "@example_core",
            "--managed-root",
            "@second",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert_eq!(value["result"]["report"]["deleted"], 1);
    assert!(!env.dest.join("@example_core/addons/extra.pbo").exists());
    assert!(
        env.dest.join(ZIP).exists(),
        "a file the repository holds compressed is never deleted"
    );
    assert!(
        env.dest.join("@second/mine.txt").exists(),
        "a folder the repository protects is never cleaned"
    );
    let codes: Vec<_> = value["result"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        codes
            .iter()
            .filter(|c| *c == "extra-kept-protected")
            .count(),
        2,
        "{codes:?}"
    );
    assert_eq!(value["result"]["unsupported_paths"][0], ZIP);
    assert_eq!(value["result"]["protected_paths"][0], "@second");
}

#[test]
fn deletion_needs_an_explicit_folder_and_is_refused_before_any_network_access() {
    let server = serve();
    let env = http_env(&server);
    let out = sync(&env, "run", &["--delete-extra"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        text(&out.stderr).contains("--managed-root"),
        "{}",
        text(&out.stderr)
    );
    assert_eq!(server.total(), 0);
    let out = sync(&env, "plan", &["--managed-root", "x"]);
    assert_eq!(out.status.code(), Some(2));
    let out = sync(
        &env,
        "plan",
        &["--delete-extra", "--managed-root", "../outside"],
    );
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(server.total(), 0);
}

#[test]
fn bad_arguments_are_usage_or_file_system_errors() {
    let server = serve();
    let env = http_env(&server);
    let out = armasync(&[
        "sync",
        "plan",
        "nope",
        "--destination",
        path(&env.dest),
        "--config",
        path(&env.config),
    ]);
    assert_eq!(out.status.code(), Some(2));
    let out = armasync(&[
        "sync",
        "plan",
        "Not Valid!",
        "--destination",
        path(&env.dest),
        "--config",
        path(&env.config),
    ]);
    assert_eq!(out.status.code(), Some(2));
    let missing = env.root.join("missing");
    let out = armasync(&ov(&[
        "sync",
        "run",
        "example",
        "--destination",
        path(&missing),
        "--config",
        path(&env.config),
    ]));
    assert_eq!(out.status.code(), Some(5));
    assert!(
        !missing.exists(),
        "the destination is never created implicitly"
    );
    let json = armasync(&[
        "--json",
        "sync",
        "plan",
        "nope",
        "--destination",
        path(&env.dest),
        "--config",
        path(&env.config),
    ]);
    let value: serde_json::Value = serde_json::from_str(&text(&json.stdout)).unwrap();
    assert_eq!(
        (value["status"].as_str(), value["error"]["kind"].as_str()),
        (Some("error"), Some("usage"))
    );
}

#[test]
fn a_corrupted_download_is_refused_and_leaves_the_old_file_without_leftovers() {
    let server = serve();
    server.behavior.lock().unwrap().corrupt_pbo = true;
    let env = http_env(&server);
    fs::create_dir_all(env.dest.join("@example_core/addons")).unwrap();
    fs::write(env.dest.join(PBO), b"old content that must survive").unwrap();
    let (out, value) = sync_json(&env, "run", &[]);
    assert_eq!(out.status.code(), Some(6), "{}", text(&out.stderr));
    assert_eq!(
        (value["status"].as_str(), value["error"]["kind"].as_str()),
        (Some("error"), Some("integrity"))
    );
    assert_eq!(read(&env, PBO), b"old content that must survive");
    assert_eq!(files(&env.dest), [PBO]);
    no_temporary_files(&env);
}

#[test]
fn a_corrupted_first_download_installs_nothing() {
    let server = serve();
    server.behavior.lock().unwrap().corrupt_pbo = true;
    let env = http_env(&server);
    let out = sync(&env, "run", &[]);
    assert_eq!(out.status.code(), Some(6));
    assert!(files(&env.dest).is_empty(), "{:?}", files(&env.dest));
}

#[test]
fn http_errors_and_dropped_connections_are_network_failures_without_damage() {
    for (name, setup) in [
        (
            "404",
            Behavior {
                pbo_status: Some(404),
                ..Behavior::default()
            },
        ),
        (
            "500",
            Behavior {
                pbo_status: Some(500),
                ..Behavior::default()
            },
        ),
        (
            "401",
            Behavior {
                pbo_status: Some(401),
                ..Behavior::default()
            },
        ),
        (
            "short body",
            Behavior {
                short_pbo: true,
                ..Behavior::default()
            },
        ),
    ] {
        let server = serve();
        *server.behavior.lock().unwrap() = setup;
        let env = http_env(&server);
        fs::create_dir_all(env.dest.join("@example_core/addons")).unwrap();
        fs::write(env.dest.join(PBO), b"previous").unwrap();
        let out = sync(&env, "run", &[]);
        assert_eq!(out.status.code(), Some(4), "{name}: {}", text(&out.stderr));
        assert_eq!(read(&env, PBO), b"previous", "{name}");
        assert_eq!(files(&env.dest), [PBO], "{name}");
        no_temporary_files(&env);
    }
}

#[test]
fn an_unreachable_server_is_a_network_failure() {
    let (addr, stop) = hang_up_server();
    let env = env_for(&format!("http://{addr}/repo"), false);
    let out = sync(&env, "plan", &[]);
    stop.store(true, Ordering::SeqCst);
    let _ = TcpStream::connect(addr);
    assert_eq!(out.status.code(), Some(4), "{}", text(&out.stderr));
    assert!(files(&env.dest).is_empty());
}

#[test]
fn an_invalid_tls_certificate_is_rejected_and_nothing_is_sent_or_written() {
    let tls = serve_tls();
    let env = env_for(&format!("https://{}/repo", tls.addr), false);
    let out = sync(&env, "run", &[]);
    assert_eq!(out.status.code(), Some(4), "{}", text(&out.stderr));
    assert!(
        text(&out.stderr).contains("secure connection failed"),
        "{}",
        text(&out.stderr)
    );
    assert_eq!(
        tls.total(),
        0,
        "no request is sent over an untrusted connection"
    );
    assert!(files(&env.dest).is_empty());
}

#[test]
fn credentials_are_never_sent_and_the_user_is_told() {
    let server = serve();
    let env = env_for(&format!("http://{}/repo", server.addr), true);
    let out = sync(&env, "run", &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains("authentication is not supported yet"));
    for (_, headers) in server.requests.lock().unwrap().iter() {
        assert!(headers.iter().all(|(k, _)| k != "authorization"));
    }
}

fn wait_for(condition: impl Fn() -> bool, what: &str) {
    let started = Instant::now();
    while !condition() {
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "timed out waiting for {what}"
        );
        thread::sleep(Duration::from_millis(20));
    }
}

fn spawn_run(env: &Env) -> Child {
    armasync_command(&ov(&[
        "sync",
        "run",
        "example",
        "--destination",
        path(&env.dest),
        "--config",
        path(&env.config),
    ]))
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .spawn()
    .unwrap()
}

#[test]
fn a_second_run_is_refused_and_a_killed_run_leaves_the_old_file_and_is_recovered_by_the_next() {
    let slow = serve();
    slow.behavior.lock().unwrap().slow_pbo = true;
    let env = http_env(&slow);
    fs::create_dir_all(env.dest.join("@example_core/addons")).unwrap();
    fs::write(env.dest.join(PBO), b"the old version").unwrap();
    // run 1 starts transferring (the server stalls after 1000 bytes)
    let mut first = spawn_run(&env);
    wait_for(
        || slow.requested("example_core.pbo") > 0,
        "the transfer to start",
    );
    thread::sleep(Duration::from_millis(500));
    // run 2 on the same destination is refused at once with a configuration error
    let second = sync(&env, "run", &[]);
    assert_eq!(second.status.code(), Some(3), "{}", text(&second.stderr));
    assert!(
        text(&second.stderr).contains("another ArmaSync run"),
        "{}",
        text(&second.stderr)
    );
    // the first run is killed in the middle of the transfer (a crash / Ctrl+C without handler)
    first.kill().unwrap();
    first.wait().unwrap();
    assert_eq!(
        read(&env, PBO),
        b"the old version",
        "an interrupted transfer never touches the target"
    );
    assert!(
        files(&env.dest).iter().any(|f| f.contains(".armasync-")),
        "the staging leftover of the killed run exists"
    );
    // the next run (against a healthy server) recovers: lock free, leftover removed, files installed
    let healthy = serve();
    let env2 = env_for(&format!("http://{}/repo", healthy.addr), false);
    let recovery = armasync(&ov(&[
        "sync",
        "run",
        "example",
        "--destination",
        path(&env.dest),
        "--config",
        path(&env2.config),
    ]));
    assert_eq!(
        recovery.status.code(),
        Some(0),
        "{}",
        text(&recovery.stderr)
    );
    assert_eq!(read(&env, PBO), pbo());
    assert_eq!(read(&env, BIKEY), bikey());
    no_temporary_files(&env);
}
