// SPDX-License-Identifier: AGPL-3.0-only
//! Behaviour tests of the synchronisation screen model (MIG-031): validation, plan, confirmation of deletions,
//! start conditions, run, cancel, failures and the platform gate. Real HTTP adapter and file system against the
//! loopback repository of `armasync-testkit`; no public server, no user data.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use armasync_adapters::LibraryStore;
use armasync_adapters::localfs::LocalDirectory;
use armasync_core::{DisplayName, ErrorKind, Library, Repository, RepositoryId, RepositoryUrl};
use armasync_gui::sync_screen::{Outcome, PlanAction, Stage, SyncScreen};
use armasync_testkit::synthetic::{Item, TestDir, TestFile, tree_file};
use armasync_testkit::{
    BIKEY, Behavior, PBO, Published, Server, ZIP, bikey, pbo, serve, serve_published, serve_tls,
};
use armasync_workflow::{PlatformStatus, platform_status_for};

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

fn env_with(url: &str) -> Env {
    let base = std::env::temp_dir().join(format!(
        "armasync-gui-sync-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    let mut root = base.clone();
    let mut attempt = 0u32;
    while fs::create_dir(&root).is_err() {
        attempt += 1;
        assert!(attempt < 1000);
        root = PathBuf::from(format!("{}-{attempt}", base.display()));
    }
    let (config, dest) = (root.join("config"), root.join("dest"));
    fs::create_dir_all(&config).unwrap();
    fs::create_dir_all(&dest).unwrap();
    let repository = Repository {
        id: RepositoryId::parse("example").unwrap(),
        name: DisplayName::parse("Example").unwrap(),
        url: RepositoryUrl::parse(url).unwrap(),
        credential: None,
    };
    let library = Library::new(vec![repository], Vec::new()).unwrap();
    LibraryStore::new(config.join("library.json"))
        .save(&library)
        .unwrap();
    Env { root, config, dest }
}

fn env_for(server: &Server) -> Env {
    env_with(&format!("http://{}/repo", server.addr))
}

fn released() -> PlatformStatus {
    platform_status_for("windows")
}

fn screen(env: &Env) -> SyncScreen {
    let mut screen = SyncScreen::new(Some(env.config.clone()), released(), || {});
    screen.destination = env.dest.to_string_lossy().into_owned();
    screen
}

fn wait_until(screen: &mut SyncScreen, what: &str, condition: impl Fn(&SyncScreen) -> bool) {
    let started = Instant::now();
    loop {
        screen.poll();
        if condition(screen) {
            return;
        }
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "timed out waiting for: {what}"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn plan(screen: &mut SyncScreen) {
    screen.request_plan();
    wait_until(screen, "the plan", |s| !s.is_busy());
}

fn run(screen: &mut SyncScreen) {
    screen.start();
    wait_until(screen, "the run to finish", |s| !s.is_busy());
}

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

#[test]
fn inputs_are_validated_next_to_their_fields_before_anything_happens() {
    let server = serve();
    let env = env_for(&server);
    let mut screen = SyncScreen::new(Some(env.config.clone()), released(), || {});
    screen.select(0);
    // no destination
    screen.request_plan();
    assert_eq!(screen.stage(), Stage::Choose);
    assert!(
        screen
            .field_errors
            .destination
            .as_deref()
            .unwrap()
            .contains("Zielordner")
    );
    // missing destination folder
    screen.destination = env.root.join("missing").to_string_lossy().into_owned();
    screen.request_plan();
    assert!(
        screen
            .field_errors
            .destination
            .as_deref()
            .unwrap()
            .contains("existiert nicht")
    );
    // deletion without folders, and with an unsafe folder
    screen.destination = env.dest.to_string_lossy().into_owned();
    screen.delete_extra = true;
    screen.request_plan();
    assert!(
        screen
            .field_errors
            .roots
            .as_deref()
            .unwrap()
            .contains("mindestens einen Ordner")
    );
    screen.managed_roots = "../outside".to_owned();
    screen.request_plan();
    assert!(
        screen
            .field_errors
            .roots
            .as_deref()
            .unwrap()
            .contains("Ungültiger Ordner")
    );
    assert_eq!(
        server.total(),
        0,
        "no network access before the inputs are valid"
    );
    // no repository chosen
    let mut none = SyncScreen::new(Some(env.config.clone()), released(), || {});
    none.select(5);
    none.destination = env.dest.to_string_lossy().into_owned();
    none.delete_extra = false;
    // a single repository is preselected; an empty library is reported
    assert!(none.selected().is_some());
    let empty = SyncScreen::new(Some(env.root.join("nowhere")), released(), || {});
    assert!(empty.repositories().is_empty());
    assert!(empty.load_error().is_none());
}

#[test]
fn a_damaged_library_is_reported_not_hidden() {
    let env = env_with("http://127.0.0.1:9/repo");
    fs::write(env.config.join("library.json"), b"{ broken").unwrap();
    let screen = SyncScreen::new(Some(env.config.clone()), released(), || {});
    assert!(screen.repositories().is_empty());
    assert!(
        screen
            .load_error()
            .unwrap()
            .contains("konnte nicht gelesen werden")
    );
}

#[test]
fn the_plan_is_shown_before_anything_changes() {
    let server = serve();
    let env = env_for(&server);
    let mut screen = screen(&env);
    assert_eq!(screen.start_blocker(), Err("Zuerst die Änderungen prüfen."));
    plan(&mut screen);
    assert_eq!(screen.stage(), Stage::PlanReady);
    let view = screen.plan().unwrap();
    assert_eq!(view.revision, "3");
    assert_eq!(
        (view.totals.downloads, view.totals.download_bytes),
        (2, 8193 + 20)
    );
    assert_eq!(view.totals.deletes, 0);
    assert_eq!(
        view.rows
            .iter()
            .map(|r| (r.action, r.path.as_str()))
            .collect::<Vec<_>>(),
        [(PlanAction::Download, PBO), (PlanAction::Download, BIKEY)]
    );
    let hint =
        format!("Komprimiert geführte Datei ohne nutzbare Archivgröße bleibt unverändert: {ZIP}");
    assert!(view.notes.contains(&hint), "{:?}", view.notes);
    assert_eq!(view.unsupported, [ZIP]);
    assert_eq!(view.protected, ["@second"]);
    assert!(
        files(&env.dest).is_empty(),
        "looking at the plan changes nothing"
    );
    assert_eq!(server.requested("example_core.pbo"), 0);
    assert!(screen.start_blocker().is_ok());
}

#[test]
fn a_run_installs_verified_files_and_reports_the_result() {
    let server = serve();
    let env = env_for(&server);
    let mut screen = screen(&env);
    plan(&mut screen);
    run(&mut screen);
    assert_eq!(screen.stage(), Stage::Finished);
    let Some(Outcome::Succeeded(report)) = screen.outcome() else {
        panic!("{:?}", screen.outcome());
    };
    assert_eq!(
        (
            report.downloaded,
            report.replaced,
            report.deleted,
            report.bytes
        ),
        (2, 0, 0, 8213)
    );
    assert_eq!(fs::read(env.dest.join(PBO)).unwrap(), pbo());
    assert_eq!(fs::read(env.dest.join(BIKEY)).unwrap(), bikey());
    assert!(
        screen
            .announcement()
            .starts_with("Abgeschlossen: 2 geladen")
    );
    // a plan applied once is gone; starting again needs a new check
    assert!(screen.start_blocker().is_err());
    screen.reset();
    plan(&mut screen);
    assert_eq!(screen.plan().unwrap().totals.unchanged, 2);
    assert!(screen.plan().unwrap().rows.is_empty());
}

#[test]
fn changing_an_input_invalidates_the_plan_and_blocks_starting() {
    let server = serve();
    let env = env_for(&server);
    let mut screen = screen(&env);
    plan(&mut screen);
    assert!(screen.plan_is_current() && screen.start_blocker().is_ok());
    let other = env.root.join("other");
    fs::create_dir_all(&other).unwrap();
    screen.destination = other.to_string_lossy().into_owned();
    screen.poll();
    assert!(!screen.plan_is_current());
    assert!(
        screen
            .start_blocker()
            .unwrap_err()
            .contains("erneut prüfen")
    );
    assert!(
        screen
            .notice()
            .unwrap()
            .contains("Vorschau gilt nicht mehr")
    );
    screen.start();
    assert_eq!(
        screen.stage(),
        Stage::PlanReady,
        "start() does nothing while the plan is stale"
    );
    assert!(files(&env.dest).is_empty() && files(&other).is_empty());
    // toggling the deletion option also invalidates the plan
    screen.destination = env.dest.to_string_lossy().into_owned();
    screen.poll();
    assert!(screen.plan_is_current());
    screen.delete_extra = true;
    assert!(!screen.plan_is_current());
}

#[test]
fn deletions_are_listed_need_confirmation_and_never_touch_protected_files() {
    let server = serve();
    let env = env_for(&server);
    fs::create_dir_all(env.dest.join("@example_core/addons")).unwrap();
    fs::write(env.dest.join("@example_core/addons/extra.pbo"), b"extra").unwrap();
    fs::write(
        env.dest.join(ZIP),
        b"local copy of a compressed repository file",
    )
    .unwrap();
    fs::create_dir_all(env.dest.join("@second")).unwrap();
    fs::write(env.dest.join("@second/mine.txt"), b"mine").unwrap();
    let mut screen = screen(&env);
    // default: nothing is ever deleted
    plan(&mut screen);
    assert_eq!(screen.plan().unwrap().totals.deletes, 0);
    assert!(screen.plan().unwrap().deletes.is_empty());
    screen.reset();
    screen.delete_extra = true;
    screen.managed_roots = "@example_core\n@second\n".to_owned();
    plan(&mut screen);
    let view = screen.plan().unwrap();
    assert_eq!(view.deletes, ["@example_core/addons/extra.pbo"]);
    assert!(view.rows.iter().any(|r| r.action == PlanAction::Delete));
    assert!(
        view.notes
            .iter()
            .any(|n| n.contains("vom Repository geschützt")),
        "{:?}",
        view.notes
    );
    // starting needs the explicit confirmation of the listed deletions
    assert_eq!(
        screen.start_blocker(),
        Err("Bitte zuerst die Löschungen bestätigen.")
    );
    screen.start();
    assert_eq!(
        screen.stage(),
        Stage::PlanReady,
        "an unconfirmed start does nothing"
    );
    assert!(env.dest.join("@example_core/addons/extra.pbo").exists());
    screen.confirm_deletes = true;
    assert!(screen.start_blocker().is_ok());
    run(&mut screen);
    let Some(Outcome::Succeeded(report)) = screen.outcome() else {
        panic!("{:?}", screen.outcome());
    };
    assert_eq!(report.deleted, 1);
    assert!(!env.dest.join("@example_core/addons/extra.pbo").exists());
    assert!(
        env.dest.join(ZIP).exists(),
        "a file the server holds compressed is kept"
    );
    assert!(
        env.dest.join("@second/mine.txt").exists(),
        "a protected folder is never cleaned"
    );
}

#[test]
fn on_a_platform_that_is_not_released_the_plan_is_visible_but_nothing_can_be_applied() {
    let server = serve();
    let env = env_for(&server);
    let mut screen = SyncScreen::new(
        Some(env.config.clone()),
        platform_status_for("linux"),
        || {},
    );
    screen.destination = env.dest.to_string_lossy().into_owned();
    assert!(!screen.platform().run_allowed());
    assert!(
        screen
            .platform()
            .message()
            .contains("noch nicht freigegeben")
    );
    plan(&mut screen);
    assert_eq!(screen.stage(), Stage::PlanReady);
    assert_eq!(screen.plan().unwrap().totals.downloads, 2);
    assert_eq!(
        screen.start_blocker(),
        Err("Auf diesem System ist das Synchronisieren noch nicht freigegeben.")
    );
    screen.start();
    assert_eq!(screen.stage(), Stage::PlanReady);
    assert!(files(&env.dest).is_empty());
    assert_eq!(server.requested("example_core.pbo"), 0);
}

#[test]
fn cancelling_a_running_transfer_keeps_existing_files_and_leaves_nothing_partial() {
    let server = serve();
    server.behavior.lock().unwrap().trickle_pbo = true;
    let env = env_for(&server);
    fs::create_dir_all(env.dest.join("@example_core/addons")).unwrap();
    fs::write(env.dest.join(PBO), b"the old version").unwrap();
    let mut screen = screen(&env);
    plan(&mut screen);
    screen.start();
    assert_eq!(screen.stage(), Stage::Running);
    // the window stays responsive: polling never blocks while the worker transfers
    let started = Instant::now();
    wait_until(&mut screen, "the transfer to start", |_| {
        server.requested("example_core.pbo") > 0
    });
    screen.cancel();
    assert!(screen.job().unwrap().cancel_requested);
    wait_until(&mut screen, "the run to stop", |s| !s.is_busy());
    assert!(started.elapsed() < Duration::from_secs(10));
    assert_eq!(screen.outcome(), Some(&Outcome::Cancelled));
    assert_eq!(fs::read(env.dest.join(PBO)).unwrap(), b"the old version");
    assert_eq!(
        files(&env.dest),
        [PBO],
        "no partial file, no temporary file"
    );
    assert_eq!(screen.announcement(), "Abgebrochen.");
}

#[test]
fn failures_are_explained_and_leave_the_destination_intact() {
    for (name, behavior, kind, expect) in [
        (
            "corrupt",
            Behavior {
                corrupt_pbo: true,
                ..Behavior::default()
            },
            ErrorKind::Integrity,
            "Prüfsumme",
        ),
        (
            "status",
            Behavior {
                pbo_status: Some(500),
                ..Behavior::default()
            },
            ErrorKind::Network,
            "Verbindung",
        ),
        (
            "short",
            Behavior {
                short_pbo: true,
                ..Behavior::default()
            },
            ErrorKind::Network,
            "Verbindung",
        ),
    ] {
        let server = serve();
        *server.behavior.lock().unwrap() = behavior;
        let env = env_for(&server);
        fs::create_dir_all(env.dest.join("@example_core/addons")).unwrap();
        fs::write(env.dest.join(PBO), b"previous").unwrap();
        let mut screen = screen(&env);
        plan(&mut screen);
        run(&mut screen);
        let Some(Outcome::Failed(item)) = screen.outcome() else {
            panic!("{name}: {:?}", screen.outcome());
        };
        assert_eq!(item.kind, kind, "{name}");
        assert!(item.hint.contains(expect), "{name}: {}", item.hint);
        assert_eq!(fs::read(env.dest.join(PBO)).unwrap(), b"previous", "{name}");
        assert_eq!(files(&env.dest), [PBO], "{name}");
    }
}

#[test]
fn an_invalid_tls_certificate_is_an_error_with_an_explanation_and_nothing_is_written() {
    let tls = serve_tls();
    let env = env_with(&format!("https://{}/repo", tls.addr));
    let mut screen = screen(&env);
    plan(&mut screen);
    let Some(Outcome::Failed(item)) = screen.outcome() else {
        panic!("{:?}", screen.outcome());
    };
    assert_eq!(item.kind, ErrorKind::Network);
    assert!(
        item.message.contains("secure connection failed"),
        "{}",
        item.message
    );
    assert!(item.hint.contains("Zertifikat"));
    assert_eq!(tls.total(), 0, "no request over an untrusted connection");
    assert!(files(&env.dest).is_empty());
}

#[test]
fn a_destination_in_use_by_another_run_is_reported_as_such() {
    let server = serve();
    let env = env_for(&server);
    let mut screen = screen(&env);
    plan(&mut screen);
    let _other_run = LocalDirectory::open_exclusive(&env.dest).unwrap();
    run(&mut screen);
    let Some(Outcome::Failed(item)) = screen.outcome() else {
        panic!("{:?}", screen.outcome());
    };
    assert_eq!(item.kind, ErrorKind::Config);
    assert!(item.hint.contains("anderen ArmaSync-Lauf"));
    assert!(files(&env.dest).is_empty());
}

#[test]
fn dropping_the_screen_while_a_transfer_runs_cancels_it_without_hanging() {
    let server = serve();
    server.behavior.lock().unwrap().trickle_pbo = true;
    let env = env_for(&server);
    let mut screen = screen(&env);
    plan(&mut screen);
    screen.start();
    wait_until(&mut screen, "the transfer to start", |_| {
        server.requested("example_core.pbo") > 0
    });
    let started = Instant::now();
    drop(screen);
    assert!(started.elapsed() < Duration::from_secs(10));
    // The worker was cancelled and joined; the old destination content is untouched.
    assert!(!env.dest.join(PBO).exists());
}

// ---- packed (ZIP) files with a usable archive size (MIG-030E, ADR-0008) ------------------------------------------
//
// Archives and metadata are generated here; no real repository content is involved.

const PACKED: &str = "@packed/packed.pbo";

fn packed_content() -> Vec<u8> {
    (0..150_000u32)
        .map(|i| (i % 239) as u8 ^ (i / 13) as u8)
        .collect()
}

fn deflated_zip(name: &str, data: &[u8]) -> Vec<u8> {
    use std::io::{Cursor, Write};
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file(
            name,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )
        .unwrap();
    writer.write_all(data).unwrap();
    writer.finish().unwrap().into_inner()
}

fn packed_repository(content: &[u8], archive: Vec<u8>) -> Published {
    use armasync_core::{HashAlgorithm, Hasher};
    let sha1 = armasync_adapters::Sha256Hasher
        .hash(HashAlgorithm::Sha1, content)
        .hex()
        .to_owned();
    let mut file = TestFile::new("packed.pbo", content.len() as i64);
    file.sha1 = sha1.as_str().into();
    file.compressed = true;
    file.compressed_size = archive.len() as i64;
    let addon = TestDir {
        name: "@packed".to_owned(),
        hidden: false,
        addon: true,
        items: vec![Item::File(file)],
    };
    let root = TestDir::new("racine", vec![Item::Dir(addon)]);
    let mut files = std::collections::BTreeMap::new();
    files.insert(format!("{PACKED}.zip"), archive);
    Published {
        sync: tree_file(&root),
        files,
        trickle: false,
        ..Published::default()
    }
}

#[test]
fn a_compressed_file_with_a_usable_archive_size_is_installed_not_listed_as_unsupported() {
    let content = packed_content();
    let archive = deflated_zip("packed.pbo", &content);
    let server = serve_published(packed_repository(&content, archive));
    let env = env_for(&server);
    let mut screen = screen(&env);

    plan(&mut screen);
    let view = screen.plan().unwrap();
    assert_eq!(
        view.rows
            .iter()
            .map(|r| (r.action, r.path.as_str()))
            .collect::<Vec<_>>(),
        [(PlanAction::Download, PACKED)]
    );
    assert!(view.unsupported.is_empty(), "{:?}", view.unsupported);
    assert!(
        !view.notes.iter().any(|n| n.contains("Komprimiert")),
        "{:?}",
        view.notes
    );
    assert_eq!(server.requested(".zip"), 0, "planning downloads nothing");

    run(&mut screen);
    assert_eq!(screen.stage(), Stage::Finished);
    assert_eq!(fs::read(env.dest.join(PACKED)).unwrap(), content);
    assert_eq!(files(&env.dest), [PACKED], "no archive is left behind");
    assert_eq!(server.requested(".zip"), 1);
}

// ---- cancelling large and packed downloads (MIG-032B) -------------------------------------------------------------
//
// The repository is synthetic and delivered slowly (`trickle`), so a cancellation arrives while the transfer runs.

const BIG: &str = "@big/big.pbo";

fn noise(len: usize, seed: u64) -> Vec<u8> {
    let mut x = seed | 1;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x as u8
        })
        .collect()
}

fn large_repository(content: &[u8]) -> Published {
    use armasync_core::{HashAlgorithm, Hasher};
    let sha1 = armasync_adapters::Sha256Hasher
        .hash(HashAlgorithm::Sha1, content)
        .hex()
        .to_owned();
    let mut file = TestFile::new("big.pbo", content.len() as i64);
    file.sha1 = sha1.as_str().into();
    let addon = TestDir {
        name: "@big".to_owned(),
        hidden: false,
        addon: true,
        items: vec![Item::File(file)],
    };
    let root = TestDir::new("racine", vec![Item::Dir(addon)]);
    let mut files = std::collections::BTreeMap::new();
    files.insert(BIG.to_owned(), content.to_vec());
    Published {
        sync: tree_file(&root),
        files,
        trickle: true,
        ..Published::default()
    }
}

#[test]
fn cancelling_a_large_download_keeps_the_existing_file_and_leaves_nothing_partial() {
    let content = noise(1024 * 1024, 11);
    let server = serve_published(large_repository(&content));
    let env = env_for(&server);
    fs::create_dir_all(env.dest.join("@big")).unwrap();
    fs::write(env.dest.join(BIG), b"the old version").unwrap();
    let mut screen = screen(&env);
    plan(&mut screen);
    screen.start();
    wait_until(&mut screen, "the transfer to start", |_| {
        server.requested("big.pbo") > 0
    });
    screen.cancel();
    wait_until(&mut screen, "the run to stop", |s| !s.is_busy());
    assert_eq!(screen.outcome(), Some(&Outcome::Cancelled));
    assert_eq!(fs::read(env.dest.join(BIG)).unwrap(), b"the old version");
    assert_eq!(
        files(&env.dest),
        [BIG],
        "no partial file, no temporary file"
    );
}

#[test]
fn cancelling_a_packed_download_leaves_no_archive_spool_or_partial_file() {
    let content = noise(1024 * 1024, 12);
    let archive = deflated_zip("packed.pbo", &content);
    let mut published = packed_repository(&content, archive);
    published.trickle = true;
    let server = serve_published(published);
    let env = env_for(&server);
    fs::create_dir_all(env.dest.join("@packed")).unwrap();
    fs::write(env.dest.join(PACKED), b"the old version").unwrap();
    let mut screen = screen(&env);
    plan(&mut screen);
    screen.start();
    wait_until(&mut screen, "the archive transfer to start", |_| {
        server.requested(".zip") > 0
    });
    screen.cancel();
    wait_until(&mut screen, "the run to stop", |s| !s.is_busy());
    assert_eq!(screen.outcome(), Some(&Outcome::Cancelled));
    assert_eq!(fs::read(env.dest.join(PACKED)).unwrap(), b"the old version");
    assert_eq!(
        files(&env.dest),
        [PACKED],
        "no archive, no spool file and no partial file"
    );
}
