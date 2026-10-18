# Datenmodell, Stores, Pfade und Credentials (MIG-020)

Stand: 2026-10-18, implementiert in `armasync-core` (Modell, Credential-Port) und `armasync-adapters` (Dokumente, Pfade, atomares Schreiben). Dies ist das **neue, neutrale** Format von ArmaSync. Es enthält keine Legacy-Feldnamen, -Klassen, Standardserver oder Altwerte; die Übernahme alter Daten ist MIG-021/022 ([COMPATIBILITY](COMPATIBILITY.md), [ADR-0003](../adr/0003-legacy-formats.md)).

## Dokumente

Zwei Dokumente im Config-Root. Beide beginnen mit `kind` und `schema_version`; ein Leser prüft diese **zuerst**.

| Datei | Format | `kind` | Inhalt |
| --- | --- | --- | --- |
| `library.json` | JSON (UTF-8, eingerückt, endet mit Zeilenumbruch) | `armasync.library` | Repository-Definitionen und Profile in einem Dokument |
| `settings.toml` | TOML | `armasync.settings` | Einstellungen |

Ein gemeinsames Dokument für Repositories und Profile ist Absicht: Profile verweisen auf Repository-IDs, und eine Datei lässt sich atomar ersetzen. Zwei Dateien könnten nach einem Absturz inkonsistent zueinander sein.

### `library.json` (Schema 1)

```json
{
  "kind": "armasync.library",
  "schema_version": 1,
  "repositories": [
    { "id": "main", "name": "Hauptrepository", "url": "https://repo.example/arma", "credential": "cred-main" }
  ],
  "profiles": [
    { "id": "default", "name": "Standard", "repositories": ["main"] }
  ]
}
```

* `id`: stabiler Schlüssel `[a-z0-9_-]{1,64}` (nie der Anzeigename). `name`: 1–120 Zeichen, keine Steuerzeichen, keine Leerzeichen am Rand.
* `url`: `http`, `https`, `ftp`, `ftps` oder `sftp`; **ohne** Benutzer/Kennwort (`user:pw@`), Query oder Fragment; gültiger Host und Port; höchstens 2048 Bytes. Das Schema wird kleingeschrieben gespeichert. Ob ein Transport implementiert ist, regelt P3/P4, nicht das Modell.
* `credential` (optional): `CredentialRef` `[a-z0-9_-]{1,64}`, nur ein Verweis. **Kennwörter, Token und Benutzernamen stehen nie in der Datei.**
* `profiles[].repositories`: geordnete Liste bekannter Repository-IDs, jede höchstens einmal. Weitere Profilinhalte (Gruppen, Prioritäten, Startoptionen) sind MIG-050 und erfordern eine neue `schema_version`.
* Reihenfolge der Listen bleibt erhalten. Höchstens 5000 Repositories, 1000 Profile.

### `settings.toml` (Schema 1)

```toml
kind = "armasync.settings"
schema_version = 1
language = "system"        # system | de | en
theme = "system"           # system | light | dark
ui_scale_percent = 100     # 75 bis 300
```

Fehlende Schlüssel erhalten Standardwerte (innerhalb derselben Schemaversion); unbekannte Schlüssel werden abgelehnt. Eine Einstellung kommt nur zusammen mit sichtbarer Wirkung und einem Versionssprung hinzu.

## Ablehnregeln (Lesen)

Der Store verändert beim Lesen nie Dateien. Jeder Fall ergibt `ErrorKind::Format` (Exitcode-Kategorie „Format“), außer Ein-/Ausgabefehlern (`FileSystem`):

| Fall | Verhalten |
| --- | --- |
| Datei fehlt | Erststart: leere Bibliothek bzw. Standardeinstellungen |
| größer als 4 MiB | abgelehnt, bevor gelesen wird; die Lesung ist zusätzlich auf diese Größe gekappt |
| keine reguläre Datei (Verzeichnis, Gerät, Pipe) | abgelehnt (`FileSystem`) |
| ungültiges JSON/TOML/UTF-8, JSON tiefer als 128 Ebenen | abgelehnt |
| falsches oder fehlendes `kind` | abgelehnt (fremde Datei) |
| `schema_version` neuer als 1 | abgelehnt mit Hinweis „von neuerer ArmaSync-Version geschrieben; aktualisieren oder Backup wiederherstellen“, **auch wenn** unbekannte Felder enthalten sind |
| `schema_version` fehlt, 0, negativ oder kein Integer | abgelehnt |
| unbekannte Felder | abgelehnt (`deny_unknown_fields`) |
| ungültige ID/Name/URL, doppelte IDs, unbekannte oder doppelte Profilverweise, Grenzen überschritten | abgelehnt; Meldungen der Modellprüfung enthalten **nicht** den abgelehnten Text (kein Echo möglicher Secrets in URLs); Typfehler des Parsers werden auf 200 Zeichen gekürzt |

