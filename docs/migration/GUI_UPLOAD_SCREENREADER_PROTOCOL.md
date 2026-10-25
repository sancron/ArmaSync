# Ergänzung zum Prüfprotokoll: Upload-Ansicht (Screenreader, Tastatur, Darstellung)

Gegenstand: die Ansicht „Upload“ der GUI (MIG-042, Schritte 4b und 4c; [ADR-0015](../adr/0015-upload-gui.md), [ADR-0016](../adr/0016-upload-progress-events.md)). Ergänzt [GUI_SCREENREADER_PROTOCOL](GUI_SCREENREADER_PROTOCOL.md); dessen Abschnitte A bis F bleiben gültig und betreffen die Prototyp- und Synchronisationsansicht.

**Stand: nicht ausgeführt.** Für die Upload-Ansicht liegt **kein** NVDA-, Narrator-, Orca-, Tastatur- oder Sichtnachweis vor. Die automatisierten Tests in Abschnitt 1 prüfen den AccessKit-Baum und das Verhalten, **nicht** was ein Lesegerät ansagt. Dieses Protokoll ändert den Gate-Status nicht: Das P1-Zugänglichkeitsgate (RB-02) bleibt geschlossen, bis die manuellen Ergebnisse aus Abschnitt 2 bis 4 vorliegen und den Kriterien in Abschnitt E des Basisprotokolls genügen.

## 0. Kopf (je Durchlauf ausfüllen)

| Feld | Wert |
| --- | --- |
| Version (Workspace) | `0.0.0` (`Cargo.toml`, Abschnitt `[workspace.package]`) |
| Build-SHA des geprüften Codes | (vor dem Durchlauf mit `git rev-parse --short HEAD` festhalten) |
| Vorbereitungsstand dieses Protokolls | Code `a2179af` (MIG-042 Schritt 4c), Dokumente bis `d1b9cd4` |
| Datum / Prüfer | |
| Betriebssystem + Version | |
| Systemskalierung (100 / 150 / 200 %) | |
| Lesegerät + Version | NVDA: ______ (Windows, **manuell offen**); Narrator (optional): ______ |
| Theme(s) geprüft (Hell / Dunkel) | |
| Ergebnis gesamt | nicht geprüft |
| Befunde (Nr., Wortlaut, Schwere) | |

Ergebnisfelder in den Tabellen: **bestanden**, **fehlgeschlagen** oder **nicht geprüft**. Ein leeres Feld zählt als nicht geprüft. Bei „fehlgeschlagen“ den Wortlaut der Ansage oder der Anzeige in der Beobachtungsspalte notieren.

## 1. Automatisierte Abdeckung (AccessKit-Baum und Verhalten)

Diese Tests laufen unter Windows mit `cargo test -p armasync-gui`. Sie sind **kein Ersatz** für Abschnitt 2 bis 4.

| Prüfpunkt | Test | Was belegt ist | Was **nicht** belegt ist |
| --- | --- | --- | --- |
| Jedes Feld mit sichtbarem Namen | `upload_ui::every_field_has_a_visible_name_…` | Namen im Baum für Quellordner, Zieladresse, Benutzername, Kennwort, Ausschlüsse; keine namenlosen Widgets | wie NVDA sie ansagt |
| Vorschau per Tastatur | `upload_ui::the_preview_button_runs_by_keyboard_…` | Enter löst die Vorschau aus; Vorschauzeilen mit Wortlaut („Hochladen: …“, „Ausgeschlossen: …, Grund: …“) | Lesereihenfolge |
| Veröffentlichen erst nach Vorschau | `upload_ui::every_field_…` (Teil ohne Vorschau), `upload_ui::the_preview_button_…` | Schaltfläche fehlt vor der Vorschau; nach der Vorschau deaktiviert, bis das Kontrollkästchen gesetzt ist | Fokuslage bei Änderung |
| Bestätigung vor Veröffentlichung | `upload_screen::publishing_needs_a_checked_and_confirmed_preview` | ohne Bestätigung startet nichts; kein Schreibzugriff | — |
| Veraltung nach Eingabeänderung | `upload_screen::editing_an_input_after_the_preview_makes_it_stale` | Änderung macht die Vorschau ungültig; Veröffentlichen gesperrt | Anzeige des Hinweistexts im Baum (nicht geprüft) |
| Statuszeile als höfliche Live-Region | `upload_ui::the_status_line_is_a_polite_live_region` | Live = Polite | welche Wechsel tatsächlich angesagt werden |
| Kennwort nicht ausgegeben | `upload_screen::tests::the_form_debug_output_hides_the_password` (Unit-Test in `src/upload_screen.rs`); `upload_screen::credentials_over_plain_http_…` | Debug-Ausgabe und Fehlermeldung enthalten das Kennwort nicht | Maskierung im Baum (nicht geprüft) |
| Kennwort leer nach Veröffentlichung | — | im Code belegt (`finish_publication`), **nicht automatisiert** | — |
| Abbruch | `upload_screen::a_cancelled_publication_keeps_the_previous_pointer` | Abbruch führt zu „Abgebrochen“; kein Zeiger | Wortlaut im Baum |
| Fortschritt Bytes, Datei, Zähler | `upload_screen::a_running_publication_shows_bytes_…` | Werte aus den Ereignissen, monoton, danach leer | **Fortschrittsleiste im Baum und Live-Verhalten: nicht geprüft** |
| Themen | `upload_ui::the_view_renders_in_both_themes_…` | dieselben Beschriftungen in Hell und Dunkel | Erscheinungsbild |
| Kontrast Fehlerfarbe | `upload_ui::the_error_colour_has_enough_contrast_…` | ≥ 4,5:1 gegen den Panelhintergrund in beiden Themen | Kontrast der Fortschrittsleiste und der deaktivierten Schaltflächen (nicht geprüft) |

