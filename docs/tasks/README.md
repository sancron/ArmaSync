# Task-Board

Gemeinsamer Status nur durch den Integrator ändern. Vor parallelem Start Eigentümer, Basis-SHA und disjunkte Schreibbereiche in einzelnen Task-Dateien committen. Ein Plan ist keine Reservierung.

Aktueller Batch: Integrator **Claude**. Rollenregel: Claude ist Standard-Hauptbearbeiter und Integrator, Codex Fallback (siehe [WORKFLOW](../ai/WORKFLOW.md)). MIG-001, MIG-002, MIG-003, MIG-010, MIG-011, MIG-012 (Prototyp) und MIG-013 (CI lokal) sind lokal nach `development` integriert und **nicht gepusht**. MIG-002/003 wurden am 2026-10-18 vom Projektverantwortlichen von Codex an Claude übertragen (Codex-Vorarbeit `760e20d`, `044e6aa` bleibt in der Historie; Details in den Taskdateien). Offene Rechtefragen: [RIGHTS_QUESTIONS.md](RIGHTS_QUESTIONS.md). MIG-010 und MIG-011 sind integriert.

| ID | Aufgabe | Eigentümer | Status | Voraussetzung |
| --- | --- | --- | --- | --- |
| MIG-000 | Analyse, Plan, gemeinsame KI-Regeln und Branchvorbereitung | Codex | abgeschlossen; Commit über Dateihistorie ermitteln | Baseline |
| MIG-001 | Formatfixtures und Export-/Parsergrundlage | Claude (`claude/mig-001-formats`) | integriert (lokal, nicht gepusht) | MIG-000 |
| MIG-002 | Rechte-/Assets-/JAR-Inventur | Claude (übertragen von Codex; `codex/mig-002-rights`) | integriert (lokal, nicht gepusht) | MIG-000 |
| MIG-003 | Transport-/Testserver-/Plattformmatrix | Claude (übertragen von Codex; `codex/mig-003-transport`) | integriert (lokal, nicht gepusht) | MIG-000 |
| MIG-010 | Cargo-Workspace, Toolchain/MSRV, CLI-Basis | Claude (`claude/mig-010-workspace`) | integriert (lokal, nicht gepusht) | P0-Gate |
| MIG-011 | Kernverträge, Ports, Events, Jobzustände, In-Memory-Use-Case | Claude (`claude/mig-011-contracts`) | integriert (lokal, nicht gepusht) | MIG-010 |
| MIG-012 | Nativer GUI-Prototyp, ADR-0002 | Claude (`claude/mig-012-gui-prototype`) | Prototyp integriert (lokal, nicht gepusht); **GUI-/Accessibility-Gate offen** (NVDA/Orca) | MIG-011 |
| MIG-013 | CI Windows/Linux, Branch-Flow-Prüfung | Claude (`claude/mig-013-ci`) | lokal integriert (nicht gepusht, auf GitHub nie gelaufen); Rulesets/Required Checks offen und nicht freigegeben | MIG-010 |

Weitere IDs und Gates: [ROADMAP.md](../migration/ROADMAP.md). Neue Taskdateien aus [TEMPLATE.md](TEMPLATE.md) erstellen. MIG-001/002 dürfen parallel laufen, wenn Zuweisungen zuvor auf gemeinsamer Basis committed wurden.