Ältere Schemaversionen gibt es noch nicht; die erste Migration entsteht mit der ersten Schemaänderung und braucht dann einen eigenen Test (Datei vorher sichern, in neuer Datei ablösen).

## Atomares Schreiben und Backup

`save` schreibt in eine temporäre Datei im Zielverzeichnis (`create_new`), `sync_all`, kopiert die bisherige gültige Version nach `<datei>.bak` (ebenfalls über Temp-Datei und Rename) und ersetzt die Datei per `rename`; unter Unix wird zusätzlich das Verzeichnis synchronisiert. Danach ist die Datei entweder alt oder vollständig neu. Fehlschläge räumen die temporären Dateien auf. Ist die aktuelle Datei **nicht** lesbar, wird das gute Backup **nicht** überschrieben. Eine beschädigte Datei wird nie automatisch repariert oder gelöscht; die Frontends müssen den Nutzer entscheiden lassen (Backup laden: `LibraryStore::load_backup`).

## Verzeichnisse

| | Windows | Linux |
| --- | --- | --- |
| Config (`library.json`, `settings.toml`) | `%APPDATA%\Lynxnode.it\ArmaSync` | `$XDG_CONFIG_HOME/lynxnode/armasync` (sonst `~/.config/...`) |
| Daten | `%LOCALAPPDATA%\Lynxnode.it\ArmaSync` | `$XDG_DATA_HOME/lynxnode/armasync` (sonst `~/.local/share/...`) |
| Cache | `…\ArmaSync\cache` | `$XDG_CACHE_HOME/lynxnode/armasync` (sonst `~/.cache/...`) |
| portabel | `<Verzeichnis>\config`, `\data`, `\cache` | `<Verzeichnis>/config`, `/data`, `/cache` |

XDG-Variablen zählen nur, wenn sie absolut sind (XDG-Spezifikation); ohne `APPDATA`/`LOCALAPPDATA` greift `%USERPROFILE%\AppData\…`. Die Auflösung ist eine reine Funktion (`DataRoots::resolve` mit injizierter Umgebung) und legt nichts an; `ensure_writable` legt die Roots an und prüft sie mit einer Probedatei und ist ausdrücklich aufzurufen (Portabelmodus, Erststart). Andere Plattformen ergeben `Unsupported`. **Alte Verzeichnisse (Legacy-Baseline) werden weder gelesen noch angelegt, verschoben oder gelöscht.** Der portable Modus wird vom Frontend ausdrücklich gewählt; es gibt keine stille Erkennung.

## Credentials

Der Core kennt nur `CredentialRef` und den Port `CredentialStore` (`get`/`put`/`delete`, `&self`, threadsicher). `Secret` zeigt in `Debug` und `Display` nur `<redacted>`, ist nicht serialisierbar und liefert den Inhalt nur über `expose()`. `Credential::new` lehnt Steuerzeichen, leere oder überlange Werte ab, ohne sie in der Meldung zu wiederholen. Der Speicher wird beim Freigeben nicht genullt (kein `unsafe`, keine Zusatzabhängigkeit); das ist eine dokumentierte Grenze.

Vorhandener Adapter: `SessionCredentialStore` (nur Prozessspeicher, nie auf Platte). **Persistente Secrets werden derzeit nirgends gespeichert**; ein Schlüsselbund-Adapter ist Folgearbeit nach [ADR-0005](../adr/0005-credentials-and-data-paths.md). Eine Klartextspeicherung ist keine Voreinstellung und existiert nicht.

## Abhängigkeiten (neu in `armasync-adapters`)

Geprüft über `cargo metadata` am 2026-10-18; alle erfüllen MSRV 1.85 und sind permissiv lizenziert:

| Paket | Version | Lizenz | Zweck |
| --- | --- | --- | --- |
| serde (+ serde_core, serde_derive) | 1.0.229 | MIT OR Apache-2.0 | Dokumentstrukturen |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | `library.json` (bereits im Baum) |
| toml (+ toml_parser, toml_writer, toml_datetime, serde_spanned, winnow) | 1.1.8 | MIT OR Apache-2.0 (winnow: MIT) | `settings.toml` |

Transitiv u. a. `indexmap`, `hashbrown`, `foldhash` (Zlib), `memchr` (Unlicense OR MIT), `itoa`, `zmij` (MIT). Der CLI-Baum bleibt GUI-frei (46 statt 37 Pakete, `.github/scripts/cli-gui-free.sh` grün). Das ist keine Rechtsprüfung.

## Nicht Teil von MIG-020

Import/Export von Altdaten, Legacy-Parser, Sync-Adapter, Schlüsselbund-Adapter, Profilinhalte jenseits der Repositoryliste, Anbindung an GUI/CLI (das GUI-Prototyp-Modell nutzt noch eigene simulierte Typen), Dateisperre gegen zwei gleichzeitig laufende Instanzen (letzter Schreiber gewinnt; Single-Instance gehört zu MIG-051), Verschlüsselung der Dokumente.
