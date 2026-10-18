# CI (MIG-013)

Stand: 2026-10-18. **Der Workflow ist lokal vorbereitet und hat auf GitHub noch nie gelaufen.** Er ist nicht gepusht, nicht aktiviert und nicht als Required Check eingetragen. Aussagen unten über „bestanden“ beziehen sich auf die **lokale Ausführung derselben Befehle**, nicht auf einen GitHub-Lauf.

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
| `fixtures` | ubuntu-latest | JVM-freier Fixture-Verifier (MIG-001), Selbsttest des Transport-Testservers (MIG-003), Shell-Guard-Test (ohne Mount-Namespace-Rechte als „übersprungen“ akzeptiert, Exitcode 2) |

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
