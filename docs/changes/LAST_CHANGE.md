# Letzte integrierte Änderung

**2026-10-18 · MIG-011 · Claude · Kernverträge und In-Memory-Use-Case**

Lokal nach `development` integriert (Merge `03d9d3c`, nicht gepusht). Neu: Verträge in `armasync-core` (RelativePath, Manifest, SyncPlan, Fehler, Events, JobState, Ports), `sync::plan`/`sync::apply` in `armasync-application`, SHA-256 und In-Memory-Adapter in `armasync-adapters`, [ADR-0004](../adr/0004-core-contracts.md). Die CLI ist unverändert (nur `version`); es gibt noch keinen echten Dateisystem- oder Netzwerkadapter. [Bericht](2026-10-18-MIG-011.md), [Handoff](../handoffs/MIG-011.md), [aktueller Stand](../migration/STATUS.md).

Prüfung am zusammengesetzten Stand (Windows 11): `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace --locked` (52 Tests), MSRV 1.85, MIG-001 `run.ps1`/`verify.py`, MIG-003-Selbsttest, `git diff --check`. Linux (WSL2): Cargo-Checks und Tests im LF-Clone des Task-Branches.

**Offen:** Rechtefragen RQ-01..RQ-08 (ungeklärt); echte Adapter und CLI-Sync (P3); CI (MIG-013); GUI-Prototyp (MIG-012, Screenreader-Nachweis); macOS/BSD, Hardlinks; `cargo audit/deny` nicht ausgeführt.

Vorherige Integrationen am selben Tag: [MIG-010](2026-10-18-MIG-010.md), [MIG-002](2026-10-18-MIG-002.md), [MIG-003](2026-10-18-MIG-003.md), [MIG-001](2026-10-18-MIG-001.md).
