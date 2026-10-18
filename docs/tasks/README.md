# Task-Board

Gemeinsamer Status nur durch den Integrator ändern. Vor parallelem Start Eigentümer, Basis-SHA und disjunkte Schreibbereiche in einzelnen Task-Dateien committen. Ein Plan ist keine Reservierung.

Aktueller Batch (Basis `development` @ `af1347c040531a0f6900d3ca1e9736e2b1286721`): Integrator **Claude** (initiale Rolle, übernommen am 2026-10-18). MIG-001 ist Claude zugewiesen ([Taskdatei](MIG-001.md)). MIG-002 und MIG-003 sind frei und für Codex vorgesehen, sobald der Projektverantwortliche sie zuweist; ihre Schreibbereiche (Lizenz-/Asset-Dokumente bzw. Transportmatrix) überschneiden sich nicht mit MIG-001.

| ID | Aufgabe | Eigentümer | Status | Voraussetzung |
| --- | --- | --- | --- | --- |
| MIG-000 | Analyse, Plan, gemeinsame KI-Regeln und Branchvorbereitung | Codex | abgeschlossen; Commit über Dateihistorie ermitteln | Baseline |
| MIG-001 | Formatfixtures und Export-/Parsergrundlage | Claude (`claude/mig-001-formats`) | zugewiesen | MIG-000 |
| MIG-002 | Rechte-/Assets-/JAR-Inventur | frei | geplant | MIG-000 |
| MIG-003 | Transport-/Testserver-/Plattformmatrix | frei | geplant | MIG-000 |
| MIG-010–013 | Rustfundament, Verträge, GUI-Auswahl, CI | frei | geplant | P0-Gate |

Weitere IDs und Gates: [ROADMAP.md](../migration/ROADMAP.md). Neue Taskdateien aus [TEMPLATE.md](TEMPLATE.md) erstellen. MIG-001/002 dürfen parallel laufen, wenn Zuweisungen zuvor auf gemeinsamer Basis committed wurden.
