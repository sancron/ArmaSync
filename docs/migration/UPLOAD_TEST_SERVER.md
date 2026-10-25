# Manueller Upload-Prüfserver (Windows)

Ein Entwicklungs- und Prüfwerkzeug für die Upload-Prüfung des [Screenreader-Protokolls](GUI_UPLOAD_SCREENREADER_PROTOCOL.md). **Keine Produktfunktion.** Der Server ermöglicht nur die manuelle Prüfung; er führt selbst **keine** Nutzung eines Lesegeräts (NVDA, Narrator, Orca) und **keine** Sicht- oder Tastaturprüfung durch.

## Was der Server kann

- Bindet ausschließlich an `127.0.0.1` mit einem freien Port. Anonymes HTTP, keine Anmeldung, kein TLS.
- Unterstützt die Anfragen eines WebDAV-Uploads: `OPTIONS`, `PROPFIND`, `MKCOL`, `PUT`, `GET`, `DELETE`.
- Speichert alles nur im Arbeitsspeicher. Auf der Festplatte entsteht nichts; nach dem Beenden ist alles weg.
- Begrenzt: 64 MiB je Datei (größer: `413`), 256 MiB insgesamt und 200 Dateien (darüber: `507`). Pfade mit `..`, Trennzeichen, Doppelpunkt oder Steuerzeichen werden mit `400` abgelehnt.
- Hat keine Testfunktionen (keine Verzögerungen, keine Fehlerschalter) und keine Verbindung nach außen.

## 1. Server starten

Im Projektordner `X:\GitHub\sancron\ArmaSync` in PowerShell:

```powershell
cargo run -p armasync-testkit --example upload_webdav
```

Die Ausgabe nennt die Adresse für die GUI:

```
  Zieladresse für die GUI: http://127.0.0.1:53788/dav
```

Der Port ändert sich bei jedem Start. Das Fenster offen lassen, solange geprüft wird.

## 2. Testdaten anlegen (synthetisch)

```powershell
$src = 'C:\Temp\armasync-upload-test'
New-Item -ItemType Directory -Force -Path "$src\docs" | Out-Null
[IO.File]::WriteAllText("$src\a.txt", 'alpha')
[IO.File]::WriteAllText("$src\docs\b.txt", 'beta')
[IO.File]::WriteAllText("$src\.armasync-lock", '')
$fs = [IO.File]::Create("$src\big.bin"); $fs.SetLength(16MB); $fs.Close()
```

## 3. GUI starten und bedienen

1. `cargo build -p armasync-gui`, dann `target\debug\armasync-gui.exe` starten.
2. Oben die Ansicht **Upload** wählen.
3. Eintragen: **Quellordner** `C:\Temp\armasync-upload-test`, **Zieladresse** die URL aus Schritt 1, **Benutzername** und **Kennwort** leer lassen, **Ausschlüsse** leer.
4. **Vorschau prüfen** drücken.
5. Das Kontrollkästchen „Ich habe die Vorschau geprüft und bestätige die Veröffentlichung.“ setzen, dann **Veröffentlichen** drücken.
6. Nach dem Lauf **Veröffentlichen** erneut drücken, ohne etwas zu ändern (zweiter Lauf).
7. Für den Abbruch: vor dem Lauf **Abbrechen** drücken (bei 16 MiB kann die Übertragung schon fertig sein; dann nochmals versuchen).

## 4. Erwartete Ergebnisse

| Schritt | Erwartung |
| --- | --- |
| Vorschau | Statuszeile „Vorschau bereit: 3 Dateien (…), 1 ausgeschlossen.“; Zeile „Hochladen: a.txt …“, „Ausgeschlossen: .armasync-lock …, Grund: Interne ArmaSync-Daten“ |
| Veröffentlichen ohne Häkchen | nicht möglich; Hinweis „Bitte die Veröffentlichung ausdrücklich bestätigen.“ |
| Lauf | Fortschrittsleiste, „Dateien: n von 3 fertig.“, „Gerade wird übertragen: …“ |
| Ergebnis | „Veröffentlicht: Revision r-…, 3 Dateien hochgeladen.“; Kennwortfeld leer |
| Zweiter Lauf ohne Änderung | „Veröffentlicht: Revision r-…, 0 Dateien hochgeladen.“ und „Die Revision war auf dem Ziel bereits vollständig.“ |
| Abbruch | „Abgebrochen. Die bisherige Veröffentlichung bleibt unverändert.“ |
| Ziel nicht erreichbar (z. B. Port falsch) | „Fehler: …“ in der Statuszeile und neben dem Ergebnis |

Die HTTP-Antworten, die der Server gibt, sind mit `curl.exe` geprüft worden (am echten Prozess): `OPTIONS` 200, `PROPFIND` 207, `MKCOL` 201, `PUT` 201, `GET` 200, `DELETE` 204, unbekannter Pfad 404, Pfad mit `%2F..%2F` 400, Datei über 64 MiB 413.

## 5. Server beenden

Im Serverfenster `q` und Enter eingeben. Der Server listet die gespeicherten Dateien auf, schließt den Port und beendet sich. Strg+C beendet ihn sofort, ohne Abschlussausgabe. Der Quellordner auf der Festplatte bleibt unverändert.

## Grenzen und Hinweise

- Ein Benutzername oder ein Kennwort in der Adresse (`http://nutzer:…@…`) wird von der GUI abgelehnt; das ist beabsichtigt (Schritt U7).
- Eine Datei über 64 MiB beantwortet der Server mit `413`. Wie die GUI diese Antwort anzeigt, ist **nicht geprüft**.
- Falls im Windows-Benutzerkonto `HTTP_PROXY`, `HTTPS_PROXY` oder `ALL_PROXY` gesetzt sind, kann die Verbindung über einen Proxy laufen. Dann `NO_PROXY=127.0.0.1` setzen. Das ist **nicht geprüft**, weil in der Prüfumgebung keine Proxy-Variablen gesetzt waren.
- Der Server ist nicht für andere Rechner gedacht und nur über `127.0.0.1` erreichbar.
- Die Ergebnisse der Prüfung (Ansagen, Fokus, Kontrast) trägt jemand mit Lesegerät und Sicht in das [Protokoll](GUI_UPLOAD_SCREENREADER_PROTOCOL.md) ein. Der Server erledigt das nicht.
