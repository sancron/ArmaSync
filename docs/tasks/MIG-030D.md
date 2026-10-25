# MIG-030D — CLI `sync plan` / `sync run` (Verdrahtung und Ende-zu-Ende)

- Status: **integriert** (2026-10-19; siehe [Board](README.md)). Eigentümer und Integrator: Claude. Basis `39de92676b745002502c3a8cc9dd83f1848c860b` (GitHub-CI pausiert). Branch `claude/mig-030d-sync-cli`, Worktree `../ArmaSync-claude-mig-030d`.
- Voraussetzungen: MIG-030A/P/B/C/H integriert; [ADR-0006](../adr/0006-sync-metadata-and-http-transport.md), [ADR-0007](../adr/0007-transfer-ports.md), [LOCAL_FS_THREATS](../migration/LOCAL_FS_THREATS.md), [CLI_CONTRACT](../migration/CLI_CONTRACT.md).
- Schreibbereich: `crates/armasync-cli/src/**` und `crates/armasync-cli/tests/**` (+ Dev-Abhängigkeiten `rcgen`/`rustls` für den TLS-Test), `crates/armasync-adapters/src/http/mod.rs` (additiv: `HttpSnapshot`), `docs/migration/CLI_CONTRACT.md`, Task/Bericht/Handoff, Root-`Cargo.lock` (nur Dev-Einträge).
- Ziel: `armasync sync plan <repository-id> --destination <dir>` und `armasync sync run …` verbinden die bestehenden Application-Use-Cases (`plan`/`apply`) mit `HttpRepository` und `LocalDirectory`; `PlanOptions::protected_paths` kommt immer aus den Metadaten (`plan_options`); Löschen nur mit `--delete-extra` **und** ausdrücklichen `--managed-root`; `--dry-run`; stabile Exitcodes, Text und `--json`; GUI-freier CLI-Baum.
- Nicht im Scope: Ctrl+C-Handler (Abbruch durch Prozessende ist sicher, siehe Bericht), Authentifizierung, Resume, ZIP, Proxy-Einstellung, GUI.
- Prüfung: fmt, clippy, Workspace-/MSRV-Tests, Ende-zu-Ende gegen lokalen Loopback-Server mit den MIG-001-Metadaten (Windows); WSL/Linux zurückgestellt.
- Ergebnis: [Bericht](../changes/2026-10-19-MIG-030D.md), [Handoff](../handoffs/MIG-030D.md). Windows: 288 Tests, 14 Ende-zu-Ende-Tests; WSL/Linux zurückgestellt.
