# ADR-0009 — Resume mit `Range`: Validatorvertrag, Teildaten und Abbruch (MIG-032R)

Datum: 2026-10-25. Status: **angenommen** (Vertragsreview durch den Integrator vor der Implementierung; Gegenprüfung durch einen zweiten Reviewer ist nicht erfolgt, siehe „Offen“). Bezug: [ADR-0006](0006-sync-metadata-and-http-transport.md), [ADR-0007](0007-transfer-ports.md), [ADR-0008](0008-packed-files-zip.md), [LOCAL_FS_THREATS](../migration/LOCAL_FS_THREATS.md), [MIG-032R](../tasks/MIG-032R.md).

Dieser ADR **ergänzt** ADR-0007. Wo er ADR-0007 ausdrücklich ändert, gilt die Regel hier; alle übrigen Festlegungen von ADR-0007 bleiben.

## Kontext

ADR-0007 schließt Resume aus: jede Übertragung beginnt bei Null, ein Fehler verwirft das Geschriebene. Große Mod-Dateien und Archive im Gigabytebereich machen das bei unterbrochenen Verbindungen teuer. Ein Fortsetzen ist aber nur sicher, wenn die bereits geladenen Bytes zur **aktuellen** Revision der Datei gehören.

## Entscheidungen

### 1. Validator

1. Ein Validator ist ausschließlich ein **starkes `ETag`**: der Wert beginnt und endet mit `"`, hat kein Präfix `W/`, enthält nur sichtbare ASCII-Zeichen und Leerzeichen und ist höchstens 256 Byte lang. Die Prüfung ist `armasync_core::is_strong_validator`.
2. **Schwache** ETags (`W/…`) und `Last-Modified` werden nie verwendet. Begründung: schwache ETags sagen nur „inhaltlich ähnlich“, `Last-Modified` hat eine Sekundenauflösung und ändert sich bei gleichem Inhalt.
3. Ein Validator ist nur gültig, wenn er **aus derselben Antwort** stammt, die die Bytes liefert (der Validator einer früheren Antwort gilt nicht).
4. Ohne gültigen Validator gibt es **kein** Fortsetzen. Die Teildatei wird dann bei jedem Fehler verworfen.

### 2. Anfrage und Antwort (HTTP)

Fortsetzen wird nur angefragt, wenn der Teilstand einen gültigen Validator und einen Offset `0 < offset < size` hat. Dann sendet der Adapter `Range: bytes=<offset>-` und `If-Range: <validator>`.

| Antwort | Bedingung | Ergebnis |
| --- | --- | --- |
| `206` | `Content-Range` ist genau `bytes <offset>-<size-1>/<size>`, `ETag` vorhanden und stark und **gleich** dem gespeicherten Validator, `Content-Length` (falls gesetzt) `= size - offset` | **Fortsetzen**: Body beginnt bei `offset`. |
| `206` mit anderem Bereich, anderer Gesamtgröße, fehlendem oder anderem Validator | | Protokollfehler `Integrity` („resume refused“). Die Anwendung verwirft die Teildatei und fragt **einmal** ohne `Range` neu an. |
| `200` (Range ignoriert oder `If-Range` nicht erfüllt) | `Content-Length` (falls gesetzt) `= size` | **Neustart** bei 0: Der Body ist die vollständige Datei. Ein neuer starker ETag aus dieser Antwort wird gespeichert, sonst kein Validator. |
| `416` | Bereich nicht erfüllbar (Datei geändert oder Teilstand unpassend) | wie „resume refused“ (einmal ohne `Range` neu). |
| `404`, `401/403`, sonstige Statuscodes, Weiterleitungsregeln | | unverändert nach ADR-0006 (`Network`). |

Eine Antwort mit `Content-Encoding` ungleich `identity` bleibt abgelehnt (ADR-0006). Bei jedem Fortsetzen wird die Gesamtdatei **trotzdem** über Größe und Manifest-Hash geprüft. Ein `Range`-Header ist kein Vertrauensnachweis.

### 3. Port-Erweiterung (additiv, Core)

