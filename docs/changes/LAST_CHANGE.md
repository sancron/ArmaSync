# Letzte integrierte Änderung

**2026-10-19 · MIG-030D · Claude · CLI `sync plan` / `sync run` und Ende-zu-Ende-Prüfung**

Nach `development` integriert (Merge `0e76255`). **GitHub-CI ist pausiert** ([CI](../migration/CI.md)): keine CI-Aussage; Freigabegrundlage sind die lokalen Windows-Prüfungen (letzte grüne CI: `15b7a32`).

* **Neu:** `armasync sync plan|run <repository-id> --destination <dir>` (`--dry-run`, `--delete-extra` + `--managed-root`, `--json`, `--quiet`); Verdrahtung `HttpRepository`/`HttpSnapshot` → Application-Use-Cases → `LocalDirectory` mit exklusiver Zielsperre; Schutzpfade immer aus den Metadaten. [Bericht](2026-10-19-MIG-030D.md), [Handoff](../handoffs/MIG-030D.md), [CLI_CONTRACT](../migration/CLI_CONTRACT.md).
* **Stand der Synchronisation:** Ende-zu-Ende **unter Windows gegen einen lokalen Loopback-Server** belegt (echte Binärdatei, echter HTTP-Adapter, echtes Dateisystem, MIG-001-Metadaten): Erfolg, Wiederholung ohne Transfer, Schutzpfade, falscher Hash, HTTP-Fehler, abgebrochene Verbindung, ungültiges TLS-Zertifikat, zweiter Lauf, hart beendeter Lauf mit Wiederherstellung. **Nicht** belegt: echte Server, Linux, Ctrl+C-Handler, Authentifizierung, Resume, ZIP, GUI.

Prüfung (Windows 11, Rust 1.99.1, lokal): fmt, clippy `-D warnings`, 288 Tests, MSRV 1.85 259 Tests, GUI-freier CLI-Baum, `git diff --check`. **WSL/Linux lokal: offen (zurückgestellt).**

**Offen:** P1-GUI-Gate (NVDA/Orca/Sichtprüfung nicht ausgeführt), RQ-01..RQ-08, **Unix-Pfadgrenzen-Race (Release-Blocker für Linux-Aussage)**, MIG-030E/031/032, Authentifizierung, Ctrl+C, Wiederaktivierung der GitHub-CI, Linux-Abschlussvalidierung.

Vorherige Integrationen: [MIG-030H](2026-10-19-MIG-030H.md), [MIG-030C](2026-10-19-MIG-030C.md), [MIG-030B](2026-10-19-MIG-030B.md), [MIG-030P](2026-10-19-MIG-030P.md), [PROC-002](2026-10-19-PROC-002.md), [MIG-030A](2026-10-19-MIG-030A.md), [MIG-023](2026-10-19-MIG-023.md), [MIG-030-Vorprüfung](2026-10-19-MIG-030-preliminary.md), [MIG-022](2026-10-19-MIG-022.md), [PROC-001](2026-10-19-PROC-001.md), [MIG-021](2026-10-18-MIG-021.md), [MIG-020](2026-10-18-MIG-020.md), [MIG-012A](2026-10-18-MIG-012A.md), [MIG-013](2026-10-18-MIG-013.md), [MIG-012](2026-10-18-MIG-012.md), [MIG-011](2026-10-18-MIG-011.md), [MIG-010](2026-10-18-MIG-010.md), [MIG-002](2026-10-18-MIG-002.md), [MIG-003](2026-10-18-MIG-003.md), [MIG-001](2026-10-18-MIG-001.md).
