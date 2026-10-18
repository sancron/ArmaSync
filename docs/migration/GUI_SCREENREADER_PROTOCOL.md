# Manuelles Prüfprotokoll: Screenreader, Tastatur, Darstellung

Zweck: Belege für das P1-GUI-/Accessibility-Gate ([ROADMAP](ROADMAP.md), [UI_REQUIREMENTS](UI_REQUIREMENTS.md)). **Stand: nicht ausgeführt.** Die Abschnitte B, C und D sind bewusst leer; erst ausgefüllte, mit Build-SHA und Datum versehene Protokolle auf **beiden** Systemen erfüllen das Gate. Dieses Dokument ändert nichts am Gate-Status.

Gegenstand ist der Prototyp aus MIG-012 (`armasync-gui`, simulierte In-Memory-Daten). Keine Aussage über spätere Ansichten.

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

## E. Gate-Kriterien (Erinnerung)

Das P1-GUI-Gate gilt erst, wenn zusammen vorliegen: ausgefülltes NVDA-Protokoll (Windows) **und** Orca-Protokoll (Linux) für das gewählte Backend, ohne offene Befunde der Schwere „blockiert Kernablauf“; Sichtprüfung nach D; ADR-0002 von „vorläufig gewählt“ auf „angenommen“ gesetzt; unveränderte oder dokumentiert akzeptierte Befunde in der Evidenzdatei. Den Gate-Status setzt der Integrator in `STATUS.md`/`ROADMAP.md`, nicht dieses Dokument.
