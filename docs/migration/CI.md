# CI (MIG-013)

Stand: 2026-10-18. **Erster GitHub-Lauf am 2026-10-18** (Push von `development`, [Run 39164378010](https://github.com/sancron/ArmaSync/actions/runs/39164378010), Commit `6b1f261`): die Jobs `Rust (windows-latest)`, `Rust (ubuntu-latest)`, `MSRV 1.85 (without GUI crate)` und `Fixtures and test server` waren **erfolgreich**; `Branch flow` wurde übersprungen, weil er nur bei Pull Requests läuft und damit noch **nie ausgeführt** wurde. Weiterhin gilt: kein `actionlint`, Actions nicht auf Commit-SHAs gepinnt, nicht als Required Check eingetragen, keine Rulesets. Lokale Läufe unten beziehen sich auf dieselben Befehle unter Windows 11 und WSL2.

## GitHub-Prüfungen pausiert (PROC-002, seit 2026-10-19)

Wegen des knappen Free-Limits von GitHub Actions laufen **keine automatischen GitHub-Prüfungen**. Maßgeblich sind bis auf Widerruf die **lokalen Windows-Prüfungen** (fmt, clippy `-D warnings`, Workspace-Tests, MSRV 1.85, `cli-gui-free.sh`, gezielte Tests, `git diff --check`).

| Workflow | Art | Zustand |
| --- | --- | --- |
| `.github/workflows/ci.yml` („CI“: Jobs `branch-policy`, `rust`, `msrv`, `fixtures`) | Build-, Test- und Branch-Policy-Prüfung | **pausiert**: `push` und `pull_request` entfernt (als Kommentar im Kopf der Datei erhalten), nur `workflow_dispatch` bleibt; während der Pause **nicht starten** |
| `.github/workflows/publish-winget.yml` | Veröffentlichung (WinGet) | **unverändert** (Release-Trigger und manueller Trigger), kein Testworkflow |

Keine GitHub-Einstellungen, Rulesets oder Required Checks wurden geändert; kein Workflow wurde gelöscht oder umbenannt. Konsequenz: Ein Pull Request oder Push löst keine Prüfung aus, auch `Branch flow` läuft nicht; Fast-Forward-Pushes nach `development` stützen sich auf die lokalen Windows-Prüfungen im Bericht. Die letzte grüne GitHub-CI gehört zu `15b7a32`; spätere Stände haben **keine** CI-Aussage. Die Ubuntu-Jobs laufen nicht; das ändert nichts an der zurückgestellten WSL-/Linux-Abschlussvalidierung.

**Wiederherstellung:** In `.github/workflows/ci.yml` den Block `on:` durch die im Kommentar stehenden ursprünglichen Trigger ergänzen (`pull_request` mit Branches `development, testing, master, legacy`; `push` mit Branches `development, testing, master`; `workflow_dispatch` beibehalten), den Pausenhinweis entfernen, per YAML-Prüfung kontrollieren, in einem eigenen Commit nach `development` bringen und den ersten Lauf auf dem Push dokumentieren (Run, Commit, Jobs). Erst danach gelten Aussagen zur GitHub-CI wieder.

## Prüfebenen und Prüfregel (seit 2026-10-19)

| Ebene | Wo | Wann | Status in Berichten |
| --- | --- | --- | --- |
| Lokale Entwicklungs- und Testläufe | Windows 11 | laufend, vor jeder Integration | Befehle und Ergebnisse nennen |
| Automatisierte CI | GitHub Actions: `windows-latest` und `ubuntu-latest` (Jobs unten); **derzeit pausiert** (siehe oben) | nach jedem Push/PR, solange nicht pausiert | Run, Commit und Jobergebnis getrennt festhalten; während der Pause „pausiert, nicht geprüft“ |
| Lokale WSL-/Linux-Prüfung | WSL2 Ubuntu | **erst in der abschließenden Plattformvalidierung**; bis dahin keine Läufe | als **offen** kennzeichnen; keine Aussage aus älteren Läufen |

Die Ubuntu-CI ist **keine** lokale Linux-Prüfung und ersetzt die Abschlussvalidierung nicht; sie wird deshalb auch nicht als solche ausgegeben. Der Abschnitt „Lokal ausgeführt“ unten ist **historisch** (2026-10-18, einschließlich WSL2) und beschreibt nicht den aktuellen Stand. Die CI-Plattformen bleiben unverändert.

**CI-Stand `8e3b25d` (2026-10-19, per `gh run view` geprüft, [Run 39255913370](https://github.com/sancron/ArmaSync/actions/runs/39255913370)):** `Rust (windows-latest)`, `Rust (ubuntu-latest)`, `MSRV 1.85 (without GUI crate)` und `Fixtures and test server` erfolgreich; `Branch flow` übersprungen (nur Pull Requests).

## Dateien

| Datei | Zweck |
| --- | --- |
| `.github/workflows/ci.yml` | Workflow „CI“: Auslöser Pull Request (Ziele `development`, `testing`, `master`, `legacy`), Push auf `development`/`testing`/`master`, manuell |
| `.github/scripts/branch-policy.sh` | Branch-Flow-Prüfung nach [BRANCHING](../BRANCHING.md): Task-Branch → `development` → `testing` → `master`; `legacy` nie Ziel; Forks nur nach `development` |
| `.github/scripts/test-branch-policy.sh` | Tabellentest der Policy (18 Fälle, positive und negative) |
| `.github/scripts/cli-gui-free.sh` | Schlägt fehl, wenn der CLI-Abhängigkeitsbaum GUI-/Windowing-/Accessibility-Toolkit-Crates enthält |

## Jobs

| Job | Läuft auf | Prüft |
| --- | --- | --- |
| `branch-policy` (nur Pull Requests) | ubuntu-latest | Quell-/Zielbranch und Fork-Herkunft; Selbsttest des Skripts |
| `rust` (Matrix) | windows-latest, ubuntu-latest | Toolchain aus `rust-toolchain.toml` (1.99.1), `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace --locked`, `cargo build -p armasync-cli --locked`, GUI-freier CLI-Baum, CLI-Rauchtest (`version`, `--json version`) |
| `msrv` | ubuntu-latest | `cargo +1.85.0 test --workspace --exclude armasync-gui --locked` |
| `fixtures` | ubuntu-latest | JVM-freier Fixture-Verifier (MIG-001), Offline-Exporter-Tests (MIG-021), Selbsttest des Transport-Testservers (MIG-003), Shell-Guard-Test (ohne Mount-Namespace-Rechte als „übersprungen“ akzeptiert, Exitcode 2) |

Nicht in der CI: die Java-Baseline (`gradle test :updater:test`, braucht JDK 21 und 25), der Fixture-Generator-Lauf (`run.sh`/`run.ps1`), `cargo audit`/`cargo deny`, Releasebuilds, Pakete/Installer, Signaturen, GUI-Tests mit Screenreader. Die bestehende WinGet-Automatik (`publish-winget.yml`) bleibt unverändert.

## Lokal ausgeführt (Windows 11 und WSL2 Ubuntu, 2026-10-18)

* Workflow-YAML mit PyYAML geladen (Trigger, vier Jobs, Matrix, Berechtigungen `contents: read`, jeder Schritt entweder `uses` oder `run`); **kein** `actionlint` und **kein** GitHub-Lauf (Docker-Daemon war nicht gestartet).
* Alle `run`-Befehle der Jobs `branch-policy`, `rust`, `msrv` und `fixtures` wurden unter Linux (WSL2, Ubuntu, Rust 1.99.1, LF-Clone des Branches) ausgeführt: 18 von 18 Policy-Fällen, fmt, clippy, 81 Tests, CLI-Build, GUI-freier Baum (37 Pakete), CLI-Rauchtest, MSRV 1.85, Verifier, Testserver-Selbsttest; der Guard-Test endete wie vorgesehen mit Exit 2 (übersprungen, kein root).
* Unter Windows 11 (Git Bash, Rust 1.99.1): Policy-Tests, `cli-gui-free.sh`, CLI-Rauchtest sowie fmt, clippy, 81 Tests und MSRV-Test (vorher auf dem integrierten Stand).
* Negativkontrolle: `cli-gui-free.sh` gegen `armasync-gui` gerichtet schlägt fehl und listet `egui`/`egui_extras`.

## Nicht geprüft und Risiken

* Verhalten auf den GitHub-Runnern (Toolchain-Installation aus `rust-toolchain.toml`, vorinstallierte Systembibliotheken für das GUI-Crate unter `ubuntu-latest`, Laufzeiten, Caching).
* Actions sind nur mit Major-Tag (`actions/checkout@v4`) referenziert; vor dem Aktivieren auf geprüfte Commit-SHAs pinnen. Kein Cache für Cargo (bewusst weggelassen).
* Die Policy kennt keine Ausnahmen außer `rc-fix/*` nach `testing`; Hotfix-Abläufe wären bei Bedarf zu ergänzen.

## Offene Administrationsschritte (nicht freigegeben, nach außen wirksam)

1. Änderung pushen und Workflow auf GitHub erstmals laufen lassen; Ergebnis prüfen.
2. Erst danach tatsächlich vorhandene Checknamen (`Branch flow`, `Rust (windows-latest)`, `Rust (ubuntu-latest)`, `MSRV 1.85 (without GUI crate)`, `Fixtures and test server`) als Required Checks in Rulesets für `master`, `testing` und `development` eintragen; `legacy` gegen Updates/Löschen sperren.
3. Default-Branch-Wechsel auf `development`, Release-Umgebung und geschützte Tags nach BRANCHING einrichten; Zustand dokumentieren.
4. WinGet-Automatik vor der ersten neuen Produktveröffentlichung kontrolliert ersetzen.
