# Letzte integrierte Änderung

**2026-10-18 · MIG-013 · Claude · CI-Workflow (lokal vorbereitet)**

Lokal nach `development` integriert (Merge `9f21c21`, nicht gepusht). Neu: `.github/workflows/ci.yml`, `.github/scripts/branch-policy.sh` (mit Tabellentest), `.github/scripts/cli-gui-free.sh`, [CI](../migration/CI.md). Der Workflow ist **nicht aktiviert und hat auf GitHub nie gelaufen**; es wurden keine Rulesets, Required Checks, Default-Branch- oder Release-Einstellungen geändert, die WinGet-Automatik ist unverändert. [Bericht](2026-10-18-MIG-013.md), [Handoff](../handoffs/MIG-013.md), [aktueller Stand](../migration/STATUS.md).

Prüfung: alle Job-Befehle lokal unter Linux (WSL2) und teilweise unter Windows 11 ausgeführt (Policy 18/18, fmt, clippy, 81 Tests, GUI-freier CLI-Baum, MSRV 1.85, Verifier, Testserver-Selbsttest); YAML-Struktur mit PyYAML, kein `actionlint`.

**Offen:** NVDA/Orca-Nachweis (P1-GUI-Gate), GitHub-Schritte (Push, erster Lauf, SHA-Pinning, Rulesets), Rechtefragen RQ-01..RQ-08, echte Adapter (P3), macOS/BSD, Hardlinks, FTP/FTPS/SFTP/TLS-Tests.

Vorherige Integrationen am selben Tag: [MIG-012](2026-10-18-MIG-012.md), [MIG-011](2026-10-18-MIG-011.md), [MIG-010](2026-10-18-MIG-010.md), [MIG-002](2026-10-18-MIG-002.md), [MIG-003](2026-10-18-MIG-003.md), [MIG-001](2026-10-18-MIG-001.md).
