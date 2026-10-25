# MIG-040A — Planung und Vertrag für FTP/FTPS, SFTP und WebDAV (P4)

- Status: **integriert** (2026-10-25, Fast-Forward nach `development`; siehe [Board](README.md)). Eigentümer und Integrator: Claude. Basis-Branch `development`, Basis-SHA `145f121` (`145f121` = Stand nach MIG-032R2). Eigener Branch `claude/mig-040a-transport-plan` (Hauptcheckout, kein paralleler Worktree nötig).
- Phase / Voraussetzungen / relevante ADRs: P4; Voraussetzungen MIG-003 (Transportmatrix, Loopback-Harness), MIG-030B (HTTP-Adapter), ADR-0005, ADR-0006, ADR-0007, ADR-0009, RQ-06. Entstandene ADR: [ADR-0010](../adr/0010-ftp-sftp-webdav-contract.md) (Status vorgeschlagen).
- Schreibbereich: `docs/adr/0010-ftp-sftp-webdav-contract.md`, `docs/tasks/MIG-040A.md`, `docs/changes/2026-10-25-MIG-040A.md`, `docs/handoffs/MIG-040A.md`. Keine Code-, Cargo- oder Lockdateien.
- Integrator-exklusiv (durch den Integrator bei Übernahme): `STATUS.md`, `LAST_CHANGE.md`, `docs/tasks/README.md`, `CHANGELOG.md`, `ROADMAP.md`, `docs/adr/README.md`.
- Ziele: Reihenfolge und Schnitt von MIG-040 in Folgetasks festlegen; Parität, Sicherheit und Interoperabilität je Protokoll bewerten; Auth-Arten, TLS-/Host-Key-Prüfung, Fehlerfälle und lokale Testserver beschreiben; Bibliotheken mit Version, MSRV, Lizenz, Plattform und Wartung erfassen; offene Entscheidungen nennen.
- Nicht im Scope: jede Protokollimplementierung, neue Abhängigkeiten in `Cargo.toml`, Testserver-Code, persistente Credentials oder Host-Key-Speicher, echte Server oder Zugangsdaten, Linux-/WSL-Prüfungen, GitHub-Actions.
- Gemeinsame Verträge: bestehende Ports nach ADR-0007; keine Änderung am Core.
- Abnahmekriterien und geplante Checks:
  - ADR-0010 enthält Priorisierung, gemeinsamen Vertrag, Auth, TLS/Host-Key, Fehlerliste, Testserver-Vorschlag und offene Entscheidungen.
  - Bibliotheksangaben stammen aus der crates.io-API (am 2026-10-25) und sind als solche gekennzeichnet.
  - Markdown-Links prüfen (PowerShell, Windows), `git diff --check`.
  - Keine Behauptung eines Builds oder einer Protokollprüfung.
- Ergebnis: siehe [Bericht](../changes/2026-10-25-MIG-040A.md) und [Handoff](../handoffs/MIG-040A.md).
- Blocker und nächste sichere Arbeit: Blockiert ist die Protokollimplementierung durch offene Entscheidungen (MSRV, SFTP-Bibliothek, Plaintext-FTP, WebDAV über `http`, Host-Key-Speicher, Schlüsselauth, Testwerkzeuge). Sichere Arbeit ohne Entscheidung: MIG-040B (WebDAV) nach Bestätigung von ADR-0010 starten, sofern Entscheidung 4 mit „nein“ beantwortet wird.
