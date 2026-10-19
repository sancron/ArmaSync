# Handoff MIG-030C

- Status: reviewbereit; Claude; 2026-10-19; Basis `25138d7`. Bericht: [MIG-030C](../changes/2026-10-19-MIG-030C.md).
- Windows: fmt, clippy, 265 Tests, MSRV 236, GUI-freier Baum. **Offen:** WSL/Linux lokal (inkl. der Unix-only-Tests); GitHub-CI pausiert.
- Nächster Schritt: MIG-030D (CLI `sync plan`/`sync run`, `plan_options` erzwingen, Ende-zu-Ende mit `HttpRepository` + `LocalDirectory` gegen einen Loopback-Server, JSON/`--no-input`/Abbruch). **Nicht begonnen.**
- Offen: P1-GUI-Gate, RQ-01..RQ-08.
