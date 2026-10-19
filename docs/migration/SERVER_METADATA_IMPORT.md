# Import von Server-Metadaten (MIG-022)

Stand: 2026-10-19. Umsetzung: Crate `crates/armasync-compat` (Parser), `crates/armasync-adapters/src/import/{server,notice}.rs`, CLI `armasync repository import`. Rahmen: [COMPATIBILITY](COMPATIBILITY.md) („Direkte Nutzung vorhandener Repositoryserver“), [ADR-0003](../adr/0003-legacy-formats.md), [FORMAT_INVENTORY](FORMAT_INVENTORY.md), [IMPORT_SCHEMA](IMPORT_SCHEMA.md). Es werden **keine Rechte- oder Übernahmefragen entschieden** (RQ-01..RQ-08 unverändert offen).

## Was MIG-022 ist – und was nicht

MIG-022 liest die **öffentlich vom Server veröffentlichte** Verbindungsbeschreibung eines Repositorys (`.a3s/autoconfig`) mit einem begrenzten, datenorientierten Parser und legt daraus ein Repository im neuen Bestand an (Name und Adresse, ohne Zugangsdaten).

MIG-022 ist **keine Lösung** für:

- die **fehlende Entschlüsselung** der lokalen `.a3s.repository`-Dateien (ADR-0003, Schlüsselquelle und Rechtefrage RQ-01 bleiben offen; der Importer entschlüsselt nichts und trägt keinen Altschlüssel);
- **Einstellungen** (`a3s.cfg`, `a3s.prefs`) und **Addon-Listen/Profilinhalte** (MIG-050/053);
- das Lesen der Sync-Daten (`sync`, `serverinfo`, `changelogs`, `events`): sie gehören zu Prüfung und Synchronisation (ab MIG-030) und werden hier nicht gelesen.

## Welche Repositoryinformation woher stammt

| Information | Öffentliche Server-Metadaten (`.a3s/autoconfig`) | Nur in verschlüsselten `.a3s.repository`-Dateien bzw. lokal |
| --- | --- | --- |
| Repository-Name | **ja** (importiert) | ja (Quelle für Profilverweise) |
| Transport und Adresse (Host, Pfad, Port) | **ja** (importiert) | ja |
| „Zugangsdaten nötig“ | als Hinweis `needs_credentials` (nur ob vorhanden; **Werte werden nie gelesen**) | Kennwörter im Klartext nach dem Entschlüsseln (nicht importiert) |
| Favoriten-Spieleserver | in der Datei vorhanden, **nur gezählt**, nicht importiert | ja |
| Lokaler Download-Ordner | nein | **ja** |
| Ausschlüsse (Build/Sync), Dateizuordnungen | nein | **ja** |
| Upload-Protokoll und -Einstellungen, Verbindungs-/Geschwindigkeitslimits | nein | **ja** |
| Ereignis-Download-Ordner, Benachrichtigung, weitere Nutzereinstellungen | nein | **ja** |
| Programmeinstellungen, Profile, Addon-Listen | nein | nein (eigene Dateien; MIG-050/053 bzw. MIG-021 für Profilnamen) |

Ein erfolgreicher MIG-022-Import ersetzt deshalb **nie** die lokalen Repositorydateien: wer Einstellungen nur dort hatte, muss sie neu setzen oder später über die offene Entschlüsselungsentscheidung übernehmen.

## Ablauf und Grenzen

```text
.a3s/autoconfig (lokale Datei) ──armasync-compat──▶ ServerAutoConfig ──▶ ImportDocument (armasync.import)
                                                                      ──▶ Importer: Vorschau → --apply (Backup, atomarer Commit, Journal)
```

