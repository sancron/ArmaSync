# Prüfung und Release-Abnahme

## Tatsächlich geprüft am 2026-10-18

Baseline `b7753cea18b2ac2cc57e29f926901997a4de8570`; Windows 11 x86_64, Oracle JDK 25.0.2, Gradle 9.4.1. Ausgeführt:

```text
gradle test :updater:test --console=plain
```

Ergebnis **BUILD SUCCESSFUL**, 20 JUnit-Suites, 64 Tests, 0 Failures, 0 Errors, 1 übersprungen; somit 63 erfolgreich. Der übersprungene Test `FtpDAOSafetyTest.copiesBuiltStandardReleaseArchiveWhenAvailable()` benötigt ein lokal gebautes Releasearchiv und wurde mit „local beta artifact is not available“ abgebrochen. Dies ist keine erfolgreiche Paketabnahme.

Reports nach lokaler Ausführung: `build/reports/tests/test/index.html`, `updater/build/reports/tests/test/index.html`; maschinenlesbar in den zugehörigen `test-results/test/TEST-*.xml`. Generierte Reports sind nicht Teil des Commits und können neu erzeugt werden. Updaterkompilierung meldete einen Deprecated-API-Hinweis; kein Testfehler.

Go-Bootstrapper-Prüfung und Dokumentationschecks werden im [MIG-000-Bericht](../changes/2026-10-18-MIG-000.md) mit ihrem tatsächlichen Resultat protokolliert. Es wurde keine GUI-/Linux-/Produktivnetzwerk-/Installerprüfung durchgeführt. `cargo`/`rustc` waren nicht im PATH; ein Rust-Workspace besteht noch nicht. Daher wurden keine Rustprüfungen behauptet.

## Nach Erstellung des Workspace

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
cargo build -p armasync-cli --locked
```

Auf Windows und Linux ausführen. Bei GUI-Systemdependencies getrennte CLI-/Core-Jobs auf einer Umgebung **ohne** GUI-Bibliotheken betreiben; zusätzlich Gesamtsystemjob mit den benötigten GUI-Abhängigkeiten. Konkrete Rust-Version und Featurematrix erst nach P1 pinnen, nicht ein ungeprüftes `--all-features` für inkompatible Backends verwenden. Lockfile bei Build nicht still verändern.

## Testschichten und Abnahmen

| Ebene | Erforderliche Nachweise |
| --- | --- |
| Core | deterministische Diffs/Pläne, Auswahl-/Löschregeln, IDs, Zustände, Größenüberläufe |
| Formate | Golden Read/Write-Vectors, Versionsgrenzen, ungültige/zu große/zyklische Eingaben, Fuzzing ohne OOM/Panic/Pfadausbruch |
| Persistenz | atomare Writes, Crash/Abbruch/Disk Full, Backup, idempotenter Import, Secrettrennung |
| Transport | lokale reproduzierbare HTTP(S)/FTP/FTPS/SFTP/WebDAV-Server, Auth/TLS/Host-Key, Proxy, Range/416, Timeout/Retry/Abbruch |
| Dateisystem | Windows/Linux, Case/Unicode, Symlinks/Junctions, Traversal, Dateisperren, lange Pfade, Cross-Device, fremde Dateien |
| Use-Cases | Sync/Verify/Build/Upload gegen Baseline-Fixtures; identisches Ergebnis über GUI und CLI |
| Launcher | Windows-Argumentvektoren, Pfade mit Leerzeichen, Sonderzeichen, ungültige Executables; unter Linux klare Unsupported-Capability |
| GUI | Tastatur, NVDA/Orca, 200 % Skalierung, Themes, lange Namen, leere/offline/Fehlerzustände, kein UI-Thread-I/O |
| CLI | Help, Exitcodes, JSON-Schema, stdout/stderr, Pipe/Nicht-TTY/`--no-input`, Ctrl+C, keine Secretlecks |
| Updater | gültige/ungültige Signatur, Version/Channel, Downgrade, unterbrochener Austausch, Windows-Dateisperre und Rollback |
| Pakete | frische Installationen, Update/Deinstallation/Nutzerdaten, GUI ohne Java, CLI ohne Displayserver, Desktop-Metadaten |
| Lizenz/Branding | Provenienz/Notices/SBOM/Quellen, exaktes Tag, keine alten eigenen Namen/Serverdefaults im ausgelieferten Produkt |

Tests sollen reale Risiken prüfen, keine privaten Implementierungsdetails spiegeln. Protokollintegration ist nicht durch reine Mocks erledigt; Accessibility nicht durch Farb-/Tokentests; Release nicht durch erfolgreiches Kompilieren.

## Leistungsprüfung

Vor Optimierung repräsentative Datensätze mit 1.000/10.000/50.000 Dateien, kleinen/vielen und großen PBO-Dateien anlegen. Scan-, Hash-, Plan-, Transferzeit, Speichermaximum, Netzwerkanfragen und UI-Reaktionsfähigkeit mit gleicher Hardware/Cache-/Netzwerksituation gegenüber Java messen. Erste Budgets in P1/P3 anhand dieser Messungen festlegen; keine erfundenen Leistungsversprechen. Worker-/Verbindungslimits und Abbruchlatenz gesondert beobachten.

## Releaseprotokoll pro Kandidat

Version, Quell-SHA/Tag, Buildumgebung/Toolchain, OS-Versionen, Artefakthashes, Signaturen, Prüfresultate, übersprungene Checks, Paritätsmatrix und Freigabe dokumentieren. `testing` bleibt Gate vor `master`. Nur tatsächlich bestandene Prüfungen als bestanden markieren; fehlende Plattform-/Sicherheits-/Lizenzabnahmen blockieren die zugehörige zugesagte Fähigkeit. Releaseprüfer übernimmt Änderungen nach Fehlerbehebung erst nach passender erneuter Prüfung.
