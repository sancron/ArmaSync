# Handoff MIG-042, Schritt 3b (Upload-Ausschlüsse)

- Status und Eigentümer: integriert (Fast-Forward nach `development`), Claude (Hauptbearbeiter und Integrator). Feste Regeln, Ausschlüsse als validierte Pfade und Vorschau umgesetzt. **Benutzerformat für Ausschlussregeln blockiert** (Produktentscheidung, ADR-0013).
- Datum mit Zeitzone: 2026-10-25, Europe/Berlin.
- Basis- und letzter Codecommit: Basis `9d5d696` (`origin/development`). Branch `claude/mig-042-exclusions`; Commits siehe `git log`.
- Branch / eigener Worktree: `claude/mig-042-exclusions` im Hauptcheckout.
- Fertiges Verhalten und Änderungen:
  - Core `exclusion`: feste Regeln (Teildateien, interne Daten), `ExclusionRules` aus validierten Pfaden, Grund je Datei, ungenutzte Regeln.
  - Quelle: `files()` und `inventory()` rein lesend; keine Entfernung von Staging-Resten, keine Sperre, keine Link-Verfolgung.
  - Anwendung: `partition`; `upload::plan_with_rules` mit Ausschlussvorschau; `revision::publish_with_rules`.
  - ADR-0013 (Vertrag, Blockade des Benutzerformats mit Optionen A, B, C).
- Getestet und Ergebnis: siehe Abschnitt „Abschluss“.
  - Core-Modultests (`exclusion.rs`).
  - `exclusion_cases` 9/9 (Vorschau mit Gründen, Vollständigkeit, ungenutzte Regeln, Plan ohne Ordner für ausgeschlossene Ordner, nur einbezogene Dateien geschrieben, Revision und Manifest ohne ausgeschlossene Pfade, leere Revision, Pfade nicht aus der Quelle heraus, Dubletten).
  - `exclusion_flow` 4/4 (echter Ordner, Loopback-Server; Quelle unverändert nach Vorschau; Revision ohne Ausschlüsse; ungenutzte Regel).
  - Bestehende Suiten (`publish_flow` 6/6, `publish_readback` 6/6, `upload_flow` 8/8) bleiben grün.
- Nicht getestet / Blocker:
  - **Blockiert:** das Benutzerformat (Muster, Regeldatei, Oberfläche). Keine Implementierung, bis die Produktentscheidung getroffen ist.
  - Entfernen alter Staging-Reste im früheren Scan: Code belegt, kein Test mit älteren Dateien (ohne Zeitstempel-Abhängigkeit nicht machbar).
  - Keine echten Server; keine Linux-/WSL-Läufe; keine GitHub-Actions.
- Abgestimmte Verträge / neue ADRs: ADR-0013 (angenommen für feste Regeln, validierte Pfade und Vorschau; Benutzerformat offen).
- Konfliktgefährdete Dateien / reservierter Schreibbereich: `crates/armasync-core/src/{exclusion.rs,port.rs,lib.rs}`; `crates/armasync-adapters/src/localfs/{mod.rs,partial.rs}`; `crates/armasync-application/src/{exclusion.rs,upload.rs,revision.rs,lib.rs}`; Tests in den drei `tests/`-Verzeichnissen.
- Nächster konkreter Schritt:
  1. Produktentscheidung zum Benutzerformat (ADR-0013: A, B oder C).
  2. Danach Schritt 3c (Parallelität, Wiederaufnahme) oder Schritt 4 (CLI/GUI), eigener Vertrag.
- Report: [2026-10-25-MIG-042-S3B](../changes/2026-10-25-MIG-042-S3B.md)

## Abschluss

Auf dem endgültigen Code, Windows 11:

- `cargo fmt --all -- --check`: Exitcode 0.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: Exitcode 0.
- `cargo test --workspace --locked`: Exitcode 0, 58 Suiten „ok“, 0 Fehler.
- `cargo +1.85.0 test --workspace --exclude armasync-gui --locked`: Exitcode 0, 52 Suiten „ok“, 0 Fehler.
- Markdown-Linkprüfung: 187 Dateien, 657 lokale Links, 0 defekt. `git diff --check`: Exitcode 0.

Integration nach `development` per Fast-Forward nach diesen Prüfungen.
