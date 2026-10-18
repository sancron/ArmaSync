# Neutrales Importformat und Offline-Export (MIG-021)

Stand: 2026-10-18. Umsetzung: `crates/armasync-adapters/src/import/`, CLI `armasync import`, Exporter `migration-tools/legacy-export/export.py`, Golden-Dateien `fixtures/neutral-import/`. Rahmen: [COMPATIBILITY](COMPATIBILITY.md) („Lokaler Migrationsweg“), [ADR-0003](../adr/0003-legacy-formats.md), [DATA_MODEL](DATA_MODEL.md), [FORMAT_INVENTORY](FORMAT_INVENTORY.md). Es werden **keine Rechte- oder Übernahmefragen entschieden** (RQ-01..RQ-08 unverändert offen).

## Ablauf

```text
Altverzeichnis ──(1) export.py, nur lesend──▶ export.json (armasync.import) ──(2) armasync import ──▶ library.json
                                                                   Vorschau → --apply (Backup, atomarer Commit, Journal)
```

1. **Export (offline, einmalig, Python, nur Standardbibliothek).** Liest das Altverzeichnis ausschließlich lesend (jede Datei mit `open(..., 'rb')`; keine Verschiebung, Änderung oder Löschung), wertet mit dem begrenzten, JVM-freien Leser aus MIG-001 (`verify.py`, importiert, nicht geändert) die **unverschlüsselten** Dateien aus und schreibt das neutrale Dokument. Die Ausgabe wird nie überschrieben und darf nicht im Altverzeichnis liegen. Exitcodes: 0 Erfolg; 2 Aufruf-/E/A-Problem (nichts geschrieben); 3 geschrieben, aber unlesbare Dateien übersprungen.
2. **Import (Rust).** `armasync import <datei>` zeigt standardmäßig nur die Vorschau (nichts wird geschrieben, auch kein Verzeichnis). `--apply` führt aus. Es gibt keine Eingabeaufforderung und keinen versteckten Import.

## Was exportiert wird und was nicht

| Quelle | Export |
| --- | --- |
| `*.a3s.profile`, Profile aus `export.a3s.autoconfig` | **ja**: Anzeigename und die geordnete, eindeutige Liste der Repository-**Namen**, auf die der Profilbaum verweist |
| `*.a3s.repository` (verschlüsselt) | **nein**: nur als Hinweis `skipped-encrypted-repository` mit Namen aus dem Dateinamen. Das Entschlüsseln ist eine offene Entscheidung (ADR-0003, Punkt „verschlüsselt“; Schlüsselquelle und Rechtefrage RQ-01). Dieses Werkzeug trägt oder verwendet den Altschlüssel nicht. |
| Favoritenserver, Kennwörter, Logins, URLs mit Zugangsdaten | **nie**; Favoritenserver nur als Zähler (`skipped-favorite-servers`) |
| `a3s.cfg`, `a3s.prefs` | nein, nur Zähler `skipped-settings` (keine Abbildung auf `settings.toml`; Lookandfeel, Frequenzen u. ä. sind keine ArmaSync-Einstellungen) |
| Profilinhalte (Startoptionen, Addon-Listen, Prioritäten, Pfade) | nein; MIG-050 |
| Server-Metadaten (`.a3s/*`) | nein; MIG-022 |

Der Exporter lehnt Streams ab, deren Klassenversion (`serialVersionUID`) für `Profile`, `TreeDirectory`, `TreeLeaf` von der Tabelle in `export.py` (aus FORMAT_INVENTORY, Abschnitt 3) abweicht; unlesbare, zu große, falsch typisierte oder beschädigte Dateien werden mit einem Hinweis übersprungen und brechen den Lauf nicht ab. Die alten Klassen- und Feldnamen sind **Wire-Daten der eingefrorenen Baseline** und bleiben im Exporter; sie erscheinen nicht im neutralen Dokument. Profile mit Namen, die nicht dem Anzeigenamen-Format entsprechen (leer, > 120 Zeichen, Steuerzeichen), werden übersprungen (`skipped-invalid-profile-name`).

