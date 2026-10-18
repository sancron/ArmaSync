# ArmaSync · Lynxnode.it — Changelog

## Unreleased — Rust-Migrationsvorbereitung (2026-10-18)

- Projekt analysiert und vollständiges Baseline-Dateiinventar erstellt.
- Modulare Rust-Zielarchitektur, P0–P7-Upgradeplan, GUI-/CLI-Anforderungen, Datenimport, Branding und AGPLv3-Ziel dokumentiert.
- Gemeinsame Regeln für Claude/Codex, Worktrees, Tasks, Änderungen und Übergaben angelegt.
- Branchrollen `development → testing → master` und unveränderter `legacy`-Stand festgelegt.
- MIG-001 integriert: synthetische Legacy-Formatfixtures mit Generator und unabhängigem Verifier, Class-/UID-Inventar und Exportstrategie (`fixtures/legacy-formats/`, `migration-tools/fixture-generator/`, [FORMAT_INVENTORY](docs/migration/FORMAT_INVENTORY.md)).
- MIG-002 integriert: Rechte-, Asset- und JAR-Inventur ([RIGHTS_INVENTORY](docs/migration/RIGHTS_INVENTORY.md)); Rechtefragen RQ-01..RQ-08 offen ([Tracker](docs/tasks/RIGHTS_QUESTIONS.md)), keine Rechtefreigabe.
- MIG-003 integriert: Transportmatrix ([TRANSPORT_MATRIX](docs/migration/TRANSPORT_MATRIX.md)) und Loopback-Testserver (`migration-tools/transport-testserver/`, nur HTTP/WebDAV-Ausschnitt).
- Rollenregel in `docs/ai/WORKFLOW.md`: Claude Standard-Hauptbearbeiter/Integrator, Codex Fallback.
- MIG-010 integriert: Cargo-Workspace (`armasync-core`, `-application`, `-adapters`, `-cli`), Rust 1.99.1 gepinnt (MSRV 1.85), CLI `armasync version [--json]`; Dokument [RUST_TOOLCHAIN](docs/migration/RUST_TOOLCHAIN.md).
- MIG-011 integriert: Kernverträge (sichere Pfade, Manifest, SyncPlan, Fehler, Events, Jobzustände, Ports), Use-Cases `sync::plan`/`sync::apply` und In-Memory-Adapter; [ADR-0004](docs/adr/0004-core-contracts.md). Noch keine echte Synchronisation.
- MIG-012 integriert: GUI-Prototyp `armasync-gui` (egui/eframe mit AccessKit) auf simulierten Daten; ADR-0002 „vorläufig gewählt“, Screenreader-Nachweis (NVDA/Orca) und damit das P1-GUI-Gate offen; [Prüfprotokoll](docs/migration/GUI_PROTOTYPE_EVIDENCE.md).
- MIG-013 integriert: lokal vorbereiteter CI-Workflow und Branch-Flow-Prüfung (`.github/workflows/ci.yml`, `.github/scripts/`); erster GitHub-Lauf am 2026-10-18 erfolgreich, keine Branchregeln aktiviert; [CI](docs/migration/CI.md).
- `development` am 2026-10-18 nach `origin/development` gepusht (Fast-Forward); erste GitHub-CI erfolgreich.
- MIG-012A integriert: Kontrast der Auswahlfläche im hellen Theme 3,17:1 (Ausnahme entfernt), manuelles NVDA-/Orca-Prüfprotokoll [GUI_SCREENREADER_PROTOCOL](docs/migration/GUI_SCREENREADER_PROTOCOL.md); P1-GUI-Gate weiterhin offen.
- MIG-020 integriert: neutrales Datenmodell, versionierte `library.json`/`settings.toml`, atomares Schreiben mit Backup, Datenverzeichnisse Windows/Linux/portabel, Credential-Port; [DATA_MODEL](docs/migration/DATA_MODEL.md), [ADR-0005](docs/adr/0005-credentials-and-data-paths.md). Kein Schlüsselbund-Adapter, keine Legacy-Formate.
- GUI-Leitvorgabe dokumentiert: vertraute Abläufe der Baseline, moderner, intuitiver und zugänglicher, keine Altmarken ([UI_REQUIREMENTS](docs/migration/UI_REQUIREMENTS.md)).
- `development` bis `734a20b` (inkl. MIG-012A, MIG-020) gepusht; CI grün.
- MIG-021 integriert: neutrales Importformat `armasync.import`, Rust-Importer mit Vorschau, Backup, Abbruch und Journal, `armasync import`, Offline-Exporter für unverschlüsselte Profile; verschlüsselte Repositorydateien (ADR-0003/RQ-01 offen) nicht importiert; [IMPORT_SCHEMA](docs/migration/IMPORT_SCHEMA.md).
- MIG-001: vollständiger Linux-Generatorlauf bestanden; `run.sh`/`run.ps1` bauen nur das Root-Gradle-Projekt.
- `.gitattributes`: Shell-Skripte mit LF.
- Noch keine Rust-Portierung oder Änderung am bestehenden Laufzeitcode.

Technische Berichte: [MIG-000](docs/changes/2026-10-18-MIG-000.md), [MIG-001](docs/changes/2026-10-18-MIG-001.md), [MIG-002](docs/changes/2026-10-18-MIG-002.md), [MIG-003](docs/changes/2026-10-18-MIG-003.md), [MIG-010](docs/changes/2026-10-18-MIG-010.md), [MIG-011](docs/changes/2026-10-18-MIG-011.md), [MIG-012](docs/changes/2026-10-18-MIG-012.md), [MIG-013](docs/changes/2026-10-18-MIG-013.md). Aktuelle letzte Integration: [LAST_CHANGE.md](docs/changes/LAST_CHANGE.md).

## Historischer Java-Änderungsverlauf

- [Deutscher fortlaufender Änderungsverlauf](CHANGELOGS_DE.md)
- [Continuous English change log](CHANGELOGS_EN.md)
