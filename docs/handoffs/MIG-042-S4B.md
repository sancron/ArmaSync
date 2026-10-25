# Handoff MIG-042, Schritt 4b (Upload in der GUI)

- Status und Eigentümer: integriert (Fast-Forward nach `development`, nach den Abschlussprüfungen), Claude (Hauptbearbeiter und Integrator). Schritt 4b: GUI-Ansicht. Fortschritt pro Datei (4c) ist ein eigener, offener Task.
- Datum mit Zeitzone: 2026-10-25, Europe/Berlin.
- Basis- und letzter Codecommit: Basis `52bf119` (`origin/development`). Branch `claude/mig-042-s4b-gui`; Commits siehe `git log`.
- Branch / eigener Worktree: `claude/mig-042-s4b-gui` im Hauptcheckout.
- Fertiges Verhalten und Änderungen:
  - Ansicht „Upload“ in `armasync-gui`: Formular, Vorschau (eingeschlossen/ausgeschlossen mit Grund, ungenutzte Regeln), Bestätigungs-Kontrollkästchen, Veröffentlichen, laufender Zustand mit Abbrechen, Ergebnis und Fehler.
  - Verhalten in `upload_screen.rs`, Darstellung in `upload_ui.rs`, Einbindung in `ui.rs` und `lib.rs`.
  - Gemeinsame Verdrahtung: `armasync_workflow::upload::{preview, publish}`. Keine Upload-Regeln in der GUI.
  - Veraltung: Jede Eingabeänderung nach der Vorschau sperrt die Veröffentlichung.
  - Kennwort: nur für die Sitzung; nach jeder Veröffentlichung geleert; nie ausgegeben.
  - ADR-0015 (angenommen für 4b).
- Getestet und Ergebnis:
  - `upload_screen` 8/8 (Vorschau mit Gründen ohne Schreibzugriff, Gate der Bestätigung, veraltete Vorschau, Abbruch lässt keinen Zeiger, HTTP-Zugangsdaten und Adresse mit Zugangsdaten ohne Verbindung, Statustext, Rücksetzen).
  - `upload_ui` 5/5 (AccessKit-Namen, Live-Region, Tastatur, Vorschau-Wörter, beide Themen, Kontrast der Fehlerfarbe).
  - Workspace-Test: Exitcode 0, 63 Suiten „ok“, 0 Fehler. MSRV 1.85 ohne GUI: Exitcode 0, 55 Suiten.
  - fmt Exitcode 0; Clippy `-D warnings` Exitcode 0 (nach Korrektur eines `ptr_arg` in einem Test); Markdown-Links 0 defekt; `git diff --check` Exitcode 0.
- Nicht getestet / Blocker:
  - **Fortschritt pro Datei** fehlt (4c).
  - **Kein Lesegerät** (NVDA, Narrator, Orca, JAWS) wurde ausgeführt. Die Barrierefreiheit ist nur im AccessKit-Baum belegt.
  - **Kennwort-Löschung nach Veröffentlichung** ist nur im Code belegt, nicht automatisiert getestet (Loopback nutzt HTTP; Zugangsdaten über HTTP werden abgelehnt; keine Test-Wurzel).
  - **Keine Sichtprüfung** des Erscheinungsbilds in beiden Themen; geprüft sind Labels, Baum und Kontrast.
  - Keine echten Server; keine Interoperabilitätsaussage; keine atomare Veröffentlichung behauptet.
  - Keine Linux-/WSL-Läufe; keine GitHub-Actions.
- Abgestimmte Verträge / neue ADRs: ADR-0015 (angenommen für 4b). Keine Änderung an ADR-0014 oder am CLI-Vertrag.
- Konfliktgefährdete Dateien / reservierter Schreibbereich: `crates/armasync-gui/src/{ui.rs,lib.rs,upload_screen.rs,upload_ui.rs}`; `crates/armasync-gui/tests/{upload_screen.rs,upload_ui.rs}`. Keine Änderung an `armasync-workflow`, `armasync-cli`, `armasync-application`, `armasync-core` oder am Ereigniskanal.
- Nächster konkreter Schritt:
  1. Task 4c: Fortschritt pro Datei in CLI und GUI. Setzt einen Ereigniskanal in `revision::publish` voraus. Vor dem Start mit dem Projektverantwortlichen den Kanal abstimmen, weil er den Workflow-Ereigniskanal berührt.
  2. Bei der Gelegenheit, mit einem Lesegerät (NVDA oder Narrator) die Ansicht prüfen, sobald ein Mensch dafür verfügbar ist. Bis dahin nicht als geprüft behaupten.
- Report: [2026-10-25-MIG-042-S4B](../changes/2026-10-25-MIG-042-S4B.md)

## Abschluss

Auf dem endgültigen Code, Windows 11:

- `cargo fmt --all -- --check`: Exitcode 0.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: Exitcode 0.
- `cargo test --workspace --locked`: Exitcode 0, 63 Suiten „ok“, 0 Fehler.
- `cargo +1.85.0 test --workspace --exclude armasync-gui --locked`: Exitcode 0, 55 Suiten „ok“, 0 Fehler.
- Markdown-Linkprüfung: 195 Dateien, 698 lokale Links, 0 defekt. `git diff --check`: Exitcode 0.

Integration nach `development` per Fast-Forward nach diesen Prüfungen.
