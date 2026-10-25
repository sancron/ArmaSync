# ADR-0015 — Upload-Ansicht der GUI: Vorschau, Bestätigung, Zustände und Grenzen (MIG-042, Schritt 4b)

Datum: 2026-10-25. Status: **angenommen** für Schritt 4b. Bezug: [ADR-0014](0014-upload-cli.md) (CLI, gleiche Verdrahtung), [ADR-0012](0012-revisions-and-publication.md) (Veröffentlichung), [ADR-0013](0013-upload-exclusions.md) (Ausschlüsse), [ADR-0005](0005-credentials-and-data-paths.md) (Zugangsdaten), [UI_REQUIREMENTS](../migration/UI_REQUIREMENTS.md) (Leitvorgabe: zugänglich, Tastatur, helle und dunkle Themen).

## Kontext

Die GUI hat bereits eine Synchronisationsansicht mit Worker-Thread, Abbruch über `CancelToken` und einer Barrierefreiheits-Prüfung über den AccessKit-Baum. Die Upload-Ansicht soll dieselben Regeln einhalten wie die CLI und darf keine eigene Upload-Logik haben.

## Entscheidungen

1. **Dieselben Funktionen wie die CLI.** Die Ansicht ruft ausschließlich `armasync_workflow::upload::{preview, publish}` auf. `armasync-gui` enthält keine Upload-Regel. Der Ereigniskanal des Workflows wird nicht verändert (das bleibt 4c).
2. **Vorschau vor jeder Veröffentlichung.** Die Vorschau zeigt eingeschlossene Dateien mit Aktion in Worten („Hochladen“, „ersetzt die vorhandene Datei“), ausgeschlossene mit Grund („Interne ArmaSync-Daten“, „Teildatei …“, „ausgeschlossen durch »…«“) und ungenutzte Regeln.
3. **Ausdrückliche Bestätigung.** Die Schaltfläche „Veröffentlichen“ erscheint nur nach einer Vorschau und ist deaktiviert, bis das Kontrollkästchen „Ich habe die Vorschau geprüft und bestätige die Veröffentlichung.“ gesetzt ist. Es gibt **keinen modalen Dialog**: Die Bestätigung ist ein gewöhnliches, beschriftetes Kontrollkästchen. So entsteht kein versteckter Fokus, der sich dem Tastaturnutzer entzieht.
4. **Veraltete Vorschau wird verhindert.** Jede Änderung der Eingaben nach der Vorschau macht sie ungültig. Veröffentlichen ist dann gesperrt, mit einer Begründung in Worten.
5. **Laufender Zustand ohne Fortschritt pro Datei.** Während eines Vorgangs zeigt die Ansicht einen Spinner mit Text („kein Fortschritt pro Datei; Abbruch ist jederzeit möglich“) und den Abbrechen-Knopf. Die Formularfelder sind dann deaktiviert. Der Fenster-Thread blockiert nie: Die Arbeit läuft im Worker, das Ergebnis wird pro Frame abgeholt. Fortschritt pro Datei ist 4c.
6. **Abbruch über den vorhandenen `CancelToken`.** Ein Abbruch endet als „Abgebrochen. Die bisherige Veröffentlichung bleibt unverändert.“ Für den Fall eines Fehlers, bei dem der Zeigerstand unklar ist, sagt die Ansicht nur, was die Fehlermeldung aussagt, und verspricht keine Unveränderlichkeit.
7. **Zugangsdaten.** Das Kennwort steht nur im Formular dieser Sitzung. Es wird nicht gespeichert, nicht ausgegeben und nicht im Debug-Ausdruck gezeigt. Zugangsdaten gehen nur über HTTPS; der Workflow lehnt Adressen mit Zugangsdaten und Zugangsdaten über HTTP vor jeder Verbindung ab. Nach einer Veröffentlichung wird das Kennwort aus dem Formular entfernt.
8. **Barrierefreiheit.** Jedes Feld ist mit seiner sichtbaren Beschriftung verbunden. Die Statuszeile ist eine höfliche Live-Region. Fehler stehen bei ihrem Ergebnis mit dem Präfix „Fehler:“. Der Status hängt nie nur an der Farbe.
9. **Themen.** Helles und dunkles Thema teilen dieselben Beschriftungen. Die Fehlerfarbe erreicht in beiden Themen mindestens 4,5:1 gegen den Hintergrund des Panels (geprüft in `tests/upload_ui.rs`).

## Grenzen

- Keine Fortschrittsanzeige pro Datei (4c).
- Keine Bildschirmleser-Läufe; die Tests prüfen den AccessKit-Baum, nicht das Verhalten echter Lesegeräte.
- Die Passwort-Löschung nach einer Veröffentlichung ist nicht über den Loopback-Server testbar, weil dort Zugangsdaten über HTTP abgelehnt werden und nur HTTPS sie zulässt; der Pfad ist im Code klar begrenzt, aber nicht automatisiert belegt.
- Kein Resume, keine Parallelität, keine Legacy-Formate, keine echten Server.

## Belege

- Verhalten: `crates/armasync-gui/src/upload_screen.rs`; Tests `crates/armasync-gui/tests/upload_screen.rs` (8).
- Darstellung: `crates/armasync-gui/src/upload_ui.rs`; Tests `crates/armasync-gui/tests/upload_ui.rs` (5, AccessKit, Tastatur, Themen, Kontrast).
- Einbindung: `crates/armasync-gui/src/ui.rs` (neue Ansicht „Upload“).
