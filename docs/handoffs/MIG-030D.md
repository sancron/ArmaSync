# Handoff MIG-030D

- Status: reviewbereit; Claude; 2026-10-19; Basis `39de926`. Bericht: [MIG-030D](../changes/2026-10-19-MIG-030D.md).
- Windows: fmt, clippy, 288 Tests, MSRV, GUI-freier Baum, 14 Ende-zu-Ende-Tests gegen Loopback. **Offen:** WSL/Linux (inkl. Unix-Race, Release-Blocker für „race-freie Pfadgrenze“), GitHub-CI pausiert.
- Nächste Schritte (Vorschlag): Ctrl+C-Handler (Exitcode 130, kooperativer Abbruch), Authentifizierung (`CredentialStore`), Resume (MIG-032), ZIP-Blätter (MIG-030E), GUI-Anbindung (MIG-031); vor dem Linux-Release `openat`-basierte Pfadgrenze.
- Offen: P1-GUI-Gate, RQ-01..RQ-08.
