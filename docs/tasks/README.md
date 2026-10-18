# Task-Board

Gemeinsamer Status nur durch den Integrator ändern. Vor parallelem Start Eigentümer, Basis-SHA und disjunkte Schreibbereiche in einzelnen Task-Dateien committen. Ein Plan ist keine Reservierung.

Aktueller Batch: Integrator **Claude** (initiale Rolle, übernommen am 2026-10-18). MIG-001 ist integriert (Merge `de8c868` auf `development`, lokal). MIG-002 und MIG-003 sind vom Projektverantwortlichen Codex zugewiesen und auf der Basis `development` nach der MIG-001-Integration reserviert ([MIG-002](MIG-002.md), [MIG-003](MIG-003.md)); die Schreibbereiche sind disjunkt. MIG-010 beginnt erst nach dem P0-Gate.

| ID | Aufgabe | Eigentümer | Status | Voraussetzung |
| --- | --- | --- | --- | --- |
| MIG-000 | Analyse, Plan, gemeinsame KI-Regeln und Branchvorbereitung | Codex | abgeschlossen; Commit über Dateihistorie ermitteln | Baseline |
| MIG-001 | Formatfixtures und Export-/Parsergrundlage | Claude (`claude/mig-001-formats`) | integriert (lokal, nicht gepusht) | MIG-000 |
| MIG-002 | Rechte-/Assets-/JAR-Inventur | Codex (`codex/mig-002-rights`) | zugewiesen | MIG-000 |
| MIG-003 | Transport-/Testserver-/Plattformmatrix | Codex (`codex/mig-003-transport`) | zugewiesen | MIG-000 |
| MIG-010–013 | Rustfundament, Verträge, GUI-Auswahl, CI | frei | geplant; erst nach P0-Gate (MIG-002/003 abgeschlossen) | P0-Gate |

Weitere IDs und Gates: [ROADMAP.md](../migration/ROADMAP.md). Neue Taskdateien aus [TEMPLATE.md](TEMPLATE.md) erstellen. MIG-001/002 dürfen parallel laufen, wenn Zuweisungen zuvor auf gemeinsamer Basis committed wurden.
