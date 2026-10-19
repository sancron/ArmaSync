# Letzte integrierte Änderung

**2026-10-19 · MIG-030C · Claude · Dateisystemadapter**

Nach `development` integriert (Merge `3fdb49a`). **GitHub-CI ist pausiert** ([CI](../migration/CI.md)): keine CI-Aussage für diesen Stand; Freigabegrundlage sind die lokalen Windows-Prüfungen (letzte grüne CI: `15b7a32`).

* **Neu:** `LocalDirectory` (`LocalStore`): Scan mit Hashart des Manifests (abbrechbar, keine Links), Staging neben dem Ziel, atomarer Austausch, Löschen regulärer Dateien, Containment gegen Links/Junctions, reservierte Staging-Namen, Aufräumen alter Reste. [Bericht](2026-10-19-MIG-030C.md), [Handoff](../handoffs/MIG-030C.md), [ADR-0007](../adr/0007-transfer-ports.md).
* **Belegt unter Windows** mit echten temporären Ordnern (Junction nach außen, exklusiv gesperrte Datei, 32-MiB-Streaming); keine neuen Abhängigkeiten.
* **Es wird weiterhin nichts Reales synchronisiert:** CLI-Use-Case und Ende-zu-Ende-Prüfung (HTTP + Dateisystem) fehlen; MIG-030D ist nicht begonnen.

Prüfung (Windows 11, Rust 1.99.1, lokal): fmt, clippy `-D warnings`, 265 Tests, MSRV 1.85 236 Tests, GUI-freier Baum, `git diff --check`. **WSL/Linux lokal: offen (zurückgestellt); die Unix-only-Tests sind nicht ausgeführt.**

**Offen:** P1-GUI-Gate (NVDA/Orca/Sichtprüfung nicht ausgeführt), RQ-01..RQ-08, MIG-030D/E, Wiederaktivierung der GitHub-CI, Netzlaufwerke/Langpfade/Antivirus/große Bäume ungeprüft, Prozesssperre fehlt.

Vorherige Integrationen: [MIG-030B](2026-10-19-MIG-030B.md), [MIG-030P](2026-10-19-MIG-030P.md), [PROC-002](2026-10-19-PROC-002.md), [MIG-030A](2026-10-19-MIG-030A.md), [MIG-023](2026-10-19-MIG-023.md), [MIG-030-Vorprüfung](2026-10-19-MIG-030-preliminary.md), [MIG-022](2026-10-19-MIG-022.md), [PROC-001](2026-10-19-PROC-001.md), [MIG-021](2026-10-18-MIG-021.md), [MIG-020](2026-10-18-MIG-020.md), [MIG-012A](2026-10-18-MIG-012A.md), [MIG-013](2026-10-18-MIG-013.md), [MIG-012](2026-10-18-MIG-012.md), [MIG-011](2026-10-18-MIG-011.md), [MIG-010](2026-10-18-MIG-010.md), [MIG-002](2026-10-18-MIG-002.md), [MIG-003](2026-10-18-MIG-003.md), [MIG-001](2026-10-18-MIG-001.md).
