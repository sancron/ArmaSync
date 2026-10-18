# Task-Board

Gemeinsamer Status nur durch den Integrator ändern. Vor parallelem Start Eigentümer, Basis-SHA und disjunkte Schreibbereiche in einzelnen Task-Dateien committen. Ein Plan ist keine Reservierung.

Aktueller Batch: **keine parallelen Implementierungsaufträge aktiv**. MIG-000 wurde von Codex als Dokumentationsvorbereitung bearbeitet. Integrator für den nächsten Batch: **noch nicht zugewiesen**. Das zuerst gestartete System kann nach Prüfung des aktuellen Git-Zustands MIG-001 und die anfängliche Integratorrolle übernehmen, solange noch kein anderer Integrator benannt wurde.

| ID | Aufgabe | Eigentümer | Status | Voraussetzung |
| --- | --- | --- | --- | --- |
| MIG-000 | Analyse, Plan, gemeinsame KI-Regeln und Branchvorbereitung | Codex | abgeschlossen; Commit über Dateihistorie ermitteln | Baseline |
| MIG-001 | Formatfixtures und Export-/Parsergrundlage | frei | geplant | MIG-000 |
| MIG-002 | Rechte-/Assets-/JAR-Inventur | frei | geplant | MIG-000 |
| MIG-003 | Transport-/Testserver-/Plattformmatrix | frei | geplant | MIG-000 |
| MIG-010–013 | Rustfundament, Verträge, GUI-Auswahl, CI | frei | geplant | P0-Gate |

Weitere IDs und Gates: [ROADMAP.md](../migration/ROADMAP.md). Neue Taskdateien aus [TEMPLATE.md](TEMPLATE.md) erstellen. MIG-001/002 dürfen parallel laufen, wenn Zuweisungen zuvor auf gemeinsamer Basis committed wurden.
