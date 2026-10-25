# Handoff MIG-042, Schritt 3a (Rückleseprüfung vor Veröffentlichung)

- Status und Eigentümer: integriert (Fast-Forward nach `development`), Claude (Hauptbearbeiter und Integrator). Schritt 3a umgesetzt; Schritte 3b, 4 und das Legacy-Schreibformat offen.
- Datum mit Zeitzone: 2026-10-25, Europe/Berlin.
- Basis- und letzter Codecommit: Basis `d0417c1` (`origin/development`). Branch `claude/mig-042-readback`; Commits siehe `git log`.
- Branch / eigener Worktree: `claude/mig-042-readback` im Hauptcheckout.
- Fertiges Verhalten und Änderungen:
  - `revision::publish` liest jede Datei der Revision über `RemoteTarget::fetch` zurück (Größenlimit = angekündigte Größe) und prüft Größe und SHA-256, bevor `manifest.json` und `current.json` geschrieben werden. Gilt auch für vollständig erkannte Revisionen.
  - `manifest.json` wird nach dem Schreiben per GET zurückgelesen und verglichen.
  - Fehlerarten: fehlende Datei, Größenabweichung, Hashfehler → `Integrity` (Pfad im Text); Lesefehler → Transportfehler; Abbruch → `Cancelled`. `current.json` bleibt jeweils unverändert.
  - ADR-0012 Nachtrag 3a.
- Getestet und Ergebnis: siehe Abschnitt „Abschluss“.
  - `publish_readback` (neu): 6 Loopback-Tests (verfälschte Datei, verkürzte Datei, fehlende Datei, Lesefehler, Abbruch während der Rückleseprüfung, beschädigte Datei einer vollständigen Revision).
  - Bestehend: `publish_flow` 6/6, `revision_cases` 5/5, `upload_cases` 7/7.
- Nicht getestet / Blocker:
  - Keine Atomarität: Veränderung zwischen Prüfung und Zeiger wird erst beim nächsten Lauf erkannt (dokumentiert).
  - Keine echten Server; keine Linux-/WSL-Läufe; keine GitHub-Actions.
  - Legacy-Schreibformat (ADR-0003) weiterhin offen.
- Abgestimmte Verträge / neue ADRs: ADR-0012 Nachtrag 3a (angenommen mit diesem Schritt).
- Konfliktgefährdete Dateien / reservierter Schreibbereich: `crates/armasync-application/src/revision.rs`; `crates/armasync-adapters/tests/support/mod.rs`; neue Testdatei `publish_readback.rs`.
- Nächster konkreter Schritt: Schritt 3b (Ausschlüsse, Ereignisse, Parallelität, Wiederaufnahme) mit eigenem Vertrag; Schritt 4 (CLI/GUI) danach.
- Report: [2026-10-25-MIG-042-S3A](../changes/2026-10-25-MIG-042-S3A.md)

## Abschluss

Auf dem endgültigen Code, Windows 11:

- `cargo fmt --all -- --check`: Exitcode 0.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: Exitcode 0.
- `cargo test --workspace --locked`: Exitcode 0, 56 Suiten „ok“, 0 Fehler.
- `cargo +1.85.0 test --workspace --exclude armasync-gui --locked`: Exitcode 0, 50 Suiten „ok“, 0 Fehler.
- Markdown-Linkprüfung: 184 Dateien, 643 lokale Links, 0 defekt. `git diff --check`: Exitcode 0 (nur CRLF-Hinweise der Arbeitskopie).

Integration nach `development` per Fast-Forward nach diesen Prüfungen.
