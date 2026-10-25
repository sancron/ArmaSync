# Task-Board

Gemeinsamer Status nur durch den Integrator ändern. Vor parallelem Start Eigentümer, Basis-SHA und disjunkte Schreibbereiche in einzelnen Task-Dateien committen. Ein Plan ist keine Reservierung.

Aktueller Batch: Integrator **Claude**. Rollenregel: Claude ist Standard-Hauptbearbeiter und Integrator, Codex Fallback (siehe [WORKFLOW](../ai/WORKFLOW.md)). MIG-001, MIG-002, MIG-003, MIG-010, MIG-011, MIG-012 (Prototyp), MIG-012A, MIG-013, MIG-020 bis MIG-023, MIG-030A, MIG-030P, MIG-030B, MIG-030C, MIG-030H, MIG-030D, MIG-030E, MIG-030T, MIG-031 (GUI-Teil von P3), MIG-032A und MIG-032B sind nach `development` integriert; MIG-032R ist vorbereitet (geplant, nicht begonnen); bis MIG-020 ist `origin/development` @ `734a20b` (2026-10-18, auf Autorisierung des Projektverantwortlichen, CI grün) gepusht; `testing`/`master`/`legacy` unverändert. MIG-002/003 wurden am 2026-10-18 vom Projektverantwortlichen von Codex an Claude übertragen (Codex-Vorarbeit `760e20d`, `044e6aa` bleibt in der Historie; Details in den Taskdateien). Offene Rechtefragen: [RIGHTS_QUESTIONS.md](RIGHTS_QUESTIONS.md). MIG-010 und MIG-011 sind integriert.

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
| MIG-030 | Scan/Plan/HTTP(S)/atomare Installation (Dachtask, zerlegt in A–E) | Claude | Vorprüfung abgeschlossen ([ADR-0006](../adr/0006-sync-metadata-and-http-transport.md), [SYNC_METADATA](../migration/SYNC_METADATA.md)); Umsetzung in Teiltasks | MIG-023 |
| MIG-030E | ZIP-Blätter mit Archivgröße sicher entpacken (Spool, ein Eintrag, Grenzen, SHA-1-Prüfung) | Claude (`claude/mig-030e-zip`) | integriert (`4c6be08`, gepusht 2026-10-25); echte Serverarchive nicht belegt; GUI-Hinweis angeglichen | MIG-030D |
| MIG-032B | Freiplatzprüfung (Vorabkontrolle, Port `free_space`), Abbruchtests für große und gepackte Downloads | Claude (`claude/mig-032b-freiplatz`) | integriert (Fast-Forward 2026-10-25); Konsolen-Ctrl+C nicht behauptet; WSL/Linux, CI, Server offen | MIG-030E, MIG-032A |
| MIG-032R | Resume mit `Range` und Validatorvertrag ([ADR-0009](../adr/0009-resume-range.md)) | Claude (`claude/mig-032r-resume`) | in Arbeit (reserviert 2026-10-25, Basis `1d7049d`); Vertrag vor Code | MIG-032A |
| MIG-032A | Ctrl+C-Abbruch der CLI, Fehlerfälle des Zielordners (Teil 1 von MIG-032), GUI-Screenreader-Protokoll für die Sync-Ansicht | Claude (`claude/mig-032a-cancel`) | integriert (Merge 56a40f3); Signalzustellung nur manuell prüfbar; Resume/Authentifizierung offen | MIG-031 |
| PROC-003 | Plattformsperre ohne Umgehung im Release-Build (Override nur mit Validierungs-Feature) | Claude (`claude/proc-003-platform-gate`) | integriert (Merge 15ada32) | MIG-031 |
| MIG-030T | Testflake-Analyse und Stabilisierung (Ordnerkollision, Portwiederverwendung, expliziter Handshake) | Claude (`claude/mig-030t-race-test`) | integriert (Merge bd17b1d) | MIG-030D |
| MIG-031 | Sync-GUI (Plan, Löschbestätigung, Fortschritt, Abbruch, Ergebnis), gemeinsame Crate `armasync-workflow`, `armasync-testkit` | Claude (`claude/mig-031-sync-gui`) | integriert (Merge f53ac24); Windows automatisiert + einmal UI Automation; **NVDA/Orca/Sichtprüfung offen (P1-Gate)**; Linux nur Plan | MIG-030D |
| MIG-030D | CLI `sync plan`/`sync run`, Ende-zu-Ende gegen Loopback-Server | Claude (`claude/mig-030d-sync-cli`) | integriert (Merge 0e76255); unter Windows gegen lokalen Loopback-Server belegt; Linux/echte Server offen | MIG-030B, MIG-030C, MIG-030H |
| MIG-030H | Hardening Dateisystemadapter: exklusive Zielsperre, Windows-Ordner-Pinning ([Analyse](../migration/LOCAL_FS_THREATS.md)) | Claude (`claude/mig-030h-hardening`) | integriert (Merge 39f7ebd); Unix-Race **nicht** behoben (Release-Blocker für „race-freie Pfadgrenze unter Linux“) | MIG-030C |
| MIG-030C | Dateisystemadapter (Scan, Staging, atomarer Austausch, Containment) | Claude (`claude/mig-030c-localfs`) | integriert (Merge 3fdb49a); unter Windows geprüft, Linux/WSL offen | MIG-030P |
| MIG-030B | HTTP(S)-Adapter (`ureq`, TLS immer geprüft, Streaming) | Claude (`claude/mig-030b-http`) | integriert (Merge 7ded7b3); gegen Loopback-/Test-TLS-Server belegt, keine echten Server | MIG-030P |
| MIG-030P | Vertrag: Streaming-Übertragung, Staging, Schutzpfade ([ADR-0007](../adr/0007-transfer-ports.md)) | Claude (`claude/mig-030p-ports`) | integriert (Merge b040c02); nur gegen In-Memory-Referenz belegt | MIG-030A |
| MIG-030A | Sync-Metadaten-Leser, SHA-1 im Core, Manifest-Abbildung (kein Netzwerk/Dateisystem) | Claude (`claude/mig-030a-sync-metadata`) | integriert (Merge 460e4ac); es wird noch nichts Reales synchronisiert | MIG-022 |
| MIG-023 | CLI-Anzeige importierter Repositories und Profile (`repository list`, `profile list`) | Claude (`claude/mig-023-repository-list`) | integriert (Merge 738f035); CLI-Teil des P2-Gates | MIG-022 |
| MIG-022 | Begrenzter Import von Server-Metadaten (`.a3s/autoconfig`), Umfangs-Hinweise | Claude (`claude/mig-022-server-metadata`) | integriert (Merge `0f2ee31`); nur Name/Adresse, kein Netzwerkabruf; löst weder Entschlüsselung noch Einstellungen/Addon-Listen | MIG-021 |

Weitere IDs und Gates: [ROADMAP.md](../migration/ROADMAP.md). Neue Taskdateien aus [TEMPLATE.md](TEMPLATE.md) erstellen. MIG-001/002 dürfen parallel laufen, wenn Zuweisungen zuvor auf gemeinsamer Basis committed wurden.
