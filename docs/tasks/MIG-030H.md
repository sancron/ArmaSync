# MIG-030H — Hardening des Dateisystemadapters (Sperre, Verzeichnis-Pinning)

- Status: reviewbereit (2026-10-19). Eigentümer und Integrator: Claude. Basis `e0589dca9e6cddfd11bbfb7927efa27b7b6659c4` (GitHub-CI pausiert). Branch `claude/mig-030h-hardening`.
- Anlass: die zwei in MIG-030C dokumentierten Risiken (gleichzeitige Läufe; Ordner-Tausch zwischen Prüfung und Nutzung), vor der CLI-Verdrahtung (MIG-030D).
- Schreibbereich: `crates/armasync-adapters/src/localfs/mod.rs`, Tests `localfs.rs`/`localfs_hardening.rs` und `crates/armasync-application/tests/localfs_flow.rs`, `crates/armasync-adapters/Cargo.toml` + `Cargo.lock` (`fs4`), `docs/migration/LOCAL_FS_THREATS.md`, Task/Bericht/Handoff, `THIRD_PARTY_NOTICES.md`.
- Ergebnis: [Bedrohungsanalyse](../migration/LOCAL_FS_THREATS.md), [Bericht](../changes/2026-10-19-MIG-030H.md), [Handoff](../handoffs/MIG-030H.md).
