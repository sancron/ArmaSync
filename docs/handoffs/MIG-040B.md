# Handoff MIG-040B

- Status und Eigentümer: reviewbereit, Claude (Integrator). WebDAV-Client mit OPTIONS, PROPFIND, MKCOL, PUT und DELETE über `reqwest` (Option b, ADR-0010).
- Datum mit Zeitzone: 2026-10-25, Europe/Berlin.
- Basis- und letzter Codecommit: Basis `8158fb9` (`development`). Branch `claude/mig-040b-reqwest`; Commits siehe `git log`.
- Branch / eigener Worktree: `claude/mig-040b-reqwest` im Hauptcheckout.
- Fertiges Verhalten und Änderungen:
  - `armasync_adapters::webdav::WebDavClient` mit `capabilities()`, `exists()`, `make_collection()`, `put(path, &mut File)` und `delete()`.
  - `BasicCredentials` mit redigiertem `Debug`; Basic nur über `https://`, nur an die Herkunft der Basis-Adresse, bei jeder Anfrage und jedem Redirect geprüft.
  - Client mit `Policy::none`; eigene Redirect-Schleife: `OPTIONS` und `PROPFIND` folgen gleicher Herkunft, schreibende Methoden nicht.
  - Zertifikatsprüfung über den Plattformspeicher; Test-Wurzeln nur per `tls_certs_only` für diesen Client.
  - Änderungen: `Cargo.toml`/`Cargo.lock` (`reqwest` 0.13.5), `http/mod.rs` (drei `pub(crate)`), `lib.rs`, `THIRD_PARTY_NOTICES.md`, ADR-0010.
- Getestet und Ergebnis (Windows 11):
  - `cargo test -p armasync-adapters --test webdav --locked`: 17/17.
  - `cargo fmt --all -- --check`: bestanden. `cargo clippy --workspace --all-targets --locked -- -D warnings`: Exitcode 0.
  - `cargo test --workspace --locked`: Exitcode 0.
  - `cargo +1.85.0 test --workspace --exclude armasync-gui --locked`: Exitcode 0, 45 Suiten ok, 0 Fehler.
  - Markdown-Linkprüfung: 175 Dateien, 601 lokale Links, 0 defekt. `git diff --check`: fehlerfrei.
- Nicht getestet / Blocker:
  - Keine echten WebDAV-Server; nur Loopback.
  - Kein Abbruch- und Timeout-Test für den Upload.
  - Ob ein CMake-Programm für `aws-lc-sys` nötig ist, nicht abschließend geprüft.
  - Zwei TLS-Stacks (`ring` für ureq, `aws-lc-rs` für reqwest) als Folgeentscheidung.
- Abgestimmte Verträge / neue ADRs: ADR-0010 Entscheidung 4 angenommen; Nachtrag Option b (`reqwest`) mit Prüfergebnissen und Umsetzungsregeln.
- Konfliktgefährdete Dateien / reservierter Schreibbereich: `crates/armasync-adapters/Cargo.toml`, `Cargo.lock` (Abhängigkeit), `THIRD_PARTY_NOTICES.md`. Keine Core-, CLI- oder GUI-Dateien.
- Nächster konkreter Schritt:
  1. MIG-042 (Build/Upload) verdrahtet den Client in den Ablauf (nicht in diesem Task).
  2. MIG-040C (FTP/FTPS) nach den Entscheidungen 1, 3 und 7 aus ADR-0010.
  3. Folgeentscheidung: Vereinheitlichung der TLS-Stacks (`ring` oder `aws-lc-rs`).
- Report: [2026-10-25-MIG-040B](../changes/2026-10-25-MIG-040B.md)