Eine Folge: Der Export aus einem echten Altverzeichnis enthält heute **Profile ohne Repository-Definitionen**. Die Repositories kommen später aus einer der beiden offenen Quellen (Entschlüsselung des lokalen Bestands oder MIG-022: Autoconfig der Server) oder werden von Hand ergänzt; der Import ist darauf ausgelegt (siehe „Wiederholung“).

## Schema `armasync.import`, Version 1

```json
{
  "kind": "armasync.import",
  "schema_version": 1,
  "source": { "tool": "armasync-legacy-export", "tool_version": "1", "format": "legacy-local-files" },
  "repositories": [
    { "key": "Example HTTP", "name": "Example HTTP", "url": "https://repo.example.invalid/arma", "needs_credentials": true }
  ],
  "profiles": [ { "name": "Alpha", "repositories": ["Example HTTP"] } ],
  "notes": [ { "code": "skipped-encrypted-repository", "subject": "Example-Http", "count": 1 } ]
}
```

* `source`: Herkunft (je 1–120 Zeichen, nur Anzeige und Journal). `key`: stabiler Schlüssel der Quelle (1–200 Zeichen, eindeutig); Profile verweisen darauf. `url`: wie in `library.json` (nur http/https/ftp/ftps/sftp; keine Zugangsdaten, kein Query, kein Fragment). `needs_credentials`: nur ein Hinweis, dass die Quelle Zugangsdaten nutzte; der Import legt einen `CredentialRef` an, **kein** Geheimnis. `notes`: Hinweise des Exporters (`code` `[a-z0-9-]{1,40}`, optionale `subject`-Bezeichnung, `count`).
* **Es gibt kein Feld für Kennwort, Login, Token oder Schlüssel.** Zusätzlich lehnt der Leser jedes Dokument ab, in dem irgendwo ein Feldname `password`, `passwort`, `passwd`, `login`, `secret`, `token`, `apikey` oder `credential` enthält (Ausnahme `needs_credentials`), ohne den Wert auszugeben.
* Abgelehnt werden außerdem: falsches `kind`, fehlende/unbekannte/neuere Version (neuere mit Hinweis), unbekannte Felder, doppelte Schlüssel oder Profilnamen, doppelte Verweise in einem Profil, ungültige Namen/URLs, mehr als 5000 Repositories/1000 Profile/1000 Notizen, Dateien über 4 MiB, JSON tiefer als 128 Ebenen. Meldungen enthalten nicht die abgelehnten Werte.

## Planung und Konflikte

Die Planung ist eine reine Funktion (bestehende Bibliothek, Dokument, Richtlinie) und **deterministisch**:

* **ID** = ASCII-Slug des `key` (Kleinbuchstaben/Ziffern, sonst `-`, höchstens 48 Zeichen; Umlaute werden zu Trennzeichen); bei Profilen des Namens.
* **Repository** mit gleicher Adresse (Schema/Host ohne Groß-/Kleinschreibung, ohne abschließenden `/`) wie ein vorhandenes: *unverändert* (ein abweichender Name erzeugt eine Warnung; mit `replace` wird der Name übernommen).
* Sonst, wenn die ID frei ist: *hinzufügen*. Ist die ID von einem **anderen** Repository belegt: `keep-both` (Standard) fügt unter `<id>-2`, `-3` … hinzu und fasst Bestehendes nicht an; `fail` bricht ohne Änderung ab (Exitcode 3); `replace` überschreibt den Eintrag mit dieser ID (in der Vorschau als *replace* sichtbar).
* **Profil**: gleicher Name und gleiche (aufgelöste) Repositoryliste → *unverändert*; sonst wie bei Repositories.
* **Auflösung der Verweise**: Schlüssel aus dem Dokument, dann Bibliothekseintrag mit der ID = Slug des Schlüssels, dann eindeutiger Bibliothekseintrag mit gleichem Namen. Nicht auflösbare Verweise werden aus dem Profil weggelassen und als Warnung gemeldet.
* Warnungen: nicht auflösbarer Verweis, Zugangsdaten nötig, unverschlüsselte Adresse (`http`/`ftp`), abweichender Name.

