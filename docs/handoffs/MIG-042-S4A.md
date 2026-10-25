# Handoff MIG-042, Schritt 4a (Upload in der CLI)

- Status und Eigentümer: integriert (Fast-Forward nach `development`, sobald die Abschlussprüfungen grün sind), Claude (Hauptbearbeiter und Integrator). Schritt 4a: CLI und gemeinsame Workflow-Verdrahtung. GUI (4b) und Fortschritt pro Datei (4c) sind eigene, noch offene Tasks.
- Datum mit Zeitzone: 2026-10-25, Europe/Berlin.
- Basis- und letzter Codecommit: Basis `75edaa1` (`origin/development`). Branch `claude/mig-042-s4a-cli`; Commits siehe `git log`.
- Branch / eigener Worktree: `claude/mig-042-s4a-cli` im Hauptcheckout.
- Fertiges Verhalten und Änderungen:
  - `armasync-workflow::upload`: `preview`, `publish`, `parse_exclusions`, `check_target`; `UploadRequest` mit redigiertem `Debug`.
  - CLI: `upload plan`, `upload publish --yes`; Optionen `--source`, `--target`, `--user`, `--password-env`, `--exclude`; JSON-Hülle `upload-plan` / `upload-publish`.
  - Zugangsdaten: Kennwort nur aus der Umgebungsvariablen; Adressen mit Zugangsdaten werden abgelehnt; nur HTTPS mit Zugangsdaten.
  - Testkit: Loopback-WebDAV-Server (`armasync_testkit::webdav`); Adapter-Tests nutzen ihn unverändert über ihre Re-Exportdatei.
  - ADR-0014 (angenommen für 4a), CLI-Vertrag mit Abschnitt Upload.
- Getestet und Ergebnis:
  - Workflow `upload_workflow` 9/9 (Vorschau ohne Schreibzugriff, Veröffentlichung nur Eingeschlossenes, Abbruch lässt keinen Zeiger, Adresse mit Zugangsdaten vor jeder Verbindung abgelehnt, Query und Fragment abgelehnt, Zugangsdaten über HTTP abgelehnt, halbe Zugangsdaten abgelehnt, unsicherer Ausschluss, Kennwort in Debug redigiert).
  - CLI `upload_cli` 8/8 (Zustimmung ohne `--yes` mit 3, Adresse mit Zugangsdaten mit 3, fehlendes Kennwort mit 3, unsicherer Ausschluss mit 2, fehlende Quelle mit 5, unerreichbares Ziel mit 4, Abbruch mit 130, Hilfe).
  - CLI-Prozesstest `upload_process` 5/5 gegen den Loopback-Server über anonymes HTTP: Exitcode 0 und Ausgabe bei der Veröffentlichung, veröffentlichter Zeiger mit Revision und Manifest-Hash, Dateien byte-genau, keine internen Dateien auf dem Server; zweiter Lauf ändert nichts; Vorschau schreibt nichts; Lauf ohne `--yes` mit 3 ohne Verbindung; Quelle nicht lesbar mit 5 ohne Zeiger.
  - Adapter-Suiten nach dem Umzug des Servers grün.
  - Abschlussprüfungen auf dem endgültigen Code: siehe Abschnitt „Abschluss“.
- Nicht getestet / Blocker:
  - **Fortschritt pro Datei** fehlt (4c).
  - **GUI** fehlt (4b).
  - Der HTTPS-Erfolgspfad über den Prozess fehlt, weil die CLI keine Test-Wurzeln akzeptiert; er ist im Workflow-Test belegt.
  - Keine echten Server; keine Interoperabilitätsaussage; keine atomare Veröffentlichung behauptet.
  - Keine Linux-/WSL-Läufe; keine GitHub-Actions.
- Abgestimmte Verträge / neue ADRs: ADR-0014 (angenommen für 4a).
- Konfliktgefährdete Dateien / reservierter Schreibbereich: `crates/armasync-workflow/src/{lib.rs,upload.rs}`; `crates/armasync-cli/src/{lib.rs,upload_cmd.rs}`; `crates/armasync-testkit/src/{lib.rs,webdav.rs}`; `crates/armasync-adapters/tests/support/mod.rs` und die Adapter-Testdateien (nur `trust`-Aufrufe und Import); `docs/migration/CLI_CONTRACT.md`.
- Nächster konkreter Schritt:
  1. Task 4b: GUI-Ansicht für Vorschau, Bestätigung, Ergebnis und Fehler (eigene Taskdatei; nutzt `armasync_workflow::upload`).
  2. Task 4c: Fortschritt pro Datei (Ereigniskanal in `revision::publish`, dann CLI und GUI).
- Report: [2026-10-25-MIG-042-S4A](../changes/2026-10-25-MIG-042-S4A.md)

## Abschluss

Auf dem endgültigen Code, Windows 11:

- `cargo fmt --all -- --check`: Exitcode 0.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: Exitcode 0.
- `cargo test --workspace --locked`: Exitcode 0, 61 Suiten „ok“, 0 Fehler (einschließlich CLI-Prozesstest `upload_process` 5/5).
- `cargo +1.85.0 test --workspace --exclude armasync-gui --locked`: Exitcode 0, 55 Suiten „ok“, 0 Fehler.
- Markdown-Linkprüfung: 191 Dateien, 674 lokale Links, 0 defekt. `git diff --check`: Exitcode 0.

Integration nach `development` per Fast-Forward nach diesen Prüfungen.
