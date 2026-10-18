# Gemeinsamer Arbeitsvertrag für Claude und Codex

Diese Regeln benötigen keine anbieterspezifischen Befehle. Beide Systeme lesen normales Markdown und denselben Git-Zustand. Einstieg: `AGENTS.md` beziehungsweise `CLAUDE.md` → `AGENTS.md`. Ein Prompt ist ein Arbeitsauftrag, kein Ersatz für Repositoryzustand und Statusdateien.

## Quellenhierarchie

Aktueller Nutzerauftrag → `AGENTS.md`/`PRODUCT.md` → angenommene ADRs → zugewiesener Task und dessen Verträge → Roadmap/Paritätsmatrix → Analyse/Altquellen. Bei Widerspruch nicht still wählen: Konflikt dokumentieren, betroffenen Teil abgrenzen, unabhängige Arbeit fortsetzen. Neue ADRs dürfen Nutzervorgaben nicht eigenmächtig aufheben.

## Vor Beginn

1. `git status --short`, `git branch --show-current`, `git rev-parse HEAD` und letzte Änderungen prüfen.
2. `docs/migration/STATUS.md`, `docs/changes/LAST_CHANGE.md`, Task-Datei und relevante Handoffs lesen. Veraltete Handoffs am Commit/Branch erkennen.
3. Eigentümer, Integrator, Basis-SHA, erlaubte Dateien und Abhängigkeiten im Task prüfen. Nicht eigenständig den Task eines anderen Agenten übernehmen.
4. Kurz mitteilen, was umgesetzt und wie es überprüft wird. Routinearbeit ohne erneute Bestätigung erledigen.

## Zuweisung ohne Schreibkollision

Ein menschlicher oder ausdrücklich benannter KI-Integrator verwaltet je Batch das [Task-Board](../tasks/README.md). Vor parallelem Start erstellt/committet er pro Task eine separate Datei mit **einem** Eigentümer und disjunktem Schreibbereich auf der gemeinsamen Basis. Erst nach diesem Commit gelten Aufgaben als reserviert. Zwei Systeme dürfen nicht gleichzeitig durch lokale Änderungen derselben Boarddatei „Locks“ beanspruchen: Git-Merges sind kein verteilter Lockdienst.

Unzugewiesene Aufgaben bleiben unzugewiesen. Ein einzelnes gestartetes System kann den ersten freien Task plus Integratorrolle übernehmen und diese Reservierung vor weiterer paralleler Arbeit committen. Falls bereits ein Integrator arbeitet, bei dessen Zuweisung bleiben. Übergabe/Entzug nur als explizite dokumentierte Koordination; keine automatische Lease-Übernahme allein aufgrund Zeitablauf.

Jeder Eigentümer ändert seine Task-Datei, seinen Report und seinen Handoff. **Integrator-exklusiv:** `STATUS.md`, `LAST_CHANGE.md`, `docs/tasks/README.md`, `CHANGELOG.md`, globale Roadmap-/Paritätszustände, Workspace-Manifeste und `Cargo.lock` sowie spätere zentrale CI-/Releasekonfiguration. Er kann einzelne zentrale Dateien ausdrücklich einem Task exklusiv zuweisen. Codeänderungen an einer Shared API erst nach integriertem Vertrags-Commit; bei unerwartetem Bedarf den Schreibbereich neu zuweisen.

## Getrennte Worktrees

Diese Beispiele laufen aus dem Hauptcheckout, wenn die Zielpfade noch nicht existieren. Verzeichnisnamen sind Beispiele, nicht ungeprüfte Lösch-/Moveziele.

```text
git worktree add -b codex/mig-001-formats ../ArmaSync-codex development
git worktree add -b claude/mig-002-licenses ../ArmaSync-claude development
git worktree list
```

Jedes System arbeitet ausschließlich in seinem zugewiesenen Worktree. Separate Buildverzeichnisse benutzen, keine gemeinsame `CARGO_TARGET_DIR`. Keine parallelen Branchwechsel im gleichen Checkout; keine fremden Reset/Clean/Stash-/Rebase-Aktionen. Bei vorhandenen uncommitteten Änderungen nur eigene Dateien gezielt stagen. Niemals `git add .` ungeprüft über fremde Arbeit ausführen.

