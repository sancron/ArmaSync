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