- **Quelle:** eine **lokale Datei**. MIG-022 führt **keinen Netzwerkzugriff** aus (kein Abruf vom Server, kein TLS-/Proxy-Verhalten); der Abruf ist Aufgabe der Transporte ab P3/P4 ([TRANSPORT_MATRIX](TRANSPORT_MATRIX.md)). Der Parser nimmt Bytes entgegen, ein späterer Transport kann sie ihm geben. Das ist ein *begrenzter* Direktimport, kein vollständiger.
- **Parser (`armasync-compat`):** liest das GZIP-komprimierte Serialisierungsformat der Altserver ohne JVM, ohne Klassenladen und ohne Ausführen von etwas, das im Stream genannt wird; keine I/O, keine Netzwerkabhängigkeit. Geprüft und begrenzt: Eingabe ≤ 1 MiB, entpackt ≤ 4 MiB (`Limits::AUTOCONFIG`), Verschachtelung ≤ 100, ≤ 50 000 Objekte/Elemente, Arraylängen gegen den Reststream **vor** der Allokation, GZIP-Prüfsumme (bis zum Ende gelesen), keine Daten nach dem Stream, Rückverweise nur auf vorhandene Werte, zyklische Klassenketten abgelehnt, genau ein Top-Level-Objekt.
- **Bekannte Klassen:** Der Stream darf nur die in `autoconfig.rs` tabellierten Deskriptoren (Name und Versionsnummer) enthalten; jede andere Klasse oder Version wird mit `UnknownClass` abgelehnt, nichts wird geladen oder geraten. Diese Namen sind **Wire-Daten** des bestehenden Formats (ADR-0003) und stehen nur dort, mit Erklärung und Tests; sie sind keine Typnamen dieses Projekts.
- **Transporte:** FTP, FTPS, SFTP, HTTP, HTTPS und die WebDAV-Varianten werden zu einer Adresse mit Schema `ftp`/`ftps`/`sftp`/`http`/`https`. Bei WebDAV wird nur die Webadresse übernommen (Hinweis `server-webdav-address-only`; ArmaSync nutzt WebDAV noch nicht). Proxy-Typen (SOCKS) und unbekannte Transporttypen werden mit Exitcode 8 (*nicht unterstützt*) abgelehnt, nicht geraten.
- **Adresse:** Host und optionaler Pfad aus dem Feld `url`, Port aus dem Feld `port` (weggelassen, wenn er dem Standardport des Transports entspricht). Abgelehnt werden `@`, `\`, `?`, `#`, `%`, Leerzeichen/Steuerzeichen, ein Schema im Feld, ein Port im Hostteil (außer IPv6-Literal in `[...]`), Ports außerhalb 1–65535 und nicht numerische Ports – ohne Wert in der Meldung.
- **Geheimnisse:** Felder, deren Name `password`, `passwd`, `passwort`, `login`, `secret`, `token`, `credential` oder `key` enthält, werden beim Lesen **nie als Text gehalten**: die Bytes werden übersprungen, gemerkt wird nur „vorhanden/leer“. Fehlermeldungen enthalten nie Daten aus der Datei (feste Texte). Im Ergebnis, in `Debug`-Ausgaben, `library.json`, Sicherung, Journal, JSON und Logs erscheint kein Geheimnis (Tests prüfen die Fixture-Platzhalter dort).
- **Anwendung:** der vorhandene Importer (MIG-021): Vorschau ohne Schreibzugriff, deterministische ID aus dem Namen, Konfliktrichtlinien `keep-both`/`fail`/`replace`, gleiche Adresse = *unverändert* (Wiederholung ändert nichts), Vorab-Sicherung, ein atomarer Commit, Journal ohne Namen/Adressen. Herkunft im Journal: `armasync-server-metadata`.

## CLI

```text
armasync repository import <autoconfig-datei> [--apply] [--on-conflict keep-both|fail|replace] [--config <dir>] [--json] [--quiet]
```

Ohne `--apply` nur Vorschau (nichts wird geschrieben). Exitcodes wie `armasync import`: 0, 2 (fehlende Datei/Aufruf), 3 (Konflikt unter `fail`), 5 (Dateisystem), 7 (ungültige oder nicht unterstützte Datei), 8 (Transport kein Repository-Transport). Gibt man eine neutrale Importdatei an, verweist die Meldung auf `armasync import`. JSON: `command` = `repository-import`, sonst derselbe Envelope.

## Hinweise zum Umfang (CLI und GUI)

Beide begrenzten Importwege nennen verständlich, was sie **nicht** enthalten. Quelle ist `armasync_adapters::import::notice::scope_notices` mit stabilen Codes, Schweregrad (`info`/`limitation`) und Texten auf Englisch und Deutsch; die CLI druckt den englischen Text unter „What this import covers:“ (nicht bei `--quiet`) und liefert in `--json` das Feld `scope_notices` (`code`, `severity`, `text`, `text_de`) – ein additives Feld; `schema_version` bleibt 1.

| Code | Gilt für | Schweregrad | Inhalt (Kurzfassung) |
| --- | --- | --- | --- |
| `local-import-scope` | MIG-021 (`armasync import` mit Export des Offline-Exporters) | limitation | Nur Profilnamen und Repository-Namen; keine Adressen, Einstellungen, Addon-Listen, Startoptionen, Kennwörter |
| `encrypted-repositories-not-read` | MIG-021, wenn verschlüsselte Repositorydateien übersprungen wurden | limitation | `.a3s.repository` verschlüsselt, nicht gelesen; Adressen und lokale Einstellungen fehlen; Entscheidung offen; Originale unverändert |
| `profile-repositories-missing` | MIG-021, wenn Verweise ausgelassen wurden | limitation | Repositories hinzufügen, Profildatei erneut importieren, vorhandenes Profil ersetzen |
| `server-metadata-scope` | MIG-022 | info | Nur Name und Adresse; Kennwörter werden nie gelesen |
| `server-metadata-not-local-settings` | MIG-022 | limitation | Ersetzt eigene Repositorydateien nicht; lokale Einstellungen, Programmeinstellungen, Profile, Addon-Listen nicht importiert |
| `server-favorite-servers-skipped` | MIG-022, wenn die Datei Favoritenserver enthält | info | Nicht importiert |
| `server-webdav-address-only` | MIG-022 bei WebDAV | info | Nur Webadresse; WebDAV noch nicht genutzt |