* `RepositorySource::identity(&self) -> &str`: stabiler Name der Quelle (HTTP: normalisierte Basis-URL). Leer = **kein Fortsetzen** für diese Quelle.
* `RepositorySource::open(path, expected_size, offset, if_range) -> Result<Opened, Error>`: liefert `Opened { start, validator, body }`. `start` ist `offset` (Fortsetzen akzeptiert) oder `0` (Neustart). `expected_size` ist nötig, damit die Zeitgrenze des Body der **Restgröße** folgt (Korrektur gegenüber dem ersten Entwurf, der die Größe nicht enthielt). Die Standardimplementierung liefert die ganze Datei ab 0 aus `read`.
* Jeder Adapter und die Anwendung prüfen die **Stärke** des Validators: ein schwacher Wert wird wie „kein Validator“ behandelt (HTTP-Adapter, In-Memory-Referenz und Anwendung, jeweils unabhängig).
* `fetch` bleibt im ADR-0007-Vertrag und wird über `open` mit `offset = 0` implementiert; die HTTP-Überschreibung entfällt.
* `LocalStore::partial(&mut self, key: &PartialKey) -> Result<Box<dyn PartialFile + '_>, Error>`: öffnet oder erzeugt die Teildatei für `key = (source, path, size)`. Die Datei ist **nie** unter dem Zielpfad sichtbar. Das Handle leiht den Speicher aus, solange es lebt; eine Teildatei kann deshalb nicht zugleich mit einer Staging-Datei desselben Speichers geführt werden (Folge: gepackte Archive, siehe Abschnitt 7).
* `PartialFile: Read + Write + Seek` mit:
  * `stored() -> Option<StoredPartial { offset, validator }>`: gültiger Teilstand aus einem früheren Lauf (Zustand passt zum Schlüssel, Validator stark, Daten nicht länger als `size`); sonst `None`.
  * `start(validator: Option<&str>)`: kürzt die Daten auf 0 und speichert den Validator (oder löscht den Zustand bei `None`).
  * `keep(self)`: lässt Daten und Zustand für einen späteren Lauf liegen. Ohne gespeicherten Validator wird stattdessen verworfen.
  * `discard(self)`: entfernt Daten und Zustand.
  * `commit_to(self, target)`: übernimmt die vollständige und geprüfte Teildatei **atomar** als Ziel (wie `StagedFile::commit`).

### 4. Aufbewahrung und Wiederaufnahme über Prozessläufe

* **Dateien** im Zielverzeichnis: Daten `.armasync-partial-<id>.part`, Zustand `.armasync-partial-<id>.state`. `<id>` sind die ersten 32 Hex-Zeichen von SHA-256 über `source`, `path` und `size`. Beide Namen tragen das reservierte Präfix `.armasync-`, werden nie gelistet und nie als Repositorydatei akzeptiert.
* **Zustand** (Textformat, Zeilen): Formatkennung, `source`, `path`, `size`, `validator`. Er wird **vor** dem ersten Body-Byte geschrieben. Ein unlesbarer oder nicht passender Zustand ergibt „nicht fortsetzbar“ und keinen Fehler.
* **Offset** ist die Länge der Datendatei. Ist sie größer als `size` oder der Zustand ungültig, wird neu begonnen.
* **Wiederaufnahme nach Prozessende:** Ein neuer Prozess (neue Instanz, Zielsperre neu erworben) findet die Teildatei über denselben Schlüssel und setzt sie mit `Range` und `If-Range` fort. Das ist der eigentliche Zweck.
* **Aufräumen unter der Zielsperre:** Ein Scan mit gehaltener Zielsperre entfernt (a) Teildateien, deren Gegenstück fehlt (Daten ohne Zustand oder Zustand ohne Daten; nie fortsetzbar), und (b) Teildateien und Zustände, deren Änderungszeit mehr als 7 Tage zurückliegt (`PARTIAL_MAX_AGE`). Ohne Zielsperre wird nichts entfernt.

### 5. Abbruch, Fehler und Neustart — ÄNDERUNG von ADR-0007

ADR-0007 sagt: „Abbruch und Fehler lassen die Zieldatei unberührt und entfernen die Staging-Datei.“ Diese Regel wird wie folgt präzisiert:

| Ereignis | Teildatei | Fehler |
| --- | --- | --- |
| **Kooperativer Abbruch** (`CancelToken`, z. B. Ctrl+C, GUI-Abbrechen) | **entfernt** (Daten und Zustand) | `Cancelled` |
| **Verbindungsabbruch** während des Body (Lesefehler, z. B. Zeitüberschreitung, unerwartetes Ende der Verbindung) | **bleibt** erhalten, wenn ein gültiger Validator gespeichert ist; sonst entfernt | `Network` |
| Netzfehler **vor** dem Body (Verbindung, Status, Zeitüberschreitung) | wie oben (bisherige Teildaten bleiben, wenn gültig) | `Network` |
| **Sauberes Ende** vor der angekündigten Größe (kein Lesefehler, weniger Bytes) | **entfernt** | `Integrity` (wie ADR-0007) |
| **Zu viele Bytes**, Größe oder Hash der Gesamtdatei falsch, Protokollfehler nach dem einmaligen Neustart | **entfernt** | `Integrity` |
| Schreib-/Dateisystemfehler, Fehler beim Austausch (`commit_to`) | **entfernt**; das Ziel bleibt unverändert | `FileSystem` |
| **Prozessende ohne Aufräumen** (Absturz, Kill, Stromausfall) | bleibt liegen; wird beim nächsten Lauf geprüft | – |
| Erneuter Start nach einem der obigen Fälle | Fortsetzen nur bei passendem Schlüssel und gültigem Validator; sonst Neustart | |

