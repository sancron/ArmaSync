# MIG-030A — Sync-Metadaten-Leser, SHA-1 und Manifest-Abbildung

- Status: in Arbeit (2026-10-19). Eigentümer und Integrator: Claude.
- Basis: `development`, Basis-SHA wird beim Anlegen des Branches im Reservierungscommit festgehalten (Ausgangspunkt: Stand nach Integration der Vorprüfung auf `4831fa1`).
- Branch/Worktree: `claude/mig-030a-sync-metadata`, `../ArmaSync-claude-mig-030a`.
- Verträge: [ADR-0006](../adr/0006-sync-metadata-and-http-transport.md), [SYNC_METADATA](../migration/SYNC_METADATA.md), [ADR-0003](../adr/0003-legacy-formats.md).
- Schreibbereich: `crates/armasync-compat/**`, `crates/armasync-core/src/hash.rs` (nur additiv: `Sha1`), `crates/armasync-adapters/src/sync_meta/**` (neu) plus additive Zeile in `adapters/src/lib.rs`, `crates/armasync-adapters/Cargo.toml` (nur `sha1`), Root-`Cargo.lock` (nur die dafür nötigen Einträge), diese Datei, Bericht, Handoff.
- Ziele: (1) `parse_sync_tree` und `parse_serverinfo` in Compat (gleiche Grenzen, Redaktion, feste Fehlertexte); (2) `HashAlgorithm::Sha1` im Core mit Dokumentation „nur Altprotokoll-Integrität“; (3) Adapter `sync_meta`: `SyncTree` + `ServerInfo` → `Manifest` mit typisierten Warnungen (unsicherer Pfad, komprimiertes Blatt, `hidden`) und SHA-1-Hasher (`Hasher`-Port); (4) Tests gegen alle MIG-001-Fixtures (`empty`, `small`, `hostile-paths`), alle ungültigen Fixtures, Grenzfälle.
- Nicht im Scope: Netzwerk, Dateisystem, Entpacken, Plan-/Apply-Änderungen, CLI, Entschlüsselung, Rechtefragen.
- Prüfung: fmt, clippy, Tests und MSRV 1.85 unter Windows; Linux/WSL zurückgestellt (offen); Ubuntu-CI getrennt.
- Resultat: offen.