Vorgaben für die GUI (Umsetzung erst in MIG-031; hier nur vorbereitet, keine GUI-Änderung): die Hinweise vor der Bestätigung des Imports und erneut im Ergebnis zeigen; als normaler Text im Lesefluss (Screenreader-erreichbar, Tastaturfokus auf den Block möglich), Schweregrad nie nur über Farbe, sondern über ein Textpräfix („Hinweis“/„Einschränkung“); Codes für Tests und Support, nicht als Anzeigetext; Texte über die vorhandene Sprachwahl (Deutsch/Englisch) auswählen. Die Wortlaute sind Daten und können angepasst werden, die Codes sind stabil.

## Tests (belegt)

`armasync-compat` (16 Tests, `tests/autoconfig.rs`): alle drei MIG-001-Server-Fixtures, alle 15 ungültigen Fixtures abgelehnt (mit dokumentiertem Grund je Fixture), andere Metadaten-/Lokaldateien nicht als Autoconfig akzeptiert, alle Transporte (7 unterstützte, SOCKS/A3S/Klassen-Typ-Mismatch abgelehnt), Adress-/Port-/Namensvalidierung, unbekannte Klasse und abweichende Version, hängender Rückverweis, Limits, Nachlaufdaten, **jede Verkürzung** eines gültigen Streams abgelehnt und jede Byte-Veränderung ohne Absturz, Geheimnisfreiheit in Ergebnis, `Debug` und Fehlern. Adapter (`tests/server_import.rs`, 6 Tests und Unit): Vorschau schreibt nichts; Anwenden, Wiederholung, Backup, Journal; keine Fixture-Geheimnisse in einer geschriebenen Datei; Proxy-Typ → `Unsupported` ohne Schreiben; alle ungültigen Fixtures → `Format` ohne Änderung; Abbruch. CLI (`tests/repository_import.rs`, 7 Tests): Vorschau/Anwenden, JSON, `--quiet`, Fehler und Exitcodes, Hilfe, Umfangs-Hinweise für beide Importwege.

## Nicht geprüft / offen

- **Keine echten Server und keine realen Nutzerdateien** (bewusst). Es existieren nur synthetische Fixtures aus MIG-001, erzeugt mit der Baseline; reale Dateien älterer Versionen (Klassenevolution) wurden nicht gesehen.
- **Ohne Fixture belegt, nur aus den Baseline-Quellen abgeleitet:** die Versionsnummern der Klassen für FTP/FTPS und WebDAV sowie der Aufzählungstyp für den veralteten Verschlüsselungsmodus (Fixtures enthalten nur HTTPS und SFTP); Tests dafür verwenden vom Test selbst geschriebene Streams. Wenn ein echter Server hier abweicht, wird er mit `UnknownClass` abgelehnt (sicher, aber nicht importiert).
- Das Feld `url` mit eingebettetem Port (`host:port`) wird abgelehnt, weil nicht belegt ist, ob Altdaten es so speichern; ebenso URL-codierte Zeichen.
- Kein Abruf vom Server, kein TLS/Proxy; kein `serverinfo`/`sync`/`changelogs`/`events`; kein Lesen von `export.a3s.autoconfig`. Letztere Datei ist ein **unverschlüsselter** Konfigurationsexport der Altanwendung mit vollständigen Repository-Objekten und enthält Kennwörter; sie wäre eine mögliche, aber getrennt zu entscheidende Quelle für Repositorydefinitionen (Redaktion zwingend; Rechtefragen unberührt) – nicht Teil von MIG-022.
- Keine Rechtefreigabe: Der Parser ist nach der öffentlich dokumentierten Streamstruktur und den Fixtures neu geschrieben; es wurde keine Altquelle übernommen. RQ-01..RQ-08 bleiben offen.
- Die Dateiendung `.a3s.repository` und die Klassen-/Feldnamen der Wire-Tabelle sind eng begrenzte Format-Ausnahmen (BRANDING); sie erscheinen nur in dieser Dokumentation, im Hinweistext für die Nutzer (Name der eigenen Dateien) und in der Tabelle von `autoconfig.rs`.
- **P1-GUI-Gate unverändert offen** (NVDA, Orca, Sichtprüfung des hellen Themes nicht ausgeführt). MIG-022 berührt die GUI nicht.
