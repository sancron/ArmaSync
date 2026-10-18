# Mitarbeit an ArmaSync

Lies `AGENTS.md` und `docs/ai/WORKFLOW.md`; dieselben Anforderungen gelten für menschliche und KI-Beiträge. Task-Branch auf Basis von `development`, bei paralleler Arbeit eigener Worktree. Ein PR nennt Task-ID, Problem, resultierendes Verhalten, Prüfungen und Risiken.

Schnittstellen-, Format-, GUI-Technologie- und Lizenzentscheidungen in `docs/adr/` dokumentieren. Änderungen am Lizenzstatus brauchen eine Rechtebasis. Neue unabhängige Rust-Beiträge sind unter `AGPL-3.0-only` vorgesehen; Drittquellen behalten ihre Hinweise.

Funktionale Änderungen erhalten einen Bericht in `docs/changes/` und einen Handoff in `docs/handoffs/`. Veröffentlichungen gehen durch `testing` nach `master`. `legacy` bleibt unverändert.