**Wiederholung:** Dasselbe Dokument ein zweites Mal ergibt keine Änderung (kein Backup, kein Journaleintrag, Datei unverändert). Ein später ergänztes Dokument mit den fehlenden Repositories löst die Verweise; mit `--on-conflict replace` aktualisiert ein erneuter Import des Profils die Liste (ohne `replace` entsteht `alpha-2` neben dem alten Profil).

## Anwenden, Backup, Abbruch, Journal

`apply` plant erneut gegen die **aktuelle** Bibliothek (ADR-0004: Revalidierung), prüft den Abbruch, legt eine Kopie der bisherigen Bibliothek als `library.pre-import-<unixzeit>.json.bak` ab, schreibt das Ergebnis mit **einem** atomaren Commit (`LibraryStore`, zusätzlich dessen `.bak`) und hängt einen Eintrag an `import-journal.json` (höchstens 100 Einträge; nur Zähler, SHA-256 der Importdatei, Werkzeug, Backupname; **keine Namen, Adressen oder Geheimnisse**).

* Abbruch (`CancelToken`) bis zum Beginn des Commits lässt Bibliothek, Backups und Journal unverändert (Test). Die CLI verdrahtet Strg+C noch nicht (Exitcode 130 ist reserviert).
* Eine beschädigte bestehende Bibliothek wird nie überschrieben: Vorschau und Anwenden enden mit Formatfehler (Exitcode 7).
* Scheitert der Commit, bleibt die Bibliothek unverändert und kein Journaleintrag entsteht; die bereits angelegte Vorab-Sicherung bleibt liegen. Ein defektes Journal wird gemeldet und nicht überschrieben; der Import selbst gilt dann als abgeschlossen.
* Rückkehr: Sicherung zurückkopieren; die Altdaten wurden nie berührt.

## CLI

`armasync import <datei> [--apply] [--on-conflict keep-both|fail|replace] [--config <verzeichnis>] [--json] [--quiet]`. Standardverzeichnis: ArmaSync-Config-Root der Plattform ([DATA_MODEL](DATA_MODEL.md)). Exitcodes: 0 Erfolg (auch Vorschau); 2 Aufruf (fehlende Datei, ungültiger Wert); 3 Konfiguration/Konflikt unter `fail`; 5 Dateisystem; 7 ungültige Importdatei oder beschädigte Bibliothek; 130 reserviert. `--json` liefert den versionierten Envelope (`status` `ok` oder `error`, Zähler, Einzelergebnisse, Warnungen, Notizen, SHA-256, `mode`, `changed`, `backup_file`), nie Geheimnisse.

Die Vertragsbefehle `repository import` und `profile import` ([CLI_CONTRACT](CLI_CONTRACT.md)) bleiben für den Direktimport aus Autoconfig/Server (MIG-022) vorgesehen; `armasync import` ist der Weg für das neutrale Format.

## Nicht geprüft / offen

* Keine realen Nutzerdateien und keine großen Altverzeichnisse (bewusst); die Grenzen des Lesers sind Fixture-Werte (8 MiB entpackt).
* Der echte Altbestand wurde mit **keiner** echten Anwendung gegengeprüft; die Zuordnung der Repositoryverweise stützt sich auf die Fixtures (`modsetRepositoryName` in Profilbäumen vom Typ `REPOSITORY`).
* Entschlüsselung lokaler Repositorydateien, Profilinhalte, Einstellungen, Favoritenserver: ausdrücklich nicht übernommen.
* Windows-Dateisperren und Absturz mitten im Commit nicht getestet (wie MIG-020); keine Sperre gegen zwei gleichzeitige Importe.
* Strg+C-Anbindung der CLI, GUI-Anbindung (MIG-031).
