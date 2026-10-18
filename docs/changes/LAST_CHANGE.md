# Letzte integrierte Änderung

**2026-10-18 · MIG-020 und MIG-012A · Claude · Datenmodell/Stores/Pfade/Credentials, Kontrast der hellen Auswahlfläche**

Lokal nach `development` integriert (Merge `9577b58` für MIG-012A, `83e8ebe` für MIG-020; **noch nicht gepusht**). Vorher wurde `development` @ `6b1f261` autorisiert nach `origin/development` gepusht (Fast-Forward, `master`/`testing`/`legacy` unverändert); die erste GitHub-CI war erfolgreich ([CI](../migration/CI.md)).

* **MIG-020:** neutrales Modell, versionierte Dokumente `library.json`/`settings.toml`, atomares Schreiben mit Backup, Datenverzeichnisse Windows/Linux/portabel, Credential-Port mit Sitzungsspeicher; [DATA_MODEL](../migration/DATA_MODEL.md), [ADR-0005](../adr/0005-credentials-and-data-paths.md), [Bericht](2026-10-18-MIG-020.md), [Handoff](../handoffs/MIG-020.md). Kein Schlüsselbund-Adapter; keine Legacy-Formate.
* **MIG-012A:** Auswahlfläche im hellen Theme 3,17:1 (vorher 1,56:1), Ausnahme entfernt; manuelles Protokoll [GUI_SCREENREADER_PROTOCOL](../migration/GUI_SCREENREADER_PROTOCOL.md); [Bericht](2026-10-18-MIG-012A.md), [Handoff](../handoffs/MIG-012A.md).
* **GUI-Leitvorgabe** (vertraute Abläufe, moderner und zugänglicher, keine Altmarken) in [UI_REQUIREMENTS](../migration/UI_REQUIREMENTS.md), `AGENTS.md` und der Roadmap festgehalten. [Aktueller Stand](../migration/STATUS.md).

Prüfung am zusammengesetzten Stand: fmt, clippy `-D warnings`, 115 Tests unter Windows 11 und Linux (WSL2), MSRV 1.85 (86 Tests ohne GUI-Crate), GUI-freier CLI-Baum, Branch-Policy-Tests, `git diff --check`.

**Offen:** NVDA/Orca-Nachweis (**P1-GUI-Gate bleibt offen**), Sichtprüfung des hellen Themes, Schlüsselbund-Adapter (ADR-0005), MIG-021/022, GitHub-Schritte (SHA-Pinning, Rulesets, Required Checks), Rechtefragen RQ-01..RQ-08, echte Adapter (P3), macOS/BSD, Hardlinks, FTP/FTPS/SFTP/TLS-Tests.

Vorherige Integrationen am selben Tag: [MIG-013](2026-10-18-MIG-013.md), [MIG-012](2026-10-18-MIG-012.md), [MIG-011](2026-10-18-MIG-011.md), [MIG-010](2026-10-18-MIG-010.md), [MIG-002](2026-10-18-MIG-002.md), [MIG-003](2026-10-18-MIG-003.md), [MIG-001](2026-10-18-MIG-001.md).
