# ADR-0014 — Upload in der CLI: Vorschau, Bestätigung, Zugangsdaten und Exitcodes (MIG-042, Schritt 4a)

Datum: 2026-10-25. Status: **angenommen** (2026-10-25) für den Umfang Schritt 4a (CLI: `upload plan` und `upload publish`). **Nicht Teil dieses ADR:** die GUI (eigener Task MIG-042-S4B), eine Fortschrittsanzeige pro Datei (eigener Task MIG-042-S4C, benötigt einen Ereigniskanal in der Veröffentlichung), Parallelität und Upload-Resume, das Legacy-Schreibformat (ADR-0003 bleibt gesperrt) sowie eine dauerhafte Speicherung von Zugangsdaten (ADR-0005 offen). Bezug: [CLI_CONTRACT](../migration/CLI_CONTRACT.md), [ADR-0012](0012-revisions-and-publication.md), [ADR-0013](0013-upload-exclusions.md), [ADR-0010](0010-ftp-sftp-webdav-contract.md).

## Kontext

Die Veröffentlichung ist ein Use Case in `armasync-application`. Die CLI und die spätere GUI sollen dieselbe Verdrahtung nutzen. Die Vorgaben: headless, `--no-input`, `--json`, stabile Exitcodes, Zugangsdaten nur über HTTPS und ohne Persistenz.

## Entscheidungen

1. **Verdrahtung im Workflow, nicht in der Frontend-Schicht.** `armasync-workflow::upload` prüft die Anfrage, baut die Adapter (`LocalDirectory`, `WebDavClient`) und ruft `upload::plan_with_rules` bzw. `revision::publish_with_rules` auf. Die CLI übersetzt nur Argumente und Ausgabe. Die GUI wird denselben Funktionen folgen.
2. **Zwei Befehle.**
   - `armasync upload plan --source DIR --target URL [--user NAME] [--password-env VARIABLE] [--exclude PFAD]…`: zeigt, was hochgeladen würde, und welche Dateien mit welchem Grund ausgeschlossen sind. Schreibt nichts auf das Ziel.
   - `armasync upload publish …` (dieselben Optionen) und `--yes`: veröffentlicht die Revision. **Ohne `--yes` wird nichts veröffentlicht.** Es gibt in dieser Version keine interaktive Rückfrage; die CLI wartet nie auf Eingaben, auch nicht ohne `--no-input`.
3. **Zugangsdaten.** Der Benutzername kommt als `--user`. Das Kennwort wird **nie als Argument** angenommen, sondern aus einer Umgebungsvariable gelesen, deren Name `--password-env` angibt (Standard `ARMASYNC_UPLOAD_PASSWORD`). Es wird nur gelesen, wenn ein Benutzername angegeben ist. Es wird nicht ausgegeben, nicht protokolliert und nicht gespeichert. Eine Adresse mit Benutzername, Kennwort, Abfrage oder Fragment wird abgelehnt (Exitcode 3), bevor eine Verbindung entsteht. Zugangsdaten gehen nur an HTTPS-Adressen.
4. **Exitcodes** nach CLI_CONTRACT: 0 Erfolg; 2 Aufrufs- oder Ausschlussfehler (unsicherer Pfad); 3 fehlende Zustimmung (`publish` ohne `--yes`), fehlende oder unzulässige Zugangsdaten, unzulässige Adresse; 4 Netzwerk, TLS oder HTTP-Status; 5 Quelle fehlt oder Dateisystem; 6 Integrität (Revision nicht verifizierbar); 7 Format; 8 nicht unterstützte Fähigkeit; 130 Abbruch.
5. **JSON** (`--json`): gleiche Hülle wie die übrigen Befehle (`schema_version`, `command` = `upload-plan` bzw. `upload-publish`, `operation_id`, `status`, `result` oder `error` mit `kind` und `message`). Das Ergebnis der Vorschau enthält `included`, `excluded` (mit `reason` und bei Benutzerregeln `rule`), `unused_rules` und Summen. Das Ergebnis der Veröffentlichung enthält Revision, Zählungen, Zeigerstatus und die Ausschlüsse.
6. **Abbruch.** Ctrl+C-Anbindung über den vorhandenen `CancelToken`. Ein Abbruch endet mit 130. Die Veröffentlichung setzt den Zeiger nur nach vollständiger Prüfung (ADR-0012), also bleibt die bisherige Veröffentlichung bei einem Abbruch unverändert und wird nicht als neue Revision markiert.
7. **Ehrliche Grenzen.** Die CLI zeigt in Schritt 4a **keinen Fortschritt pro Datei**: Die Veröffentlichung liefert noch keinen Ereigniskanal. Die Ausgabe erscheint am Ende. Das ist offen (MIG-042-S4C). Die Zugangsdaten-Eingabe über Umgebungsvariable ist eine Übergangslösung; eine Speicherung bleibt nach ADR-0005 offen.
8. **Keine Atomarität und keine Interoperabilität behauptet.** Die Veröffentlichung nutzt ADR-0012; WebDAV garantiert keine atomare Ersetzung, und echte Server sind nicht geprüft.

## Nicht im Scope

GUI (MIG-042-S4B); Fortschritt pro Datei (MIG-042-S4C); Parallelität; Upload-Resume; Legacy-Schreibformat; dauerhafte Zugangsdaten; Rückfrage (interaktive Bestätigung); Linux-/WSL-Läufe und GitHub-Actions (Projektregel).

## Belege

- Workflow: `crates/armasync-workflow/src/upload.rs`; Tests `crates/armasync-workflow/tests/upload_workflow.rs` (9 Tests, Loopback-Server aus `armasync-testkit`).
- CLI: `crates/armasync-cli/src/upload_cmd.rs`, `lib.rs` (Argumente und Dispatch); Tests `crates/armasync-cli/tests/upload_cli.rs` (8 Tests: Zustimmung, Adressen mit Zugangsdaten, fehlendes Kennwort, Ausschlüsse, Exitcodes 2/4/5/130, Hilfe).
- Prozesstest: `crates/armasync-cli/tests/upload_process.rs` (5 Tests; das gebaute Programm gegen den Loopback-Server über anonymes HTTP, ohne Test-Root und ohne TLS-Umgehung).
- Testkit: `crates/armasync-testkit/src/webdav.rs` (Loopback-WebDAV-Server, aus den Adapter-Tests hierher verschoben).
