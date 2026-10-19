# Geplanter CLI-Vertrag

Entwurf für P1/P3, **noch nicht implementiert**. Die GUI bleibt der Hauptzugang; `armasync` ist eine separate CLI ohne Displayserver, GUI-Initialisierung oder GUI-Systembibliotheken. Subcommands rufen dieselben Application-Use-Cases auf.

## Befehlsstruktur

```text
armasync --version
armasync repository list [--json]
armasync repository add <name> --url <url>
armasync repository import <export-or-autoconfig>
armasync repository check <id>
armasync repository build <id>
armasync repository upload <id>
armasync sync plan <id> --destination <path> [--json]
armasync sync run <id> --destination <path> [--dry-run] [--delete-extra]
armasync verify <id> --destination <path>
armasync profile list
armasync profile import <export>
armasync launch <profile-id>          # Windows-Capability
armasync tools extract-bikeys --source <path> --destination <path>
armasync update check
armasync update apply
armasync diagnostics
armasync license
```

Global: `--config <path>`, `--portable`, `--json`, `--no-input`, `--verbose`, `--quiet`, `--no-color`. Gültigkeit/Kombination pro Befehl in P1 mit Help-Snapshots fixieren. Updateanwenden nur bei tatsächlich freigegebenem Updater, sonst definierte Meldung. Keine Passwörter als Commandline-Flags.

## Umgesetzt: `armasync import` (MIG-021)

```text
armasync import <neutral-export> [--apply] [--on-conflict keep-both|fail|replace] [--config <dir>] [--json] [--quiet]
```

Importiert Repositories und Profile aus dem neutralen Format (`armasync.import`, [IMPORT_SCHEMA](IMPORT_SCHEMA.md)). Ohne `--apply` nur Vorschau, ohne Schreibzugriff. Wartet nie auf Eingaben; kein Geheimnis in Ein- oder Ausgabe. Exitcodes laut Tabelle unten (2 Aufruf, 3 Konflikt unter `fail`, 5 Dateisystem, 7 Format). Die Zeile `profile import` oben bleibt offen. Nach dem Import erklärt die Ausgabe in „What this import covers:“ (und in `--json` als `scope_notices`), was nicht enthalten ist ([SERVER_METADATA_IMPORT](SERVER_METADATA_IMPORT.md)).

## Umgesetzt: `armasync repository import` (MIG-022)

```text
armasync repository import <autoconfig-datei> [--apply] [--on-conflict keep-both|fail|replace] [--config <dir>] [--json] [--quiet]
```

Legt ein Repository aus der von einem Server veröffentlichten Verbindungsbeschreibung (`.a3s/autoconfig`, **lokale Datei**, kein Download) an: nur Name und Adresse, nie Kennwörter; verschlüsselte Repositorydateien, Einstellungen und Addon-Listen sind nicht betroffen. Ohne `--apply` nur Vorschau. Exitcodes wie `import`, zusätzlich 8 für einen Transport, der kein Repository-Transport ist (z. B. Proxy). `--json`: `command` = `repository-import`. Siehe [SERVER_METADATA_IMPORT](SERVER_METADATA_IMPORT.md).

## Umgesetzt: `repository list`, `profile list` (MIG-023)

```text
armasync repository list [--config <dir>] [--json] [--quiet]
armasync profile list    [--config <dir>] [--json] [--quiet]
```

Zeigen den Bestand aus `library.json` rein lesend (nichts wird geschrieben oder angelegt; fehlende Bibliothek = leer, beschädigte = Exitcode 7 ohne Änderung). Pro Repository `id`, `name`, `url`, `unencrypted`, `credentials_required` (nie Zugangsdaten), pro Profil `id`, `name`, `repositories` (geordnete IDs). JSON: `command` = `repository-list` bzw. `profile-list`. Damit ist der CLI-Teil des P2-Gates („importierte Repositories anzeigen“) erfüllt; die GUI-Anzeige folgt mit MIG-031.

## Ausgabevertrag

Menschenlesbare Fortschritte/Diagnosen nach stderr, strukturiertes Ergebnis nach stdout. `--json`: versionierter Ergebnis-Envelope mit `schema_version`, `operation_id`, `status`, `result` oder typisiertem `error`; niemals Secrets. Für lange Events optional separat dokumentiertes NDJSON-Format, nicht still mehrere JSON-Dokumente in eine Ausgabe mischen. Lokalisierte Meldungen dürfen stabile Fehlercodes nicht verändern.

Nicht-TTY/`--no-input` darf nie auf Eingaben warten. Fehlende Credentials oder notwendige Zustimmung mit eindeutigem Fehler beenden. Credential-Eingabe per sicherem Prompt bzw. expliziter Credential-Quelle; kein Kennwort in History/Logs. Kein versteckter Import oder Löschdefault.

## Vorgeschlagene Exitcodes

| Code | Bedeutung |
| --- | --- |
| 0 | erfolgreich |
| 2 | Argument-/Benutzungsfehler |
| 3 | Konfiguration, fehlende Zustimmung oder Credentials |
| 4 | Netzwerk, Authentifizierung oder Vertrauensfehler |
| 5 | Dateisystem / Rechte / Speicherplatz |
| 6 | Integritäts-/Verifikationsfehler |
| 7 | Format-/Migrationsfehler |
| 8 | nicht unterstützte Plattform-/Transportfähigkeit |
| 130 | expliziter Abbruch / Ctrl+C |

Vor P1-Vertragsfreeze Präzedenz gemischter Fehler und Unterschied erwarteter Statuswerte (z. B. „Update verfügbar“) festlegen. Alte `System.exit(0)`-Fehler nicht übernehmen.

## Altsyntax und sichere Änderungen

`-build` → `repository build`, `-check` → `repository check`, `-sync` → `sync run`, `-extract` → `tools extract-bikeys`, `-update` → `update check/apply` mit expliziter Bedeutung. Interaktive Console-Kommandos und bisherige Automationsaufrufe inventarisieren; ein etwaiger Adapter ist dokumentiert und befristet, keine alte Produktidentität.

`--dry-run` führt keine Dateiänderung aus. `--delete-extra` ist zusätzlich explizit; Bestätigung/`--no-input`-Zustimmung für den konkreten Plan festlegen. Vor Anwendung Root, Revision und Zielzustand neu prüfen. Niemals fremde Dateien außerhalb des verwalteten Bereichs löschen. Gleichzeitige Jobs auf dasselbe Ziel über Lock/Jobjournal koordinieren; unterschiedliche Repositories/CLI-Jobs nicht durch einen globalen GUI-Single-Instance-Lock blockieren.
