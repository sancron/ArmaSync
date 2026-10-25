# MIG-030B — HTTP(S)-Adapter (`ureq`)

- Status: **integriert** (2026-10-19; siehe [Board](README.md)). Eigentümer und Integrator: Claude.
- Basis: `development` @ `0037b61ca5e94e26e52007e0a51f3c7e1a259192` (GitHub-CI pausiert). Branch/Worktree `claude/mig-030b-http`, `../ArmaSync-claude-mig-030b`.
- Verträge: [ADR-0006](../adr/0006-sync-metadata-and-http-transport.md) (inkl. Ergänzung aus 030B), [ADR-0007](../adr/0007-transfer-ports.md), [SYNC_METADATA](../migration/SYNC_METADATA.md).
- Schreibbereich: `crates/armasync-adapters/src/http/**`, `crates/armasync-adapters/tests/http_adapter.rs`, `crates/armasync-adapters/Cargo.toml`, `crates/armasync-adapters/src/lib.rs` (eine Zeile), Root-`Cargo.lock`, `THIRD_PARTY_NOTICES.md`, ADR-0006 (Ergänzung), RIGHTS_QUESTIONS (RQ-06-Zeile), Task/Bericht/Handoff.
- Ziel: `HttpRepository` implementiert `RepositorySource` (Manifest aus `.a3s/sync`/`serverinfo`, Streaming-`fetch`, `read` für kleine Dateien): TLS immer geprüft, begrenzte Zeit und Größe, Weiterleitungsregeln, Fehlerzuordnung, Tests gegen lokale Loopback-Server (HTTP und TLS).
- Nicht im Scope: Resume/Range, Authentifizierung, explizite Proxy-Einstellung, Parallelität, ZIP, CLI, Dateisystem.
- Ergebnis: [Bericht](../changes/2026-10-19-MIG-030B.md), [Handoff](../handoffs/MIG-030B.md).
