# Letzte integrierte Änderung

**2026-10-19 · MIG-030B · Claude · HTTP(S)-Adapter (`ureq`)**

Nach `development` integriert (Merge `7ded7b3`). **GitHub-CI ist pausiert** ([CI](../migration/CI.md)): keine CI-Aussage für diesen Stand; Freigabegrundlage sind die lokalen Windows-Prüfungen (letzte grüne CI: `15b7a32`).

* **Neu:** `HttpRepository` (`RepositorySource`): Manifest aus `.a3s/sync`/`serverinfo`, streamendes `fetch` mit Fortschritt, Abbruch und Größenwächter, TLS immer geprüft (Plattformverifier; nur ausdrücklich injizierte Wurzeln als Ausnahme für Tests), begrenzte Zeiten, Größen und Weiterleitungen. [Bericht](2026-10-19-MIG-030B.md), [Handoff](../handoffs/MIG-030B.md), [ADR-0006](../adr/0006-sync-metadata-and-http-transport.md) (Ergänzung), [ADR-0007](../adr/0007-transfer-ports.md).
* **Neue Abhängigkeit:** `ureq` 3.4.2 (rustls); Lizenzprüfung in `THIRD_PARTY_NOTICES.md` (RQ-06 teilweise, Release-BOM offen); nur Tests: `rcgen`, `rustls`. MSRV bleibt 1.85.
* **Nur gegen Loopback-/Test-TLS-Server belegt;** kein Dateisystemadapter, keine CLI-Synchronisation: es wird nichts Reales synchronisiert.

Prüfung (Windows 11, Rust 1.99.1, lokal): fmt, clippy `-D warnings`, 244 Tests, MSRV 1.85 215 Tests, GUI-freier Baum, `git diff --check`. **WSL/Linux lokal: offen (zurückgestellt).**

**Offen:** P1-GUI-Gate (NVDA/Orca/Sichtprüfung nicht ausgeführt), RQ-01..RQ-08 (RQ-06 teilweise), MIG-030C/D/E, Wiederaktivierung der GitHub-CI, echte Server/Proxy/TLS-Inspektion ungeprüft, Resume/Authentifizierung.

Vorherige Integrationen: [MIG-030P](2026-10-19-MIG-030P.md), [PROC-002](2026-10-19-PROC-002.md), [MIG-030A](2026-10-19-MIG-030A.md), [MIG-023](2026-10-19-MIG-023.md), [MIG-030-Vorprüfung](2026-10-19-MIG-030-preliminary.md), [MIG-022](2026-10-19-MIG-022.md), [PROC-001](2026-10-19-PROC-001.md), [MIG-021](2026-10-18-MIG-021.md), [MIG-020](2026-10-18-MIG-020.md), [MIG-012A](2026-10-18-MIG-012A.md), [MIG-013](2026-10-18-MIG-013.md), [MIG-012](2026-10-18-MIG-012.md), [MIG-011](2026-10-18-MIG-011.md), [MIG-010](2026-10-18-MIG-010.md), [MIG-002](2026-10-18-MIG-002.md), [MIG-003](2026-10-18-MIG-003.md), [MIG-001](2026-10-18-MIG-001.md).
