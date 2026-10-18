# Aktueller Migrationsstand

Aktualisiert: **2026-10-18**, Europe/Berlin. Diesen globalen Stand nur durch den jeweiligen Integrator ändern.

## Erreicht

- MIG-000: Projektanalyse, vollständiges Baseline-Dateiinventar, P0–P7-Roadmap, Architektur-/Daten-/Branding-/Lizenzstrategie, GUI-/CLI-Anforderungen und gemeinsame KI-Arbeitsregeln erstellt.
- Alle vier Branches lokal und auf `origin` vorhanden und verifiziert. `legacy`, `master` und `testing` unverändert auf `b7753cea18b2ac2cc57e29f926901997a4de8570`; Migrationspaket auf `development`. Branchaufgaben in [BRANCHING.md](../BRANCHING.md).
- Java-Hauptprojekt und Updater erfolgreich getestet: 64 Tests, 63 erfolgreich, 1 wegen fehlendem Releasearchiv übersprungen.
- MIG-001 (Claude): lokal nach `development` integriert (Merge `de8c868`, nicht gepusht). Synthetische Fixtures, Generator, JVM-freier Verifier, Class-/UID-Inventar und Exportstrategie; Codex-Review freigegeben. Im zusammengesetzten Stand: `run.ps1` Exit 0, 23 Serialisierungen, 0 Probleme, Ergebnis byte-identisch zum eingecheckten Stand. Windows-Guard-Tests 4/4, Linux-Mountpoint-Guards 13/13 (WSL, root).
- AGPLv3-Referenztext für neue Komponenten aufgenommen; vorhandene GPL-Lizenz nicht ersetzt.

## Noch nicht erreicht

P0-Gate ist offen: Formate (MIG-001) sind integriert, Rechte (MIG-002) und Transportmatrix (MIG-003) fehlen. **Der vollständige Linux-Generatorlauf von `run.sh` (gradle/javac/java/Verifier) ist offen**, da in WSL-Ubuntu kein JDK vorhanden ist; unter Linux sind nur die Guards bis vor den Build geprüft. Ebenso offen: macOS/BSD, Hardlinks. Kein Cargo-Workspace, keine Rust-Portierung, keine neue GUI, kein neuer Updater und keine finale Brandingbereinigung implementiert. Alte Java-Namen bleiben bis zur kontrollierten Ablösung bestehen. Die GUI-Toolkitwahl ist vorgeschlagen, nicht entschieden. GitHub-Branchschutz und Default-Branch-Wechsel wurden durch diese Dokumente nicht aktiviert.

## Nächster sinnvoller Schritt

Codex bearbeitet MIG-002 (Rechte-/Asset-/JAR-Inventur) und MIG-003 (Transport-/Plattformmatrix, Testserver) in eigenen Worktrees; danach Integration und P0-Gate-Prüfung, erst dann MIG-010. Nicht direkt den gesamten Java-Baum löschen oder umbenennen.

Aktive parallele Implementierungsaufträge: MIG-002 und MIG-003 (Codex, reserviert). Zuweisungen: [Task-Board](../tasks/README.md). Letzte integrierte Änderung: [LAST_CHANGE.md](../changes/LAST_CHANGE.md). Ausführlicher Übergabestand: [MIG-000](../handoffs/MIG-000.md).

## Offene Projektentscheidungen

GUI-Toolkit/Backend mit NVDA-/Orca-Nachweis; minimale OS-/Linux-Distributionen; Rechtebasis übernommener Implementierungen/Assets; Legacy-Schreibformat; Rust-Versionierung; finale Paketregistrierung; Lynxnode.it-Updatehosting und Signaturverantwortung. Diese Fragen blockieren nur die jeweiligen Gates, nicht die gesamte Vorbereitung.