**Bewusst beibehalten:** Ein kooperativer Abbruch hinterlässt **keine** temporären Dateien. Die Garantie „keine temporären Dateien nach Abbruch“ gilt also weiter. Ein Netzfehler hinterlässt Teildaten. Das ist neu und in der Tabelle ausdrücklich dokumentiert.

**Verkürzte Antwort** wird als `Network` gemeldet, nicht als `Integrity`. Begründung: Sie ist ein Übertragungsabbruch, keine Verfälschung; die Gesamtprüfung bleibt unverändert. Der Standardablauf `fetch` (ADR-0007) meldet eine verkürzte Lieferung weiterhin als `Integrity`, weil dort kein Fortsetzen möglich ist.

### 6. Verifikation

* Der Gesamtinhalt wird am Ende **immer** über Größe und Manifest-Hash geprüft. Bei Fortsetzung wird der bereits vorhandene Präfix vor dem neuen Body in den Digest eingelesen.
* Erst nach bestandener Prüfung wird die Datei mit `commit_to` atomar übernommen. Eine Teildatei ist nie das Ziel.
* Schlägt die Prüfung nach einer Fortsetzung fehl, wird die Teildatei **verworfen** (`Integrity`). Der nächste Lauf beginnt bei Null. Es gibt keine Wiederholungsschleife.

### 7. Zusammenspiel

* **Zielsperre:** Erzeugen, Fortsetzen, Aufbewahren, Verwerfen und Übernehmen einer Teildatei verlangen die exklusive Zielsperre (wie `begin`). Ein `LocalDirectory::new` (nur lesend) kann keine Teildatei ändern.
* **Freiplatzprüfung (MIG-032B):** unverändert. Sie rechnet den vollen Bedarf (Datei plus Archiv) und gutschreibt **keine** vorhandenen Teilbytes. Das ist konservativ und kann einen Lauf unnötig ablehnen, nie einen zu großen zulassen.
* **ZIP-Spool (ADR-0008):** **Gepackte Archive sind in MIG-032R nicht fortsetzbar.** Sie laufen wie bisher über den Spool: vollständig laden, entpacken, prüfen; jeder Fehler verwirft den Spool. Grund: Eine Teildatei hält den Speicher ausgeliehen, solange sie lebt, und das Entpacken braucht gleichzeitig die Staging-Datei desselben Speichers. Eine Fortsetzung der Archive braucht deshalb einen eigenen Port-Schritt (Folgetask, nicht zugesagt). Plain-Dateien sind voll fortsetzbar.
* **Atomarer Commit:** Die Zieldatei wird nur durch `commit_to` nach vollständiger Prüfung verändert.
* **Ordner-Pinning und Containment (MIG-030H, LOCAL_FS_THREATS):** Teildatei und Zustand liegen im selben Ordner wie das Ziel und laufen über dieselben Pfadprüfungen.

## Belegte Grenzen

* Gegen die echten Server (ETag-Semantik, `If-Range`, Caches, Proxys) **nicht belegt**. Belegt wird nur gegen den Loopback-Testserver und synthetische Antworten.
* Gepackte Archive sind nicht fortsetzbar (siehe Abschnitt 7). Quellen ohne `identity` und Quellen ohne starken Validator setzen nie fort.
* Kein Scatter/Gather, keine parallelen Bereiche, kein Delta.

## Offen

* Zweites Vertragsreview: Der Integrator hat den Vertrag selbst geschrieben und geprüft. Eine unabhängige Gegenprüfung ist nicht erfolgt; das ist im Bericht als Risiko geführt.
* Resume für gepackte Archive (Port-Schritt, siehe Abschnitt 7).
* Verhalten bei großen Teildateien auf FAT32/exFAT-Laufwerken (keine Sparse-Dateien) ist nicht gemessen.

## Nicht gewählt

Last-Modified als Validator; schwache ETags; Fortsetzen ohne `If-Range`; Anhängen nach einem `200`; Teildaten als installierte Datei behandeln; stilles Wiederholen mit Anhängen; kooperativen Abbruch mit Teildaten-Aufbewahrung.
