# ADR-0012 — Revisionen und Veröffentlichung (MIG-042, Schritt 2)

Datum: 2026-10-25. Status: **angenommen** für den Umfang Schritt 2 (Revision bilden, Dateien in einen revisionsbezogenen Bereich laden, Zeiger zuletzt setzen). Nicht angenommen und ausdrücklich nicht entschieden: das **Legacy-Schreibformat** der Metadaten (`.a3s/sync`, `.a3s/serverinfo`) (ADR-0003 bleibt offen), Ausschlüsse, Ereignisse und Changelogs, Parallelität, Wiederaufnahme und das Aufräumen alter Revisionen. Bezug: [ADR-0011](0011-upload-contract.md) (Upload-Vertrag), [ADR-0010](0010-ftp-sftp-webdav-contract.md) (WebDAV), [ADR-0003](0003-legacy-formats.md), [SYNC_METADATA](../migration/SYNC_METADATA.md).

## Kontext

Der Upload aus Schritt 1 schreibt Dateien direkt an ihre Zielpfade. Eine Veröffentlichung braucht eine Revision: eine feste Menge von Dateien mit Hashes, die als Ganzes gilt. Die Metadaten des Altbestands (`.a3s/sync`) sind eine GZIP-Java-Serialisierung. Sie zu **schreiben** hieße, das offene Legacy-Schreibformat (ADR-0003) festzulegen. Das geschieht hier nicht.

## Entscheidungen

1. **Revisionskennung aus dem Inhalt.** Die Kennung ist `r-` plus den ersten 32 Hexziffern von SHA-256 über die kanonische Auflistung: für jede Datei in Pfadreihenfolge eine Zeile `pfad TAB größe TAB sha256 LF`. Gleicher Inhalt ergibt dieselbe Kennung; es gibt keine Zeitstempel in der Kennung.
2. **Revisions-Manifest** (eigenes Format `armasync.revision`, Version 1), UTF-8, JSON:
   `{"schema":"armasync.revision","version":1,"revision":"r-…","files":[{"path":"…","size":N,"sha256":"…"}, …]}`. Dateien in Pfadreihenfolge. Pfade werden JSON-kodiert (Anführungszeichen, Rückstriche und Steuerzeichen werden escaped). Dieses Format ist **nicht** das Altformat und nicht kompatibel mit dem Altclient.
3. **Layout auf dem Ziel** (relativ zur Basisadresse des Ziels):
   - `revisions/<id>/files/<relativer Pfad>`: die Dateien der Revision.
   - `revisions/<id>/manifest.json`: das Revisions-Manifest, **zuletzt** geschrieben. Sein Vorhandensein bedeutet „Revision vollständig“.
   - `current.json`: der Veröffentlichungszeiger `{"schema":"armasync.current","version":1,"revision":"r-…","manifest_sha256":"…"}`.
   - `history/<sha256 der alten current.json-Bytes>.json`: Kopie des vorherigen Zeigers, bevor er ersetzt wird.
4. **Ablauf einer Veröffentlichung.**
   1. Revision aus der Quelle bilden (Hashes mit SHA-256 aus dem Scan).
   2. Existiert `revisions/<id>/manifest.json` bereits und ist identisch, gilt die Revision als vollständig: keine Datei-Uploads. Ist es abweichend, bricht der Vorgang mit `Integrity` ab, ohne etwas zu ändern.
   3. Sonst die Ordner anlegen, die Dateien über den Upload-Ablauf (Schritt 1) in `revisions/<id>/files/` laden und zuletzt `manifest.json` schreiben.
   4. Ist `current.json` bereits identisch, ist nichts zu tun. Sonst wird der alte Zeiger nach `history/` kopiert (nur wenn dort noch nicht vorhanden), dann der neue Zeiger geschrieben und per GET gegengeprüft.
5. **Fehler und Abbruch.** Bis zum Schreiben des Zeigers bleibt der bisherige Zeiger unverändert. Ein Abbruch oder Fehler in Schritt 3 hinterlässt eine unvollständige Revision (ohne `manifest.json`), die bei der nächsten Veröffentlichung neu gefüllt wird; der Zeiger zeigt weiter auf die alte Revision. Scheitert das Schreiben des Zeigers, prüft der Vorgang per GET, ob der alte oder der neue Zeiger liegt. Ist keines von beiden vorhanden oder nicht lesbar, endet der Vorgang mit `Integrity` („Veröffentlichungszustand unbekannt; der vorherige Zeiger liegt in history/“).
6. **Keine Löschungen.** Keine Revision, keine Datei und kein Zeiger wird gelöscht. Alte Revisionen bleiben liegen.

## Atomarität (ausdrücklich nicht behauptet)

WebDAV definiert kein atomares Ersetzen einer Datei über `PUT`. Ein Server kann nach einem Abbruch eine teilweise geschriebene Datei zurücklassen. Der Adapter kann deshalb **keine atomare Veröffentlichung zusagen**. Was der Ablauf stattdessen garantiert:

- Ein Leser, der nur `current.json` und dann das Manifest liest, erkennt einen falsch geschriebenen Zeiger über `manifest_sha256` und das Manifest über seine Kennung und die Dateihashes.
- Die alte Revision bleibt vollständig erhalten; der alte Zeiger liegt nach dem Ersetzen in `history/`.
- Ob ein konkreter Server `PUT` atomar ausführt, ist nicht belegt und wird nicht behauptet.

## Port-Erweiterung

- `SourceFile` trägt einen SHA-256-Hash (Pflicht). Der Scan liefert ihn bereits.
- `RemoteTarget::fetch(path, limit)` liest eine Datei per `GET` bis zum Limit und liefert `None` bei `404`. Weiterleitungen werden nur innerhalb derselben Herkunft gefolgt, Zugangsdaten gelten wie in ADR-0010.

## Nicht im Scope

Legacy-Metadaten schreiben (ADR-0003); Ausschlüsse; Ereignisse und Changelogs; Parallelität; Wiederaufnahme; Read-back aller Dateien; Aufräumen alter Revisionen; CLI- und GUI-Verdrahtung; echte Server.

## Belege

- Legacy-Metadaten: `docs/migration/SYNC_METADATA.md` (GZIP-Objektserialisierung, Felder und Hashart SHA-1).
- Tests: `crates/armasync-application/tests/revision_cases.rs` (Kennung, Manifest, Zeiger), `crates/armasync-adapters/tests/publish_flow.rs` (Erfolg, Abbruch, Fehler beim Zeiger, Wiederholung, keine Löschung, abweichende Revision).
