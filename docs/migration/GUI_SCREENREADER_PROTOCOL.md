# Manuelles Prüfprotokoll: Screenreader, Tastatur, Darstellung

Zweck: Belege für das P1-GUI-/Accessibility-Gate ([ROADMAP](ROADMAP.md), [UI_REQUIREMENTS](UI_REQUIREMENTS.md)). **Stand: nicht ausgeführt.** Die Abschnitte B, C und D sind bewusst leer; erst ausgefüllte, mit Build-SHA und Datum versehene Protokolle auf **beiden** Systemen erfüllen das Gate. Dieses Dokument ändert nichts am Gate-Status.

Gegenstand sind (1) der Prototyp aus MIG-012 (Ansicht „Repository-Liste (Prototyp, simuliert)“, Abschnitte A–D) und (2) die **Synchronisationsansicht aus MIG-031** (Abschnitt F, echter Ablauf gegen einen lokalen Testserver). **Beide sind nicht durch NVDA, Orca oder eine Sichtprüfung geprüft; ausgefüllte Ergebnisspalten gibt es nicht.**

## A. Vorbereitung

1. Build-SHA festhalten: `git rev-parse HEAD`; Build: `cargo build -p armasync-gui` (Windows: `target\debug\armasync-gui.exe`, Linux: `target/debug/armasync-gui`).
2. **Windows:** NVDA ([nvaccess.org](https://www.nvaccess.org/)) installieren oder die portable Version starten; Version notieren. Sprachausgabe laufen lassen; optional Sprachanzeige (NVDA-Menü → Extras → Sprachausgabe-Anzeige) zum Mitschreiben der Ansagen aktivieren.
3. **Linux:** Desktop mit laufender AT-SPI-Sitzung (GNOME unter Wayland oder X11), Orca (`orca`, Umschalt+Super+S oder `orca --replace`), unter X11 wird `libxkbcommon-x11` benötigt (Debian/Ubuntu: `sudo apt install libxkbcommon-x11-0`). Zusätzlich `accerciser` zum Betrachten des Zugänglichkeitsbaums. Hinweis: Unter WSLg startet der Prototyp nach bisherigem Befund nicht; auf einem echten Desktop oder einer VM testen.
4. Darstellung für die Durchläufe: Windows 100 % und 200 % Systemskalierung bzw. Skalierung „200 %“ im Programm, Theme Hell und Dunkel, optional Windows-Kontrastdesign.

## B. Ablauf (je System, je Screenreader durchführen)

Erwartung gilt sinngemäß; die genaue Formulierung hängt vom Screenreader ab. Abweichung = Befund, mit Wortlaut notieren.

| Nr. | Schritt | Erwartete Ansage / erwartetes Verhalten | Ergebnis (ok / Befund) | Notiz |
| --- | --- | --- | --- | --- |
| 1 | Programm starten | Fenstertitel „ArmaSync – Lynxnode.it“; Fokus liegt auf einem bedienbaren Element | | |
| 2 | Mit Tab durch die Toolbar | „Repository hinzufügen…, Schaltfläche“; „Synchronisieren, Schaltfläche, nicht verfügbar“ (ohne Auswahl); „Abbrechen, … nicht verfügbar“; „Darstellung:“ und „Skalierung:“ als Auswahlfelder mit aktuellem Wert; „Filter:“ als Eingabefeld | | |
| 3 | Filter: „00012“ tippen | Eingabe wird angesagt; Liste verkürzt sich; Ergebnisanzahl ist erkennbar (Befund notieren, falls nicht angesagt) | | |
| 4 | Sortierschaltflächen | Name von Spalte und Richtung wird angesagt (z. B. „Name (aufsteigend)“) | | |
| 5 | In die Liste, Pfeiltasten ab/auf | Je Zeile: Name, „ausgewählt“, Position („3 von n“). Prüfen, ob Status und Dateizahl der Zeile mit angesagt werden oder fehlen (bekannte Hypothese, siehe Evidenzdatei) | | |
| 6 | Bild ab / Bild auf, Pos1/Ende falls vorhanden | Auswahl springt um zehn Zeilen bzw. an den Rand; Ansage folgt der Auswahl | | |
| 7 | „Synchronisieren“ per Tastatur (Leertaste/Enter) | Schaltfläche ist nun verfügbar; Phasenwechsel („Prüfen“, „Herunterladen“ …) werden als Text angesagt; Fortschritt als Fortschrittsanzeige mit „n von m Dateien“; keine Ansage-Flut (Befund notieren, wenn jede Datei angesagt wird) | | |
| 8 | „Abbrechen“ während der Synchronisation | „Abgebrochen“ wird angesagt, nicht als Erfolg; Hinweis auf vollständig übertragene Dateien | | |
| 9 | Fehlerfall auslösen: im Filter „00003“ eingeben, „Repository 00003“ wählen und synchronisieren (simulierte beschädigte Übertragung); zusätzlich ein Repository mit Status „Anmeldedaten erforderlich“ synchronisieren | Fehlermeldung nennt Repository, Ursache und nächsten Schritt; Text ist ansagbar | | |
| 10 | „Repository hinzufügen…“ öffnen | „Repository hinzufügen, Dialog“; Fokus im Feld „Name:“; Hintergrund nicht bedienbar | | |
| 11 | Dialogfelder mit Tab | „Name:“, „Adresse:“, „Benutzername (optional):“, „Kennwort (optional):“ (Kennwortfeld ohne Inhaltsansage), „Hinzufügen“, „Abbrechen“; Fokus bleibt im Dialog | | |
| 12 | Leer absenden, dann Adresse mit Zugangsdaten (`https://user:pw@host/`) | Fehlertext steht neben dem Feld und wird angesagt; Zugangsdaten werden mit Erklärung abgelehnt; das Kennwort wird nirgends vorgelesen | | |
| 13 | Escape im Dialog | Dialog schließt; Fokus liegt wieder auf „Repository hinzufügen…“ und wird angesagt | | |
| 14 | Skalierung „200 %“ wählen | Alle Kernaktionen bleiben sichtbar und bedienbar; kein unzugängliches Abschneiden langer Namen (Tooltip/Volltext erreichbar) | | |
| 15 | Theme „Hell“ wählen (siehe Abschnitt D) | Darstellung lesbar; ausgewählte Zeile deutlich erkennbar | | |

## C. Kopf des Protokolls (je Durchlauf ausfüllen)

| Feld | Wert |
| --- | --- |
| Datum / Prüfer | |
| Build-SHA | |
| Betriebssystem + Version, Skalierung | |
| Screenreader + Version (NVDA / Orca) | |
| Sprachausgabe/Sprache | |
| Anzeigeserver (Windows / Wayland / X11) | |
| Ergebnis gesamt (bestanden / Befunde) | |
| Befunde (jeweils Schritt-Nr., Wortlaut, Schwere) | |

## D. Darstellung und Kontrast (Sichtprüfung)

Die Kontrastwerte sind berechnet (Tabelle in [GUI_PROTOTYPE_EVIDENCE](GUI_PROTOTYPE_EVIDENCE.md)); folgende Punkte erfordern die Sichtprüfung am realen Bildschirm:

1. Hell: ausgewählte Zeile (blaue Füllung, dunkler Text) deutlich von nicht ausgewählten Zeilen unterscheidbar; Fokusring des Filterfeldes sichtbar; deaktivierte Schaltflächen als solche erkennbar.
2. Dunkel: dasselbe wie 1.
3. 200 %: Tabellenkopf, Status und Dateizahl nicht abgeschnitten.
4. Optional: Windows-Kontrastdesign bzw. erzwungene Farben; Reduzierte Bewegung. Ergebnis ehrlich als „nicht unterstützt“ oder „unterstützt“ eintragen (der Prototyp wertet beides nicht gezielt aus).
5. Screenshot ablegen unter `docs/migration/gui-evidence/` mit Dateiname `<system>-<theme>-<skalierung>-<sha>.png`.

## F. Synchronisationsansicht (MIG-031): NVDA-Schritte und Sichtprüfung

**Stand: nicht ausgeführt.** Automatisiert belegt sind der AccessKit-Baum (Namen, Rollen, Zustände, Live-Region) und einmal die Bedienung über Windows UI Automation ([GUI_SYNC_EVIDENCE](GUI_SYNC_EVIDENCE.md)); **was NVDA tatsächlich ansagt, ist unbelegt.** Dieser Abschnitt ist die Anleitung für die manuelle Prüfung; die Ergebnisspalten bleiben leer, bis jemand sie wirklich ausfüllt. Ihr Ausfüllen allein erfüllt das P1-Gate nicht (siehe E).

### F.1 Vorbereitung (Windows, NVDA)

1. Build-SHA notieren (`git rev-parse HEAD`), `cargo build -p armasync-gui -p armasync-cli` und `cargo build -p armasync-testkit --example serve` (der Beispielserver liegt in `armasync-testkit`).
2. Testserver starten: `target\debug\examples\serve.exe` (für Schritt F14: `serve.exe trickle`, die große Datei kommt dann langsam, etwa zwei Sekunden) – er schreibt die Adresse (`http://127.0.0.1:<port>/repo`) und läuft bis zum Beenden. Er liefert die synthetischen MIG-001-Metadaten, keine echten Daten.
3. Testbibliothek anlegen: `import.json` mit dieser Adresse (siehe Beispiel in `crates/armasync-cli/tests/sync_e2e.rs`, Funktion `env_for`) und `armasync import import.json --apply --config "%APPDATA%\Lynxnode.it\ArmaSync"`. Ein leerer Zielordner, z. B. `C:\Temp\armasync-ziel`, wird angelegt.
4. NVDA starten (Version notieren), Sprachanzeige zum Mitschreiben aktivieren, `target\debug\armasync-gui.exe` starten.

### F.2 Ablauf (NVDA; je Schritt Ansage notieren)

| Nr. | Schritt | Erwartete Ansage / erwartetes Verhalten | Ergebnis (ok / Befund) | Notiz |
| --- | --- | --- | --- | --- |
| F1 | Start | Ansicht „Synchronisation“ ist aktiv („Ansicht:“ mit den zwei Schaltflächen); Plattformhinweis „Plattform: Windows: … mit einem lokalen Testserver geprüft … noch nicht geprüft“ ist lesbar | | |
| F2 | Status lesen | „Status: Bereit. Repository und Zielordner wählen.“ | | |
| F3 | Tab zu „Repository:“ | „Repository:, Auswahlfeld, Example“ (bei genau einem Repository vorgewählt); Adresse wird als Text gelesen | | |
| F4 | Tab zu „Zielordner (muss vorhanden sein):“, Pfad eintippen | „Zielordner (muss vorhanden sein):, Eingabefeld“; Eingabe wird angesagt | | |
| F5 | Ohne Zielordner „Änderungen prüfen“ drücken (Test der Fehlermeldung) | Fehler „Fehler: Bitte den Zielordner angeben.“ steht neben dem Feld und wird angesagt (nicht nur als Tooltip) | | |
| F6 | „Änderungen prüfen“ mit gültigem Ordner | Status wechselt zu „Änderungen werden geprüft.“, danach „Vorschau bereit: 2 neu, 0 ersetzen, 0 löschen.“ – **als Live-Region angesagt**, ohne den Fokus zu verschieben | | |
| F7 | Vorschau lesen (Pfeiltasten/Lesemodus) | Überschrift „3. Vorschau“; Summenzeilen („Neu laden: 2 Dateien (8.0 KiB)“ …); Tabelle mit Aktion in Worten („Neu laden“) vor jedem Pfad; Hinweis zur komprimierten Datei; „Bleibt unverändert (2)“ als auf-/zuklappbare Schaltfläche | | |
| F8 | „Synchronisieren“ ansteuern | „Synchronisieren, Schaltfläche“ (verfügbar erst nach der Vorschau); Enter löst aus | | |
| F9 | Während der Übertragung | Phase und Fortschritt als Fortschrittsanzeige „n von m Dateien, … – Phase“; **keine Ansage-Flut** je Datei/Byte; „Abbrechen, Schaltfläche“ erreichbar | | |
| F10 | Ergebnis | „Status: Abgeschlossen: 2 geladen, 0 ersetzt, 0 gelöscht.“ wird angesagt; Abschnitt „Ergebnis“ mit Zahlen und Hinweis zur Prüfsummenprüfung | | |
| F11 | „Zurück zur Auswahl“, Löschen testen: im Zielordner eine zusätzliche Datei `@example_core\\addons\\extra.pbo` anlegen; Checkbox „Zusätzliche Dateien im Zielordner löschen“ einschalten | Checkbox mit Zustand „nicht aktiviert/aktiviert“; Feld „Löschen nur in diesen Ordnern (…)“ erscheint und wird erreichbar angekündigt | | |
| F12 | Ordner `@example_core` eintragen, „Änderungen prüfen“ | Abschnitt „Löschungen bestätigen“; „Achtung: Diese 1 Dateien werden endgültig gelöscht:“ und die Dateiliste werden gelesen; „Synchronisieren“ ist **„nicht verfügbar“** mit Hinweis „Nicht möglich: Bitte zuerst die Löschungen bestätigen.“ | | |
| F13 | Bestätigungs-Checkbox „Ich habe die 1 Löschungen geprüft …“ aktivieren, dann „Synchronisieren“ | Schaltfläche wird verfügbar; nach dem Lauf „… 1 gelöscht“; geschützte/komprimierte Dateien bleiben (im Dateisystem prüfen) | | |
| F14 | Abbruch: den Testserver mit `serve.exe trickle` starten, die Synchronisation beginnen und sofort „Abbrechen“ wählen | „Abbruch angefordert: …“ wird angesagt, danach „Abgebrochen“ (nicht als Erfolg); Hinweis, dass vorhandene Dateien unverändert sind | | |
| F15 | Fehlerfall: Testserver beenden, erneut „Änderungen prüfen“ | „Fehler bei „Example“:“, Ursache und nächster Schritt werden gelesen; Fokus bleibt bedienbar; „Zurück zur Auswahl“ erreichbar | | |
| F16 | Skalierung „200 %“ | Alle Schritte F4–F13 bleiben ohne unzugängliches Abschneiden bedienbar; lange Pfade vollständig erreichbar | | |

### F.3 Linux (Orca) – nur Plan

Voraussetzung: Linux-Abschlussvalidierung (RB-03). Auf Linux ist die Synchronisation absichtlich gesperrt (RB-01): geprüft wird, dass der Banner „Eingeschränkt: Linux: die Synchronisation ist noch nicht freigegeben …“ gelesen wird, „Änderungen prüfen“ den Plan zeigt, „Synchronisieren“ **„nicht verfügbar“** ist mit „Nicht möglich: Auf diesem System ist das Synchronisieren noch nicht freigegeben.“ und nichts im Zielordner geändert wird. Die übrigen Schritte F1–F10 sinngemäß mit Orca; Befunde in Abschnitt C eintragen.

### F.4 Sichtprüfung der Synchronisationsansicht (offen)

1. Hell und Dunkel: Überschriften, Plattformhinweis, „Achtung“-Text (Fehlerfarbe, zusätzlich als Text), deaktivierte Schaltflächen und der Fokusring der Textfelder erkennbar; die Tabelle „Aktion/Datei/Größe“ lesbar; die Statusinformation steckt in Worten, nicht nur in Farbe.
2. 1024×720 und 1920×1080 bei 100/150/200 %: kein Verschwinden von „Änderungen prüfen“, „Synchronisieren“, „Abbrechen“ und der Fehlermeldungen; vertikales Scrollen der Ansicht funktioniert (Tastatur: Bild ab/auf bzw. Tab).
3. Lange Pfade/Namen in Vorschau und Löschliste: nicht unzugänglich abgeschnitten.
4. Screenshots ablegen unter `docs/migration/gui-evidence/` (`sync-<system>-<theme>-<skalierung>-<sha>.png`), mit Build-SHA im Protokollkopf (Abschnitt C).

## E. Gate-Kriterien (Erinnerung)

Das P1-GUI-Gate gilt erst, wenn zusammen vorliegen: ausgefülltes NVDA-Protokoll (Windows) **und** Orca-Protokoll (Linux) für das gewählte Backend, ohne offene Befunde der Schwere „blockiert Kernablauf“; Sichtprüfung nach D; ADR-0002 von „vorläufig gewählt“ auf „angenommen“ gesetzt; unveränderte oder dokumentiert akzeptierte Befunde in der Evidenzdatei. Den Gate-Status setzt der Integrator in `STATUS.md`/`ROADMAP.md`, nicht dieses Dokument.
