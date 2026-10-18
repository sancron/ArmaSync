# Task-Board

Gemeinsamer Status nur durch den Integrator ändern. Vor parallelem Start Eigentümer, Basis-SHA und disjunkte Schreibbereiche in einzelnen Task-Dateien committen. Ein Plan ist keine Reservierung.

Aktueller Batch: Integrator **Claude**. Rollenregel: Claude ist Standard-Hauptbearbeiter und Integrator, Codex Fallback (siehe [WORKFLOW](../ai/WORKFLOW.md)). MIG-001, MIG-002 und MIG-003 sind lokal nach `development` integriert und **nicht gepusht**. MIG-002/003 wurden am 2026-10-18 vom Projektverantwortlichen von Codex an Claude übertragen (Codex-Vorarbeit `760e20d`, `044e6aa` bleibt in der Historie; Details in den Taskdateien). Offene Rechtefragen: [RIGHTS_QUESTIONS.md](RIGHTS_QUESTIONS.md). MIG-010 ist noch nicht gestartet (siehe [STATUS](../migration/STATUS.md)).

| ID | Aufgabe | Eigentümer | Status | Voraussetzung |
| --- | --- | --- | --- | --- |
| MIG-000 | Analyse, Plan, gemeinsame KI-Regeln und Branchvorbereitung | Codex | abgeschlossen; Commit über Dateihistorie ermitteln | Baseline |
| MIG-001 | Formatfixtures und Export-/Parsergrundlage | Claude (`claude/mig-001-formats`) | integriert (lokal, nicht gepusht) | MIG-000 |
| MIG-002 | Rechte-/Assets-/JAR-Inventur | Claude (übertragen von Codex; `codex/mig-002-rights`) | integriert (lokal, nicht gepusht) | MIG-000 |
| MIG-003 | Transport-/Testserver-/Plattformmatrix | Claude (übertragen von Codex; `codex/mig-003-transport`) | integriert (lokal, nicht gepusht) | MIG-000 |
| MIG-010–013 | Rustfundament, Verträge, GUI-Auswahl, CI | frei (nicht gestartet) | geplant; P0-Gate laut ROADMAP mit Auflagen erfüllt, Start scheitert aktuell an fehlender Rust-Toolchain (`cargo`/`rustc` nicht installiert) | P0-Gate |

Weitere IDs und Gates: [ROADMAP.md](../migration/ROADMAP.md). Neue Taskdateien aus [TEMPLATE.md](TEMPLATE.md) erstellen. MIG-001/002 dürfen parallel laufen, wenn Zuweisungen zuvor auf gemeinsamer Basis committed wurden.
