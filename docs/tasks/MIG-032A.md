# MIG-032A — Abbruch per Ctrl+C und Fehlerfälle des Zielordners (Teil 1 von MIG-032)

- Status: reviewbereit (2026-10-19). Eigentümer und Integrator: Claude. Basis `034c33c` (GitHub-CI pausiert). Branch `claude/mig-032a-cancel`.
- Einordnung: P3 / MIG-032 (Fehlerfälle: Offline, Auth/TLS, fehlender Platz, Schreibschutz, geänderte Remote-Datei, Prozessabbruch, ungültige Pfade, parallele Zielzugriffe). Nicht blockiert (keine Rechtefrage berührt, Windows-zentriert). **Nicht** Teil dieses Teils: Resume mit `Range`, Authentifizierung, Freiplatzprüfung, ZIP.
- Schreibbereich: `crates/armasync-cli/{Cargo.toml,src/**,tests/**}` (Interrupt-Behandlung), `crates/armasync-workflow/tests/workflow.rs`, `crates/armasync-testkit/**` (Schalter, Beispiel), `THIRD_PARTY_NOTICES.md`, Doku (u. a. GUI-Screenreader-Protokoll Abschnitt F), Task/Bericht/Handoff.
- Ergebnis: [Bericht](../changes/2026-10-19-MIG-032A.md), [Handoff](../handoffs/MIG-032A.md).
