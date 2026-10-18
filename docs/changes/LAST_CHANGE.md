# Letzte integrierte Änderung

**2026-10-18 · MIG-010 · Claude · Cargo-Workspace, Toolchain/MSRV und CLI-Basis**

Lokal nach `development` integriert (Merge `86203b7`, nicht gepusht). Neu: `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` (Rust 1.99.1, MSRV 1.85), `crates/armasync-{core,application,adapters,cli}`, [RUST_TOOLCHAIN](../migration/RUST_TOOLCHAIN.md). Die CLI kann nur `version`; keine Synchronisationsfunktion. [Bericht](2026-10-18-MIG-010.md), [Handoff](../handoffs/MIG-010.md), [aktueller Stand](../migration/STATUS.md).

Zuvor am selben Tag: MIG-001-Nachtrag (vollständiger Linux-Generatorlauf bestanden, Merge `b29787f`), MIG-002/MIG-003 ([Bericht MIG-002](2026-10-18-MIG-002.md), [MIG-003](2026-10-18-MIG-003.md)).

Prüfung am zusammengesetzten Stand (Windows 11): `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace --locked` (13 Tests), MIG-001 `run.ps1` und `verify.py` (0 Probleme), MIG-003-Selbsttest (31 Checks), `git diff --check`. Linux (WSL2): Cargo-Checks und Tests im LF-Clone des Task-Branches.

**Offen:** Rechtefragen RQ-01..RQ-08 (alle ungeklärt); macOS/BSD, Hardlinks; FTP/FTPS/SFTP/TLS-Tests; CI (MIG-013); `cargo audit/deny` nicht ausgeführt. Nächste Arbeit: MIG-011.