## 2. Manuell: Windows, NVDA

**Vorbereitung.** Build-SHA notieren; `cargo build -p armasync-gui`; starten mit `target\debug\armasync-gui.exe`; NVDA starten und die Sprachanzeige aktivieren. Testdaten: ein Quellordner mit `a.txt`, `docs\b.txt` und `.armasync-lock`, dazu eine Datei `big.bin` von etwa 16 MiB für den Fortschritt. Zielserver: der manuelle Upload-Prüfserver aus dem Testkit (nur 127.0.0.1, anonym, kein Produktcode, Speicher nur im Arbeitsspeicher). Start, Adresse und Beenden: [UPLOAD_TEST_SERVER](UPLOAD_TEST_SERVER.md). Die Adresse für die GUI lautet `http://127.0.0.1:<port>/dav`, Benutzername und Kennwort bleiben leer. **Der Server ermöglicht nur die Prüfung; er führt weder die Nutzung eines Lesegeräts noch die Sicht- oder Tastaturprüfung aus.**

Die Ansagen sind Erwartungen, die sinngemäß gelten; der genaue Wortlaut hängt von NVDA ab. Jede Abweichung ist ein Befund.

### 2.1 Navigation und Start

| Nr. | Schritt | Erwartetes Verhalten / Ansage | Ergebnis | Beobachtung |
| --- | --- | --- | --- | --- |
| U1 | Mit Tab zur Ansichtsauswahl; „Upload“ wählen | „Upload, Auswahl“ bzw. „ausgewählt“; die Ansicht wechselt | nicht geprüft | |
| U2 | Start der Ansicht | Überschrift „Upload“ und der Erklärtext („… Zuerst die Vorschau prüfen, dann bestätigen.“) sind lesbar | nicht geprüft | |
| U3 | Statuszeile lesen | „Status: Bereit. Quellordner und Zieladresse angeben, dann die Vorschau prüfen.“ | nicht geprüft | |

### 2.2 Formular und Kennwort

