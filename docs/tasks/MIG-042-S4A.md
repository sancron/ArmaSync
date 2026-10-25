# MIG-042, Schritt 4a — Upload in der CLI (Vorschau, Bestätigung, Verdrahtung)

- Status: **integriert** (2026-10-25, Fast-Forward nach `development`; GUI 4b und Fortschritt 4c offen). Eigentümer und Integrator: Claude. Basis `75edaa1` (Stand nach MIG-042 Schritt 3b mit Option A). Branch `claude/mig-042-s4a-cli` (Hauptcheckout).
- Zerlegung von MIG-042 Schritt 4 (Verdrahtung in CLI und GUI), auf Wunsch des Projektverantwortlichen in getrennte Tasks:
  - **4a (dieser Task):** CLI `upload plan` und `upload publish --yes`; gemeinsame Verdrahtung im Workflow; Exitcodes, JSON, Zugangsdaten-Regeln, Abbruch.
  - **4b (eigener Task, offen):** GUI-Ansicht für Vorschau, Bestätigung, Ergebnis und Fehler. Nutzt dieselben Workflow-Funktionen.
  - **4c (eigener Task, offen):** Fortschritt pro Datei in CLI und GUI. Setzt einen Ereigniskanal in `revision::publish` voraus.
- Vorgaben des Projektverantwortlichen, eingehalten: beide Frontends nutzen dieselben Application-Use-Cases über `armasync-workflow::upload` (keine doppelte Geschäftslogik); Vorschau mit Ein- und Ausschlüssen samt Gründen; ausdrückliche Bestätigung (`--yes`); headless; `--no-input`-kompatibel (es wird nie gewartet); `--json`; stabile Exitcodes; Zugangsdaten nur über HTTPS; keine Persistenz; ausschließlich das neue Veröffentlichungsformat (kein `.a3s/sync`, kein `serverinfo`); Upload seriell, ohne Resume oder Parallelität; keine Behauptung atomarer Veröffentlichung oder Interoperabilität.
- Entscheidung: [ADR-0014](../adr/0014-upload-cli.md) (angenommen für 4a).
- Schreibbereich:
  - `crates/armasync-testkit/src/lib.rs`, `crates/armasync-testkit/src/webdav.rs` (Loopback-WebDAV-Server aus den Adapter-Tests hierher verschoben, damit Workflow und CLI ihn nutzen können).
  - `crates/armasync-adapters/tests/support/mod.rs` (wird zur Re-Exportdatei plus adapterspezifische Helfer), `crates/armasync-adapters/tests/*.rs` (nur die Aufrufe `trust(&server)` und der Import).
  - `crates/armasync-workflow/src/{lib.rs,upload.rs}`, `crates/armasync-workflow/tests/upload_workflow.rs`.
  - `crates/armasync-cli/src/{lib.rs,upload_cmd.rs}`, `crates/armasync-cli/tests/upload_cli.rs`.
  - `docs/adr/0014-upload-cli.md`, `docs/migration/CLI_CONTRACT.md` (Abschnitt Upload), Bericht, Handoff, Integratordokumente.
- Ziele: `upload plan` zeigt Ein- und Ausschlüsse mit Gründen und ungenutzte Regeln, ohne Schreibzugriff; `upload publish` veröffentlicht nur mit `--yes`; Adressen mit Zugangsdaten werden vor jeder Verbindung abgelehnt; das Kennwort kommt nur aus der Umgebung und erscheint nie in der Ausgabe.
- Nicht im Scope: GUI (4b); Fortschritt pro Datei (4c); Parallelität; Resume; Legacy-Schreibformat (ADR-0003); dauerhafte Zugangsdaten (ADR-0005); interaktive Rückfrage; echte Server; Linux-/WSL-Läufe; GitHub-Actions.
- Abnahmekriterien und Prüfungen (Windows): Workflow-Tests 9/9 (Vorschau, Veröffentlichung, Abbruch, Zugangsdaten-Regeln, Ausschluss-Syntax, Kennwort-Redaktion im Debug); CLI-Tests 8/8 (Zustimmung, Exitcodes 2, 3, 4, 5, 130, Adressen mit Zugangsdaten, Hilfe); bestehende Adapter-Suiten nach dem Umzug des Servers grün; fmt, Clippy `-D warnings`, Workspace-Test, MSRV 1.85 ohne GUI; Markdown-Links; `git diff --check`.
- Blocker und offene Punkte: Fortschritt pro Datei fehlt (4c); die GUI fehlt (4b); der erfolgreiche CLI-Lauf gegen einen Server wird nicht in der CLI selbst getestet, weil die CLI keine Test-Wurzeln akzeptiert (der Erfolgspfad ist im Workflow-Test und im Anwendungs- und Adapter-Test belegt).
