# Handoff MIG-042, Schritt 4c (Fortschritt pro Datei)

- Status und Eigentümer: integriert (Fast-Forward nach `development` nach den finalen Windows-Prüfungen), Claude (Hauptbearbeiter und Integrator). Schritt 4c: Ereignisvertrag, Verdrahtung in CLI und GUI.
- Datum mit Zeitzone: 2026-10-25, Europe/Berlin.
- Basis- und letzter Codecommit: Basis `4e2e134` (`origin/development`). Branch `claude/mig-042-s4c-progress`; Commits siehe `git log`.
- Branch / eigener Worktree: `claude/mig-042-s4c-progress` im Hauptcheckout.
- Fertiges Verhalten und Änderungen:
  - Ereignistyp `UploadEvent` mit sieben stabilen Typen (`upload.started`, `upload.file_started`, `upload.file_bytes`, `upload.file_finished`, `upload.finished`, `upload.cancelled`, `upload.failed`), Schemaversion 1, Sequenz je Vorgang ab 0.
  - `UploadObserver` in `armasync-application` sorgt für Reihenfolge und genau ein Endereignis; `CountingReader` meldet die gelesenen Bytes je Datei.
  - `publish_observed` in Anwendung und Workflow; CLI und GUI nutzen denselben Kanal.
  - CLI: Textmodus mit zwei Zeilen je Datei auf stderr; JSON und `--quiet` unverändert.
  - GUI: Fortschrittsleiste, Dateizähler und aktuelle Datei; Ereignisse werden pro Bild abgeholt; Abbruch über `CancelToken`.
  - ADR-0016 (angenommen für 4c); Hinweis in ADR-0015; CLI-Vertrag mit Abschnitt Fortschritt.
- Getestet und Ergebnis:
  - Workflow `upload_progress` 5/5, CLI `upload_process` 6/6 (davon 1 neu), GUI `upload_screen` 9/9 (davon 1 neu), `upload_ui` 5/5, Core-Test des Ereignisvertrags.
  - Workspace-Test Exitcode 0 (64 Suiten), MSRV 1.85 ohne GUI Exitcode 0 (56 Suiten), fmt und Clippy `-D warnings` Exitcode 0, Markdown-Links 199 Dateien, 716 Links, 0 defekt, `git diff --check` Exitcode 0.
  - Geheimnisfreiheit: Debug-Ausgabe der Ereignisse enthält weder Kennwort, Benutzername, Adresse noch den lokalen Ordnernamen; CLI-Zeilen zeigen nur relative Pfade.
- Nicht getestet / Blocker:
  - **Kein Lesegerät** (NVDA, Narrator, Orca, JAWS) geprüft; die Fortschrittsanzeige ist nicht per Screenreader abgenommen.
  - **Keine Sichtprüfung** der Fortschrittsleiste in hellem und dunklem Thema.
  - Byte-Zählung über Lesevorgänge, keine Netzwerkmessung.
  - Keine echten WebDAV-Server; keine Interoperabilitätsaussage; keine atomare Veröffentlichung behauptet.
  - Keine Linux-/WSL-Läufe; keine GitHub-Actions.
- Abgestimmte Verträge / neue ADRs: ADR-0016 (angenommen für 4c). `OperationEvent` (Synchronisation) unverändert.
- Konfliktgefährdete Dateien / reservierter Schreibbereich: `crates/armasync-core/src/{lib.rs,upload_event.rs}`; `crates/armasync-application/src/{lib.rs,progress.rs,upload.rs,revision.rs}`; `crates/armasync-workflow/src/upload.rs`; `crates/armasync-cli/src/upload_cmd.rs`; `crates/armasync-gui/src/{upload_screen.rs,upload_ui.rs}`; zugehörige Tests; `docs/migration/CLI_CONTRACT.md`.
- Nächster konkreter Schritt:
  1. Lesegerät- und Sichtprüfung der Upload-Ansicht (NVDA oder Narrator, helles und dunkles Thema), sobald jemand dafür verfügbar ist. Bis dahin nicht als geprüft behaupten.
  2. Parallelität und Resume bleiben ausdrücklich außerhalb von MIG-042; dafür braucht es eine eigene Entscheidung.
- Report: [2026-10-25-MIG-042-S4C](../changes/2026-10-25-MIG-042-S4C.md)

## Abschluss

Auf dem endgültigen Code, Windows 11:

- `cargo fmt --all -- --check`: Exitcode 0.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: Exitcode 0.
- `cargo test --workspace --locked`: Exitcode 0, 64 Suiten „ok“, 0 Fehler.
- `cargo +1.85.0 test --workspace --exclude armasync-gui --locked`: Exitcode 0, 56 Suiten „ok“, 0 Fehler.
- Markdown-Linkprüfung: 199 Dateien, 716 lokale Links, 0 defekt. `git diff --check`: Exitcode 0.

Integration nach `development` per Fast-Forward nach diesen Prüfungen.
