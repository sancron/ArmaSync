# ArmaSync · Lynxnode.it — Changelog

## Unreleased — Rust-Migrationsvorbereitung (2026-10-18)

- Projekt analysiert und vollständiges Baseline-Dateiinventar erstellt.
- Modulare Rust-Zielarchitektur, P0–P7-Upgradeplan, GUI-/CLI-Anforderungen, Datenimport, Branding und AGPLv3-Ziel dokumentiert.
- Gemeinsame Regeln für Claude/Codex, Worktrees, Tasks, Änderungen und Übergaben angelegt.
- Branchrollen `development → testing → master` und unveränderter `legacy`-Stand festgelegt.
- MIG-001 integriert: synthetische Legacy-Formatfixtures mit Generator und unabhängigem Verifier, Class-/UID-Inventar und Exportstrategie (`fixtures/legacy-formats/`, `migration-tools/fixture-generator/`, [FORMAT_INVENTORY](docs/migration/FORMAT_INVENTORY.md)).
- `.gitattributes`: Shell-Skripte mit LF.
- Noch keine Rust-Portierung oder Änderung am bestehenden Laufzeitcode.

Technische Berichte: [MIG-000](docs/changes/2026-10-18-MIG-000.md), [MIG-001](docs/changes/2026-10-18-MIG-001.md). Aktuelle letzte Integration: [LAST_CHANGE.md](docs/changes/LAST_CHANGE.md).

## Historischer Java-Änderungsverlauf

- [Deutscher fortlaufender Änderungsverlauf](CHANGELOGS_DE.md)
- [Continuous English change log](CHANGELOGS_EN.md)
