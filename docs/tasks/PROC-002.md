# PROC-002 — GitHub-CI pausieren (lokale Windows-Prüfungen maßgeblich)

- Status: reviewbereit (2026-10-19). Eigentümer und Integrator: Claude. Basis `15b7a32297898130900fa9d81fe65a694807bda8`; Branch `claude/proc-002-ci-pause`.
- Schreibbereich: `.github/workflows/ci.yml` (nur der `on:`-Block und ein Kommentar), `AGENTS.md`, `docs/ai/WORKFLOW.md`, `docs/migration/CI.md`, diese Datei, Bericht, Handoff.
- Ziel: wegen des knappen Free-Limits keine automatischen GitHub-Actions-Läufe; Release-/WinGet-Automation, Einstellungen, Rulesets, Required Checks und Workflow-Namen unverändert.
- Ergebnis: [Bericht](../changes/2026-10-19-PROC-002.md), [Handoff](../handoffs/PROC-002.md).
