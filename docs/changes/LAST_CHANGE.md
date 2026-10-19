# Letzte integrierte Änderung

**2026-10-19 · MIG-030P · Claude · Vertrag für Streaming, Staging und Schutzpfade** (nach PROC-002, GitHub-CI pausiert)

Nach `development` integriert (Merge `b040c02`). **GitHub-CI ist pausiert** ([CI](../migration/CI.md)): es gibt für diesen Stand keine CI-Aussage; Freigabegrundlage sind die lokalen Windows-Prüfungen. Die letzte grüne CI gehört zu `15b7a32`.

* **Neu:** Ports `RepositorySource::fetch` (Chunk-Streaming, Fortschritt, Abbruch, Größenwächter), `LocalStore::begin`/`StagedFile::commit` (Staging, atomare Übernahme), `LocalStore::list(algorithm, cancel)`, `Hasher::digest`, `PlanOptions::protected_paths`; `apply` streamt und verifiziert beim Empfang; SHA-1 nur als Altprotokoll-Integrität. [ADR-0007](../adr/0007-transfer-ports.md), [Bericht](2026-10-19-MIG-030P.md), [Handoff](../handoffs/MIG-030P.md).
* **Nur gegen die In-Memory-Referenz belegt:** HTTP- und Dateisystemadapter fehlen (MIG-030B/C); es wird nichts Reales synchronisiert.

Prüfung (Windows 11, Rust 1.99.1, lokal): fmt, clippy `-D warnings`, 225 Tests, MSRV 1.85 196 Tests (fand und behob einen 1.86-only-Fehler), GUI-freier Baum, `git diff --check`. **WSL/Linux lokal: offen (zurückgestellt).**

**Offen:** P1-GUI-Gate (NVDA/Orca/Sichtprüfung nicht ausgeführt), RQ-01..RQ-08, MIG-030B/C/D/E, Wiederaktivierung der GitHub-CI, Entschlüsselungsentscheidung, macOS.

Vorherige Integrationen: [PROC-002](2026-10-19-PROC-002.md), [MIG-030A](2026-10-19-MIG-030A.md), [MIG-023](2026-10-19-MIG-023.md), [MIG-030-Vorprüfung](2026-10-19-MIG-030-preliminary.md), [MIG-022](2026-10-19-MIG-022.md), [PROC-001](2026-10-19-PROC-001.md), [MIG-021](2026-10-18-MIG-021.md), [MIG-020](2026-10-18-MIG-020.md), [MIG-012A](2026-10-18-MIG-012A.md), [MIG-013](2026-10-18-MIG-013.md), [MIG-012](2026-10-18-MIG-012.md), [MIG-011](2026-10-18-MIG-011.md), [MIG-010](2026-10-18-MIG-010.md), [MIG-002](2026-10-18-MIG-002.md), [MIG-003](2026-10-18-MIG-003.md), [MIG-001](2026-10-18-MIG-001.md).
