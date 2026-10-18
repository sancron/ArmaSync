# Aktueller Migrationsstand

Aktualisiert: **2026-10-18**, Europe/Berlin. Diesen globalen Stand nur durch den jeweiligen Integrator ändern.

## Erreicht

- MIG-000: Projektanalyse, vollständiges Baseline-Dateiinventar, P0–P7-Roadmap, Architektur-/Daten-/Branding-/Lizenzstrategie, GUI-/CLI-Anforderungen und gemeinsame KI-Arbeitsregeln erstellt.
- Alle vier Branches lokal und auf `origin` vorhanden und verifiziert. `legacy`, `master` und `testing` unverändert auf `b7753cea18b2ac2cc57e29f926901997a4de8570`; Migrationspaket auf `development`. Branchaufgaben in [BRANCHING.md](../BRANCHING.md).
- Java-Hauptprojekt und Updater erfolgreich getestet: 64 Tests, 63 erfolgreich, 1 wegen fehlendem Releasearchiv übersprungen.
- MIG-001 (Claude): lokal nach `development` integriert (Merge `de8c868`, nicht gepusht). Synthetische Fixtures, Generator, JVM-freier Verifier, Class-/UID-Inventar und Exportstrategie; Codex-Review freigegeben. Im zusammengesetzten Stand: `run.ps1` Exit 0, 23 Serialisierungen, 0 Probleme, Ergebnis byte-identisch zum eingecheckten Stand. Windows-Guard-Tests 4/4, Linux-Mountpoint-Guards 13/13 (WSL, root).
- MIG-002 (Rechte-/Asset-/JAR-Inventur, [RIGHTS_INVENTORY](RIGHTS_INVENTORY.md)): lokal integriert (Merge `681981c`, nicht gepusht). Integrationsreview fand vier falsche SHA-256-Werte; korrigiert, alle 87 Hashes automatisiert gegen die Baseline-Blobs geprüft. Befunde, keine Rechtefreigabe; Rechtefragen RQ-01..RQ-08 **alle offen** ([Tracker](../tasks/RIGHTS_QUESTIONS.md)).
- MIG-003 (Transportmatrix und Loopback-Testserver, [TRANSPORT_MATRIX](TRANSPORT_MATRIX.md)): lokal integriert (Merge `0fd7418`, nicht gepusht). Integrationsreview fand einen Auth-Bypass über kodierte `/auth`-Pfade; korrigiert, Selbsttest 31 Checks. Unter Windows 11 (Python 3.15.0) und WSL2-Ubuntu (Python 3.12.3) ausgeführt; `run.ps1`/`run.sh` kurz gestartet und geprüft. FTP/FTPS, SFTP, TLS, Proxy, Last und ein echter Java-DAO-Lauf gegen den Server sind **nicht** getestet.
- MIG-001-Nachtrag: vollständiger Linux-Generatorlauf (WSL2 Ubuntu, OpenJDK 25.0.4.1, Python 3.12.3, Gradle 9.4.1, LF-Clone) Exit 0, 0 Probleme, zweiter Lauf identisch, Fixture-Bytes identisch zu Windows (nur `generated_with_java` im Manifest unterscheidet sich). Dafür `run.sh`/`run.ps1` auf `gradle :compileJava` umgestellt (Merge `b29787f`).
- MIG-010 ([RUST_TOOLCHAIN](RUST_TOOLCHAIN.md)): lokal integriert (Merge `86203b7`, nicht gepusht). Cargo-Workspace `armasync-core`/`-application`/`-adapters`/`-cli`, Rust 1.99.1 gepinnt, MSRV 1.85 (geprüft), `Cargo.lock`, CLI `armasync version [--json]` mit Exitcode-Typ nach CLI_CONTRACT. fmt, clippy `-D warnings`, 13 Tests, Release-Build auf Windows 11; fmt/clippy/Tests/Build auf Linux (WSL2). Kein GUI-Crate (MIG-012). Keine Bestandsquellen/Assets übernommen; Abhängigkeiten: clap, serde_json (+ transitive).
- Rollenregel: Claude ist Standard-Hauptbearbeiter/Integrator, Codex Fallback ([WORKFLOW](../ai/WORKFLOW.md)); MIG-002/003 wurden einmalig von Codex übertragen.
- AGPLv3-Referenztext für neue Komponenten aufgenommen; vorhandene GPL-Lizenz nicht ersetzt.

## Noch nicht erreicht

P0-Gate nach den Kriterien in [ROADMAP](ROADMAP.md) bewertet (Fixtures mit Herkunft/Hashes/Generator, keine personenbezogenen Daten, Rechtefragen als getrennte bearbeitbare Punkte, Baseline/Branchregeln bestätigt): **erfüllt mit Auflagen**. Die Rechtefragen sind damit getrennt geführt, aber **nicht geklärt**; sie blockieren die in [RIGHTS_QUESTIONS](../tasks/RIGHTS_QUESTIONS.md) genannten Portierungen (u. a. Jazsync/MD4-Code, Icon-/Installer-Grafiken, JTattoo/junique/jshortcut). Der vollständige Linux-Generatorlauf von `run.sh` wurde ausgeführt und ist bestanden (siehe MIG-001 oben). Offen: macOS/BSD, Hardlinks, andere JDK-Hauptversionen. Keine Rust-Portierung von Fachlogik (nur Workspace und CLI-Basis, MIG-010), keine neue GUI, kein neuer Updater und keine finale Brandingbereinigung implementiert. Alte Java-Namen bleiben bis zur kontrollierten Ablösung bestehen. Die GUI-Toolkitwahl ist vorgeschlagen, nicht entschieden. GitHub-Branchschutz und Default-Branch-Wechsel wurden durch diese Dokumente nicht aktiviert.

## Nächster sinnvoller Schritt

MIG-011: gemeinsame Modelle, Ports, Fehler, Events und Jobzustände anhand eines kleinen In-Memory-Use-Cases fixieren (Prüffälle zuerst aus fachlichen Beispielen). Danach MIG-012 (GUI-Prototyp, ADR-0002) und MIG-013 (CI). Offen bleiben die Rechtefragen RQ-01..RQ-08 und die Plattformlücken (macOS/BSD, Hardlinks, FTP/FTPS/SFTP/TLS-Tests).

Aktive parallele Implementierungsaufträge: keine. Zuweisungen: [Task-Board](../tasks/README.md). Letzte integrierte Änderung: [LAST_CHANGE.md](../changes/LAST_CHANGE.md). Ausführlicher Übergabestand: [MIG-000](../handoffs/MIG-000.md).

## Offene Projektentscheidungen

GUI-Toolkit/Backend mit NVDA-/Orca-Nachweis; minimale OS-/Linux-Distributionen; Rechtebasis übernommener Implementierungen/Assets; Legacy-Schreibformat; Rust-Versionierung; finale Paketregistrierung; Lynxnode.it-Updatehosting und Signaturverantwortung. Diese Fragen blockieren nur die jeweiligen Gates, nicht die gesamte Vorbereitung.
