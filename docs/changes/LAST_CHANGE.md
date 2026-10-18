# Letzte integrierte Änderung

**2026-10-18 · MIG-021 · Claude · Offline-Export und neutraler Import lokaler Altdaten**

Lokal nach `development` integriert (Merge `814720a`). Davor war `development` bis `734a20b` (MIG-012A, MIG-020) gepusht; die zugehörige CI war grün.

* **Neu:** neutrales Importformat `armasync.import` ([IMPORT_SCHEMA](../migration/IMPORT_SCHEMA.md)), Rust-Importer (Vorschau ohne Schreibzugriff, `--apply` mit Vorab-Sicherung, atomarem Commit, Abbruch, Journal ohne Geheimnisse, deterministische IDs, Konfliktrichtlinien, Wiederholung ohne Duplikate), CLI `armasync import`, Offline-Exporter `migration-tools/legacy-export/export.py` für unverschlüsselte Profile, Golden-Dateien `fixtures/neutral-import/`, zusätzlicher CI-Schritt im Job `fixtures`. [Bericht](2026-10-18-MIG-021.md), [Handoff](../handoffs/MIG-021.md).
* **Nicht enthalten:** Entschlüsselung lokaler `.a3s.repository`-Dateien (ADR-0003 und RQ-01 offen; der Export liefert deshalb Profile ohne Repository-Definitionen), Profilinhalte, Einstellungen, Favoritenserver, Keyring/persistente Credentials. MSRV bleibt 1.85.

Prüfung am zusammengesetzten Stand: fmt, clippy `-D warnings`, 141 Tests unter Windows 11 und Linux (WSL2), MSRV 1.85 (112 Tests ohne GUI-Crate), GUI-freier CLI-Baum, Branch-Policy-Tests, 9 Exporter-Tests, Fixture-Verifier (0 Probleme), `git diff --check`.

**Offen:** NVDA/Orca-Nachweis (**P1-GUI-Gate bleibt offen**, Protokoll [GUI_SCREENREADER_PROTOCOL](../migration/GUI_SCREENREADER_PROTOCOL.md)), Sichtprüfung des hellen Themes, MIG-022, Entschlüsselungsentscheidung, Schlüsselbund (ADR-0005, erst bei konkretem Bedarf), GitHub-Schritte (SHA-Pinning, Rulesets, Required Checks), Rechtefragen RQ-01..RQ-08, echte Adapter (P3), macOS/BSD, Hardlinks, FTP/FTPS/SFTP/TLS-Tests.

Vorherige Integrationen am selben Tag: [MIG-020](2026-10-18-MIG-020.md), [MIG-012A](2026-10-18-MIG-012A.md), [MIG-013](2026-10-18-MIG-013.md), [MIG-012](2026-10-18-MIG-012.md), [MIG-011](2026-10-18-MIG-011.md), [MIG-010](2026-10-18-MIG-010.md), [MIG-002](2026-10-18-MIG-002.md), [MIG-003](2026-10-18-MIG-003.md), [MIG-001](2026-10-18-MIG-001.md).
