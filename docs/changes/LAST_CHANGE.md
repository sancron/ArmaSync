# Letzte integrierte Änderung

**2026-10-19 · MIG-023 · Claude · CLI-Anzeige importierter Repositories und Profile** (zusammen mit PROC-001, der Prüfregel)

Nach `development` integriert (Merge `738f035`; vorher PROC-001-Merge `bde948f`, Basis `8e3b25d`). Der Push (Fast-Forward) und das CI-Ergebnis dieses Stands stehen in der Antwort an den Projektverantwortlichen; diese Datei behauptet kein CI-Ergebnis für ihn. **CI für den Vorgängerstand `8e3b25d`:** laut `gh run view` ([Run 39255913370](https://github.com/sancron/ArmaSync/actions/runs/39255913370)) alle vier Jobs erfolgreich (`Rust (windows-latest)`, `Rust (ubuntu-latest)`, MSRV 1.85, Fixtures), `Branch flow` übersprungen.

* **MIG-023:** `armasync repository list` und `armasync profile list` (rein lesend, `--json`, nie Zugangsdaten; beschädigte Bibliothek = Exitcode 7 ohne Änderung). Schließt den CLI-Teil des P2-Gates; GUI-Anzeige folgt mit MIG-031. [Bericht](2026-10-19-MIG-023.md), [Handoff](../handoffs/MIG-023.md). MIG-022 bleibt auf öffentliche Metadaten begrenzt.
* **PROC-001 (Prüfregel):** lokale Läufe primär unter Windows; lokale WSL-/Linux-Prüfungen bis zur Abschlussvalidierung zurückgestellt; Ubuntu-CI unverändert und getrennt dokumentiert; verankert in AGENTS.md, CLAUDE.md, [WORKFLOW](../ai/WORKFLOW.md), [CI](../migration/CI.md). [Bericht](2026-10-19-PROC-001.md).

Prüfung (Windows 11, Rust 1.99.1, am zusammengesetzten Stand `development`): fmt, clippy `-D warnings`, 181 Tests, MSRV 1.85 152 Tests, GUI-freier CLI-Baum, `git diff --check`. **Lokale WSL-/Linux-Prüfung: offen (zurückgestellt gemäß Prüfregel); keine Linux-Aussage aus älteren Läufen.**

**Offen:** **P1-GUI-Gate: NVDA, Orca und Sichtprüfung des hellen Themes sind nicht ausgeführt – das Gate bleibt offen**; Entschlüsselungsentscheidung (ADR-0003/RQ-01) und `export.a3s.autoconfig`; Schlüsselbund (ADR-0005); GitHub-Schritte (SHA-Pinning, Rulesets, Required Checks); Rechtefragen RQ-01..RQ-08 alle offen; P3 (echte Adapter, Netzwerkabruf, HTTP-Client-Wahl); macOS/BSD, Hardlinks, FTP/FTPS/SFTP/TLS-Tests.

Vorherige Integrationen: [MIG-022](2026-10-19-MIG-022.md), [PROC-001](2026-10-19-PROC-001.md), [MIG-021](2026-10-18-MIG-021.md), [MIG-020](2026-10-18-MIG-020.md), [MIG-012A](2026-10-18-MIG-012A.md), [MIG-013](2026-10-18-MIG-013.md), [MIG-012](2026-10-18-MIG-012.md), [MIG-011](2026-10-18-MIG-011.md), [MIG-010](2026-10-18-MIG-010.md), [MIG-002](2026-10-18-MIG-002.md), [MIG-003](2026-10-18-MIG-003.md), [MIG-001](2026-10-18-MIG-001.md).
