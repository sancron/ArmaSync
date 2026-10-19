# Task-Board

Gemeinsamer Status nur durch den Integrator ändern. Vor parallelem Start Eigentümer, Basis-SHA und disjunkte Schreibbereiche in einzelnen Task-Dateien committen. Ein Plan ist keine Reservierung.

Aktueller Batch: Integrator **Claude**. Rollenregel: Claude ist Standard-Hauptbearbeiter und Integrator, Codex Fallback (siehe [WORKFLOW](../ai/WORKFLOW.md)). MIG-001, MIG-002, MIG-003, MIG-010, MIG-011, MIG-012 (Prototyp), MIG-012A, MIG-013, MIG-020, MIG-021, MIG-022 und MIG-023 sind nach `development` integriert; bis MIG-020 ist `origin/development` @ `734a20b` (2026-10-18, auf Autorisierung des Projektverantwortlichen, CI grün) gepusht; `testing`/`master`/`legacy` unverändert. MIG-002/003 wurden am 2026-10-18 vom Projektverantwortlichen von Codex an Claude übertragen (Codex-Vorarbeit `760e20d`, `044e6aa` bleibt in der Historie; Details in den Taskdateien). Offene Rechtefragen: [RIGHTS_QUESTIONS.md](RIGHTS_QUESTIONS.md). MIG-010 und MIG-011 sind integriert.

| ID | Aufgabe | Eigentümer | Status | Voraussetzung |
| --- | --- | --- | --- | --- |
| MIG-000 | Analyse, Plan, gemeinsame KI-Regeln und Branchvorbereitung | Codex | abgeschlossen; Commit über Dateihistorie ermitteln | Baseline |
| MIG-001 | Formatfixtures und Export-/Parsergrundlage | Claude (`claude/mig-001-formats`) | integriert (gepusht 2026-10-18) | MIG-000 |
| MIG-002 | Rechte-/Assets-/JAR-Inventur | Claude (übertragen von Codex; `codex/mig-002-rights`) | integriert (gepusht 2026-10-18) | MIG-000 |
| MIG-003 | Transport-/Testserver-/Plattformmatrix | Claude (übertragen von Codex; `codex/mig-003-transport`) | integriert (gepusht 2026-10-18) | MIG-000 |
| MIG-010 | Cargo-Workspace, Toolchain/MSRV, CLI-Basis | Claude (`claude/mig-010-workspace`) | integriert (gepusht 2026-10-18) | P0-Gate |
| MIG-011 | Kernverträge, Ports, Events, Jobzustände, In-Memory-Use-Case | Claude (`claude/mig-011-contracts`) | integriert (gepusht 2026-10-18) | MIG-010 |
| MIG-012 | Nativer GUI-Prototyp, ADR-0002 | Claude (`claude/mig-012-gui-prototype`) | Prototyp integriert (gepusht 2026-10-18); **GUI-/Accessibility-Gate offen** (NVDA/Orca) | MIG-011 |
| MIG-013 | CI Windows/Linux, Branch-Flow-Prüfung | Claude (`claude/mig-013-ci`) | integriert; mit `development` am 2026-10-18 gepusht, erster GitHub-Lauf erfolgreich (siehe STATUS); Rulesets/Required Checks offen und nicht freigegeben | MIG-010 |
| MIG-012A | GUI-Nachbesserung: Kontrast Auswahlfläche, Screenreader-Prüfprotokoll | Claude (`claude/mig-012a-contrast`) | integriert (Merge `9577b58`, gepusht); P1-Gate bleibt offen | MIG-012 |
| MIG-020 | Datenmodell, Stores, Pfade, Credentials | Claude (`claude/mig-020-data-store`) | integriert (Merge `83e8ebe`, gepusht); Schlüsselbund-Adapter offen (ADR-0005) | MIG-011 |
| MIG-021 | Offline-Export und neutraler Import lokaler Altdaten | Claude (`claude/mig-021-import`) | integriert (Merge `814720a`, gepusht 2026-10-18); verschlüsselte Repositorydateien nicht importiert (ADR-0003/RQ-01 offen) | MIG-020 |
| MIG-023 | CLI-Anzeige importierter Repositories und Profile (`repository list`, `profile list`) | Claude (`claude/mig-023-repository-list`) | integriert (Merge 738f035); CLI-Teil des P2-Gates | MIG-022 |
| MIG-022 | Begrenzter Import von Server-Metadaten (`.a3s/autoconfig`), Umfangs-Hinweise | Claude (`claude/mig-022-server-metadata`) | integriert (Merge `0f2ee31`); nur Name/Adresse, kein Netzwerkabruf; löst weder Entschlüsselung noch Einstellungen/Addon-Listen | MIG-021 |

Weitere IDs und Gates: [ROADMAP.md](../migration/ROADMAP.md). Neue Taskdateien aus [TEMPLATE.md](TEMPLATE.md) erstellen. MIG-001/002 dürfen parallel laufen, wenn Zuweisungen zuvor auf gemeinsamer Basis committed wurden.
