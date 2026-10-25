# ADR-0016 — Ereignisvertrag für den Upload: Fortschritt pro Datei (MIG-042, Schritt 4c)

Datum: 2026-10-25. Status: **angenommen** für Schritt 4c. Bezug: [ADR-0011](0011-upload-contract.md) (Upload-Vertrag), [ADR-0012](0012-revisions-and-publication.md) (Veröffentlichung), [ADR-0014](0014-upload-cli.md) (CLI), [ADR-0015](0015-upload-gui.md) (GUI; ihre Grenze „kein Fortschritt pro Datei“ ist mit diesem Schritt aufgehoben).

## Kontext

Die Veröffentlichung liefert bisher nur ein Ergebnis am Ende. Die GUI zeigt einen Spinner, die CLI nichts während des Laufs. Der bestehende `OperationEvent` (ADR für die Synchronisation) trägt keine Dateiidentität und keine Bytes pro Datei; ihn für den Upload zu erweitern würde den Synchronisationsvertrag berühren. Deshalb bekommt der Upload einen eigenen, versionierten Ereignistyp. `OperationEvent` bleibt unverändert.

## Entscheidungen

1. **Eigener Typ.** `armasync_core::upload_event::UploadEvent` mit `schema_version` (`UPLOAD_EVENT_SCHEMA_VERSION = 1`), `operation` (`OperationId`), `sequence`, `kind` (`UploadEventKind`) und `progress` (`UploadProgress`). Der Synchronisations-Ereignistyp bleibt unverändert.
2. **Stabile Ereignisnamen.** Jeder Typ hat einen festen Namen, der in Logs und JSON-Zeilen verwendet werden kann: `upload.started`, `upload.file_started`, `upload.file_bytes`, `upload.file_finished`, `upload.finished`, `upload.cancelled`, `upload.failed`. Namen werden nicht umbenannt; ein inkompatibler Wechsel erhöht die Schemaversion.
3. **Monotone Sequenz.** Pro Vorgang beginnt `sequence` bei 0 und wächst bei jedem Ereignis um genau 1. Lücken und Wiederholungen gibt es nicht.
4. **Reihenfolge.**
   - Genau ein `upload.started` als erstes Ereignis, mit den Gesamtwerten aus dem Plan. Scheitert ein Vorgang, bevor der Plan steht (etwa eine abgelehnte Adresse mit Zugangsdaten), sendet er trotzdem `upload.started` mit Nullwerten und danach `upload.failed`.
   - Je Revisionsdatei in Planreihenfolge: `upload.file_started` (mit Größe), danach beliebig viele `upload.file_bytes` mit streng steigendem `bytes_done` der Datei, dann `upload.file_finished`.
   - Genau ein Endereignis als letztes: `upload.finished` bei Erfolg, `upload.cancelled` bei Abbruch über den `CancelToken`, `upload.failed` mit der Fehlerkategorie bei jedem anderen Fehler. Danach kommt nichts mehr.
5. **Fortschrittswerte.**
   - `files_total` und `bytes_total`: Zahl und Summe der Größen der **Revisionsdateien, die hochgeladen werden** (Plan). Ist die Revision auf dem Ziel bereits vollständig, sind beide 0.
   - `files_done` steigt bei jedem `upload.file_finished` um 1.
   - `bytes_done` ist die Summe der fertigen Dateien plus die übertragenen Bytes der laufenden Datei. Es steigt nie.
   - `upload.finished` trägt `files_done == files_total` und `bytes_done == bytes_total`.
6. **Was nicht gemeldet wird.** Die Schreibvorgänge für Manifest, Zeiger und Verlauf sowie das Rücklesen (ADR-0012) erzeugen keine Datei-Ereignisse. Sie zählen weder zu den Gesamtwerten noch zu `bytes_done`. Die Vorschau erzeugt keine Ereignisse.
7. **Übertragene Bytes.** Gezählt werden die Bytes, die der Ziel-Adapter aus der Quelle liest. Das nähert die gesendeten Bytes an, ist aber keine Netzwerkmessung.
8. **Geheimnisfreiheit.** Ereignisse enthalten nur: Schemaversion, Vorgangskennung, Sequenz, Ereignistyp, relative Pfade der Revisionsdateien, Größen, Zähler und die Fehlerkategorie (`ErrorKind`). Sie enthalten **keinen** Fehlertext, keine absoluten Pfade, keine Adresse, keinen Benutzernamen und kein Kennwort. Fehlertexte bleiben in den bestehenden Fehlermeldungen des jeweiligen Frontends.
9. **Senke.** `UploadEventSink` ist ein Trait mit `emit(&mut self, UploadEvent)`, und `Send` ist Voraussetzung. Der Vorgang besitzt die Senke (`Box<dyn UploadEventSink + Send>`). Sie wird aus dem Vorgangsthread aufgerufen und darf nicht lange blockieren.
10. **Abbruch.** Ausschließlich über den vorhandenen `CancelToken`. Ein Abbruch mitten in einer Datei endet mit `upload.cancelled`; die bisherige Veröffentlichung bleibt unverändert (ADR-0012).
11. **Verdrahtung.**
    - Workflow: `publish` nimmt die Operationskennung und die Senke entgegen. Die Ereignisse werden in `armasync-application` erzeugt.
    - CLI: Die Veröffentlichung läuft in einem eigenen Thread, der Hauptthread liest die Ereignisse und schreibt sie im Textmodus auf stderr (Beginn und Ende je Datei). Im `--json`-Modus wird nichts zusätzlich ausgegeben. Stdout-Envelope und Exitcodes bleiben unverändert.
    - GUI: Der Worker sendet Ereignisse über denselben Kanal wie das Ergebnis. `poll` verarbeitet alle wartenden Ereignisse pro Bild. Der Fenster-Thread blockiert nie.
12. **Parallelität und Wiederaufnahme.** Beides bleibt aus. Dateien werden nacheinander übertragen.

## Grenzen

- Die Byte-Zählung hängt an den Lesevorgängen des Ziel-Adapters; die Blockgröße bestimmt die Häufigkeit der `upload.file_bytes`-Ereignisse.
- Es gibt keine Netzwerk-Messung und keine Geschwindigkeitsangabe.
- Lesegeräte- und Sichtprüfung der GUI-Fortschrittsanzeige sind nicht erbracht und bleiben offen.
- Die Ereignisse werden nicht gespeichert.

## Belege (vorgesehen)

- Application- und Workflow-Tests: Reihenfolge, Sequenz, Fortschrittswerte, Abbruch mitten in der Datei, Fehler auf dem Ziel, Geheimnisfreiheit über die Debug-Ausgabe aller Ereignisse.
- CLI-Prozesstest: Textmodus mit Datei-Zeilen auf stderr, JSON-Modus mit unverändertem Envelope und Exitcode.
- GUI-Tests: Fortschritt aus Ereignissen, Abbruch, Responsivität über die Abfrage.
