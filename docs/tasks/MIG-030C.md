# MIG-030C — Dateisystemadapter (Scan, Staging, atomarer Austausch, Containment)

- Status: reviewbereit (2026-10-19). Eigentümer und Integrator: Claude.
- Basis: `development` @ `25138d78dec8b8230cbc1b85a452b54f400243ab` (GitHub-CI pausiert). Branch/Worktree `claude/mig-030c-localfs`, `../ArmaSync-claude-mig-030c`.
- Verträge: [ADR-0007](../adr/0007-transfer-ports.md), [ADR-0006](../adr/0006-sync-metadata-and-http-transport.md), COMPATIBILITY („Parser- und Dateisicherheit“).
- Schreibbereich: `crates/armasync-adapters/src/localfs/**`, `crates/armasync-adapters/tests/localfs.rs`, `crates/armasync-application/tests/localfs_flow.rs`, `crates/armasync-adapters/src/lib.rs` (eine Zeile), Task/Bericht/Handoff. Keine neuen Abhängigkeiten.
- Ziel: `LocalDirectory` implementiert `LocalStore`: Scan mit Hashart des Manifests (abbrechbar, keine Links), Staging neben dem Ziel, atomarer Austausch, Löschen einzelner Dateien, Containment gegen Links/Junctions, nur reguläre Dateien, reservierte Staging-Namen, Aufräumen alter Reste, Fehlermeldungen ohne absolute Pfade.
- Nicht im Scope: Dateisperre zwischen Prozessen, Freiplatzprüfung vorab, Größen-/Zeitverkürzung des Scans, Linux-/macOS-Prüfung (zurückgestellt), CLI (030D).
- Ergebnis: [Bericht](../changes/2026-10-19-MIG-030C.md), [Handoff](../handoffs/MIG-030C.md).