| Nr. | Schritt | Erwartetes Verhalten / Ansage | Ergebnis | Beobachtung |
| --- | --- | --- | --- | --- |
| U4 | Tab durch die Felder | Namen: „Quellordner (lokal; wird nicht verändert):“, „Zieladresse (https://…, ohne Benutzername und Kennwort):“, „Benutzername (optional, nur über HTTPS):“, „Kennwort (nur für diese Sitzung, wird nicht gespeichert):“, „Ausschlüsse (je Zeile …):“ | nicht geprüft | |
| U5 | Kennwort eingeben | Eingabe ist maskiert (Punkte oder Sterne); NVDA sagt **kein** Zeichen des Kennworts an | nicht geprüft | |
| U6 | Mit Tab das Kennwortfeld verlassen | Fokus springt zum nächsten Feld in logischer Reihenfolge | nicht geprüft | |
| U7 | Zieladresse mit Zugangsdaten eingeben (`https://nutzer:geheim@host/dav`) und Vorschau prüfen | Fehler „Fehler: …“ in der Statuszeile; die Zugangsdaten werden nicht vorgelesen; der Wortlaut nennt die Ursache | nicht geprüft | |

### 2.3 Vorschau

| Nr. | Schritt | Erwartetes Verhalten / Ansage | Ergebnis | Beobachtung |
| --- | --- | --- | --- | --- |
| U8 | „Vorschau prüfen“ mit Enter auslösen | Status ändert sich: „Vorschau wird erstellt … (es wird nichts veröffentlicht)“, danach „Vorschau bereit: n Dateien (…), m ausgeschlossen.“ wird **einmal** angesagt | nicht geprüft | |
| U9 | Vorschau lesen (Lesemodus oder Pfeiltasten) | Überschrift „Vorschau“; Zeile „Hochladen: a.txt (13 B)“; Zeile „Ausgeschlossen: .armasync-lock (0 B), Grund: Interne ArmaSync-Daten“ — der Grund wird **in Worten** gelesen | nicht geprüft | |
| U10 | Ausschluss für eine nicht vorhandene Datei eintragen (`nichts.txt`), Vorschau erneut prüfen | Zeile „Regel ohne Treffer: nichts.txt“ ist lesbar | nicht geprüft | |
| U11 | Quellordner ohne hochzuladende Datei | „Keine Datei wird hochgeladen.“ | nicht geprüft | |
| U12 | Nach der Vorschau den Zielordner im Dateimanager ansehen | Der Zielordner ist unverändert (nur die Vorschau lief) | nicht geprüft | |

### 2.4 Bestätigung und Veraltung

| Nr. | Schritt | Erwartetes Verhalten / Ansage | Ergebnis | Beobachtung |
| --- | --- | --- | --- | --- |
| U13 | Vor der Vorschau: die Schaltfläche „Veröffentlichen“ suchen | Sie ist nicht vorhanden. Nach der Vorschau: „Veröffentlichen, nicht verfügbar“ mit dem Hinweistext „Bitte die Veröffentlichung ausdrücklich bestätigen.“ | nicht geprüft | |
| U14 | Kontrollkästchen „Ich habe die Vorschau geprüft und bestätige die Veröffentlichung.“ mit Leertaste setzen | Zustand „aktiviert“ wird angesagt; „Veröffentlichen“ ist verfügbar | nicht geprüft | |
| U15 | Nach der bestätigten Vorschau eine Ausschlussregel ändern | Der Hinweis „Die Eingaben haben sich geändert; diese Vorschau gilt nicht mehr.“ ist lesbar und wird gefunden. **Offene Frage:** Die Änderung ist nicht als Live-Region markiert; prüfen, ob NVDA sie beim Verlassen des Feldes ansagt oder nur beim Suchen findet | nicht geprüft | |
| U16 | Danach „Veröffentlichen“ erreichen | Schaltfläche nicht verfügbar; Begründung „Die Eingaben haben sich seit der Vorschau geändert …“ lesbar | nicht geprüft | |

### 2.5 Laufender Upload

| Nr. | Schritt | Erwartetes Verhalten / Ansage | Ergebnis | Beobachtung |
| --- | --- | --- | --- | --- |
| U17 | Vorschau erneut prüfen, bestätigen, „Veröffentlichen“ mit Enter | Statuszeile wechselt einmal zu „Veröffentlichung läuft … Abbruch ist möglich; …“; Spinner sichtbar | nicht geprüft | |
| U18 | Während des Laufs die Fortschrittsleiste erreichen | Text „n B von m B übertragen“ ist lesbar; **die Leiste wird nicht je Bild oder je Block angesagt** (Ansagehäufigkeit beobachten und notieren) | nicht geprüft | |
| U19 | Dateizähler und aktuelle Datei | „Dateien: k von 3 fertig.“ und „Gerade wird übertragen: big.bin“ sind lesbar; kein Ansagen-Strom | nicht geprüft | |
| U20 | Während des Laufs: Formularfelder | Alle Felder sind deaktiviert; der Fokus bleibt bedienbar | nicht geprüft | |
| U21 | „Abbrechen“ mit Tab und Enter | Statuszeile „Abgebrochen. Die bisherige Veröffentlichung bleibt unverändert.“ wird angesagt; kein Erfolg | nicht geprüft | |
| U22 | Erfolgreiche Veröffentlichung (zweiter Lauf, dann mit Änderung) | Statuszeile „Veröffentlicht: Revision r-…, n Dateien hochgeladen.“ wird angesagt; Abschnitt „Ergebnis“ ist lesbar; danach ist das Kennwortfeld **leer** | nicht geprüft | |
| U23 | Fehler auslösen (Zielordner schreibgeschützt oder Server beendet) | Statuszeile „Fehler: …“ wird angesagt; der Fehlertext steht neben dem Ergebnis; die bisherige Veröffentlichung wird korrekt beschrieben | nicht geprüft | |
| U24 | Nach dem Ende: Fortschrittsleiste | Sie verschwindet; der Fokus liegt bedienbar an einer sinnvollen Stelle (Beobachtung: wohin) | nicht geprüft | |

### 2.6 Tastatur und Fokus

| Nr. | Schritt | Erwartetes Verhalten | Ergebnis | Beobachtung |
| --- | --- | --- | --- | --- |
| U25 | Gesamte Strecke ohne Maus: Felder, Vorschau, Bestätigung, Veröffentlichen | Jeder Schritt ist erreichbar; keine Falle | nicht geprüft | |
| U26 | Sichtbarer Fokusring an jedem Element | Immer erkennbar | nicht geprüft | |
| U27 | Ansicht wechseln und zurück | Der Zustand (Formular, Vorschau) bleibt erhalten, soweit die Ansicht ihn nicht verwirft | nicht geprüft | |

## 3. Manuell: Windows, Sichtprüfung (hell und dunkel)

| Nr. | Prüfung | Ergebnis | Beobachtung |
| --- | --- | --- | --- |
| S1 | Hell und Dunkel: Fehlertext „Fehler: …“ in Rot ist lesbar und steht zusätzlich als Text da | nicht geprüft | |
| S2 | Hell und Dunkel: Fortschrittsleiste hebt sich vom Hintergrund ab, Füllung und Rest sind unterscheidbar | nicht geprüft | |
| S3 | Hell und Dunkel: deaktivierte Schaltflächen und das deaktivierte Kontrollkästchen sind als solche erkennbar | nicht geprüft | |
| S4 | Hell und Dunkel: Kontrollkästchen-Zustand (gesetzt / nicht gesetzt) ist ohne Farbe erkennbar | nicht geprüft | |
| S5 | 200 %: „Vorschau prüfen“, „Veröffentlichen“, „Abbrechen“ und Ergebniszeilen sind sichtbar; lange Pfade in der Vorschau sind vollständig erreichbar (scrollen oder umbrechen) | nicht geprüft | |
| S6 | Screenshot je Theme und Skalierung ablegen unter `docs/migration/gui-evidence/` als `upload-<system>-<theme>-<skalierung>-<sha>.png`, Build-SHA im Kopf | nicht geprüft | |

## 4. Weitere Lesegeräte und Linux

- **Narrator (Windows, optional):** Die Schritte U1 bis U24 sinngemäß. Ergebnis: nicht geprüft.
- **Orca (Linux):** Offen bis zur Linux-Abschlussvalidierung ([RB-03](RELEASE_BLOCKERS.md)). Die Schritte U1 bis U24 sinngemäß mit Orca. Ergebnis: nicht geprüft.
- **Linux-Sichtprüfung:** Offen bis zur Linux-Abschlussvalidierung. Die Prüfung aus Abschnitt 3 ist dann zu wiederholen.

## 5. Erwartete Ansage-Regeln (Maßstab für die Bewertung)

- Ansagen bei Zustandswechseln: Vorschau bereit, Veröffentlichung läuft, Abgebrochen, Veröffentlicht, Fehler.
- **Keine** Ansage je Byte, je Block oder je Bild. Fortschritt ist über Leserbefehl und Fokus erreichbar, nicht über Dauerausgabe.
- Fehler stehen neben dem betroffenen Ergebnis, nicht nur als Farbe.
- Das Kennwort wird nie vorgelesen.

## 6. Gate-Status

- Die Upload-Ansicht ist zusätzlicher Prüfbedarf für das P1-GUI-/Zugänglichkeitsgate. Sie hebt die Anforderungen des Basisprotokolls (NVDA, Orca, Sichtprüfung, ADR-0002) nicht auf.
- Die automatisierten Tests aus Abschnitt 1 erfüllen das Gate **nicht**.
- Das Gate bleibt bis zur ausgefüllten, mit Build-SHA und Datum versehenen Protokollfassung auf beiden Systemen geschlossen (RB-02).
