# MIG-030P — Vertrag: Streaming-Übertragung, Staging, Schutzpfade (Ports)

- Status: **integriert** (2026-10-19; siehe [Board](README.md)). Eigentümer und Integrator: Claude. Der Name `MIG-030P` ("Ports") ist der nächste freie Name der Familie MIG-030; er geht MIG-030B/C voraus.
- Basis: `development` @ `89f1fd49f84b1687353914a9dbbb0bfebcb18120` (GitHub-CI pausiert, PROC-002). Branch/Worktree `claude/mig-030p-ports`, `../ArmaSync-claude-mig-030p`.
- Verträge: [ADR-0007](../adr/0007-transfer-ports.md) (gilt vor Adaptern), [ADR-0006](../adr/0006-sync-metadata-and-http-transport.md).
- Schreibbereich: `crates/armasync-core/src/{port,plan,lib}.rs`, `crates/armasync-core/tests/transfer_contract.rs`, `crates/armasync-application/src/sync.rs` und Tests, `crates/armasync-adapters/src/memory.rs`, `crates/armasync-adapters/src/sync_meta/mod.rs` (Hilfsfunktion `plan_options`) und Tests, Doku. Keine neuen Abhängigkeiten, kein Netzwerk, kein Dateisystemadapter.
- Ziel: getestete Ports für Streaming ohne vollständiges Laden, Fortschritt/Abbruch, Staging + Hashverifikation + atomare Übernahme, Fehlerfälle, Schutz von `unsupported`/`protected`, SHA-1 nur als Integrität.
- Nicht im Scope: echte Adapter (030B/C), CLI (030D), Resume, ZIP.
- Ergebnis: [Bericht](../changes/2026-10-19-MIG-030P.md), [Handoff](../handoffs/MIG-030P.md).
