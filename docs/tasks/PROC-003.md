# PROC-003 — Plattformsperre ohne Umgehung im Release-Build

- Status: reviewbereit (2026-10-19). Eigentümer und Integrator: Claude. Basis `aaaba2a9a78d496e78de50f359a807212708d713`. Branch `claude/proc-003-platform-gate`.
- Anlass: Prüfauftrag zu `--allow-unreleased-platform`. Befund: Die Option war in **jedem** Build vorhanden, auch im Release; ein Nutzer der normalen Binärdatei konnte damit die Linux-Synchronisation trotz RB-01 starten.
- Schreibbereich: `crates/armasync-cli/{Cargo.toml,src/lib.rs,tests/**}`, `crates/armasync-workflow/src/lib.rs` (Kommentar), Dokumente, Task/Bericht/Handoff.
- Ergebnis: [Bericht](../changes/2026-10-19-PROC-003.md).