## Ablauf pro Task

`geplant → zugewiesen → in Arbeit → reviewbereit → integriert`. `blockiert` mit konkreter Ursache, fehlender Information und unabhängiger Restarbeit dokumentieren. Ein Selbstbericht „fertig“ bedeutet zunächst reviewbereit, keine automatische Integration oder Releasefreigabe.

1. Vertrag und Abnahmekriterien prüfen, kleinsten abgeschlossenen Schritt implementieren.
2. Passende Tests/Checks ausführen; Befehle, OS, Toolchain und Resultate festhalten. Nicht ausgeführte Checks ausdrücklich nennen.
3. Eigene Änderungen als kleine sachliche Commits mit Task-ID committen. Keine falschen Testergebnisse oder zusammengesuchten Fremd-SHAs.
4. Eigenen Report `docs/changes/YYYY-MM-DD-MIG-NNN.md` und Handoff `docs/handoffs/MIG-NNN.md` aktualisieren. Report referenziert Codecommit(s); der Dokumentationscommit muss nicht seinen eigenen Hash enthalten.
5. Integrator prüft Diff und Verträge, übernimmt in Abhängigkeitsreihenfolge über PR oder nachvollziehbaren Merge nach `development`. Tests am **zusammengesetzten** Zustand ausführen; Tests aus isolierten Worktrees reichen nicht.
6. Nur der Integrator aktualisiert globalen Status, letzte Änderung, Changelog und Feature-Gates. Bei Konflikten fachliche Ursache klären, nicht blind `ours`/`theirs` wählen. Veraltete Tasks auf neue Basis bringen, ohne fremde Arbeit umzuschreiben.

## Commit- und Änderungsnachweis

Empfohlen: `docs(MIG-000): ...`, `feat(MIG-030): ...`, `fix(MIG-032): ...`, `test(MIG-001): ...`. Autor-/Coautorzeilen nur wahrheitsgemäß, keine erfundenen Personen.

- **Nutzerrelevante Historie:** `CHANGELOG.md` mit Release-/Unreleased-Änderungen.
- **Technischer Nachweis:** eigener Report mit Warum, Was, Dateien, Belegen, Migrations-/Lizenz-/Brandingfolgen.
- **Aktuelle Fortsetzung:** eigener Handoff und Taskstatus.
- **Letzte integrierte Änderung:** ausschließlich `docs/changes/LAST_CHANGE.md`; Status des zuletzt gestarteten Agenten ist dafür irrelevant.
- **Reihenfolge:** Integration anhand Git-Commit und Merge-Reihenfolge, Datum zusätzlich in ISO-8601 mit Zeitzone. Keine „zuletzt“-Aussage nur anhand Dateizeitstempel.

Report-/Handoff-/Task-Vorlagen liegen in den jeweiligen Verzeichnissen. Diese Vorlagefelder sind Platzhalter, keine bearbeiteten Tasks.

## Branch- und Releasegrenzen

`legacy` niemals verändern. `master` und `testing` nur über [BRANCHING.md](../BRANCHING.md). Keine automatische Veröffentlichung, Signaturschlüsselerzeugung auf fremde Identität oder Änderung von DNS/Hosting. Normale lokale Implementierung und Tests bis zum reviewbaren Ergebnis fortsetzen. Push-/PR-Aktionen nur im vorhandenen Nutzerauftrag bzw. der zugewiesenen Projektarbeitsberechtigung durchführen.

## Parallelisierungsbeispiele

P0: Formate vs. Lizenz-/Assetinventur. P1 nach Verträgen: CLI-Ausgabe vs. GUI-Viewmodels. P4 nach Transportvertrag: SFTP vs. WebDAV in getrennten Dateien. Ungeeignet: beide Systeme bearbeiten zugleich Root-Cargo.toml, Cargo.lock oder denselben Syncalgorithmus. Der gemeinsame Core-Vertrag wird vor der Aufteilung integriert und getestet.
