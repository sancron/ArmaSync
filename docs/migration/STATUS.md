# Aktueller Migrationsstand

Aktualisiert: **2026-10-18**, Europe/Berlin. Diesen globalen Stand nur durch den jeweiligen Integrator ändern.

## Erreicht

- MIG-000: Projektanalyse, vollständiges Baseline-Dateiinventar, P0–P7-Roadmap, Architektur-/Daten-/Branding-/Lizenzstrategie, GUI-/CLI-Anforderungen und gemeinsame KI-Arbeitsregeln erstellt.
- Alle vier Branches lokal und auf `origin` vorhanden und verifiziert. `legacy`, `master` und `testing` unverändert auf `b7753cea18b2ac2cc57e29f926901997a4de8570`; Migrationspaket auf `development`. Branchaufgaben in [BRANCHING.md](../BRANCHING.md).
- Java-Hauptprojekt und Updater erfolgreich getestet: 64 Tests, 63 erfolgreich, 1 wegen fehlendem Releasearchiv übersprungen.
- AGPLv3-Referenztext für neue Komponenten aufgenommen; vorhandene GPL-Lizenz nicht ersetzt.

## Noch nicht erreicht

P0-Gate für Formate/Rechte/Transportmatrix ist offen. Kein Cargo-Workspace, keine Rust-Portierung, keine neue GUI, kein neuer Updater und keine finale Brandingbereinigung implementiert. Alte Java-Namen bleiben bis zur kontrollierten Ablösung bestehen. Die GUI-Toolkitwahl ist vorgeschlagen, nicht entschieden. GitHub-Branchschutz und Default-Branch-Wechsel wurden durch diese Dokumente nicht aktiviert.

## Nächster sinnvoller Schritt

Neuen Integrator und MIG-001-Eigentümer auf tatsächlicher `development`-SHA festlegen und vor paralleler Arbeit committen. MIG-001 erstellt synthetische Formatfixtures/Deskriptorinventar; MIG-002 kann separat die Rechte-/Assetprüfung beginnen, MIG-003 Transport-/Testmatrix. Danach P1, nicht direkt den gesamten Java-Baum löschen oder umbenennen.

Aktive parallele Implementierungsaufträge: keine. Zuweisungen: [Task-Board](../tasks/README.md). Letzte integrierte Änderung: [LAST_CHANGE.md](../changes/LAST_CHANGE.md). Ausführlicher Übergabestand: [MIG-000](../handoffs/MIG-000.md).

## Offene Projektentscheidungen

GUI-Toolkit/Backend mit NVDA-/Orca-Nachweis; minimale OS-/Linux-Distributionen; Rechtebasis übernommener Implementierungen/Assets; Legacy-Schreibformat; Rust-Versionierung; finale Paketregistrierung; Lynxnode.it-Updatehosting und Signaturverantwortung. Diese Fragen blockieren nur die jeweiligen Gates, nicht die gesamte Vorbereitung.
