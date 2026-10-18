# GUI-Prototyp: Prüfprotokoll (MIG-012)

Stand: 2026-10-18. Code-Stand der Prüfung: Commit `ae5d466` auf `claude/mig-012-gui-prototype` (Prototyp `crates/armasync-gui`). Entscheidungskontext: [ADR-0002](../adr/0002-gui-toolkit.md), Anforderungen: [UI_REQUIREMENTS](UI_REQUIREMENTS.md).

**Kurzfassung:** Der Prototyp läuft nativ unter Windows 11 und baut/testet unter Linux. Der Zugänglichkeitsbaum wurde automatisiert (AccessKit-Harness) und live über Windows UI Automation geprüft. **NVDA und Orca wurden nicht ausgeführt** (nicht installiert bzw. nicht lauffähig, siehe „Lücken“). Das P1-Gate für GUI/Accessibility ist deshalb **nicht erfüllt**; ADR-0002 bleibt „vorläufig gewählt“.

## Prototypumfang

Repositoryliste mit 10.000 synthetischen Zeilen (lange Namen, Filter, Sortierung, virtualisiert), Auswahl per Maus/Tastatur, Dialog „Repository hinzufügen“ mit Feldfehlern, Fortschritt mit Abbrechen, Fehlerzusammenfassung, Theme (System/Hell/Dunkel) und Skalierung (100–200 %). Die Synchronisation läuft über denselben Use-Case `armasync_application::sync` wie in der CLI, aber auf **simulierten In-Memory-Daten** (40 ms Verzögerung je Datei; ein Teil der Repositories liefert absichtlich beschädigte Daten bzw. ist „nicht erreichbar“). Es gibt keinen echten Transport und keinen Dateisystemzugriff; das Fenster weist im Prototyphinweis darauf hin. Die Logik liegt in einem anzeigefreien View-Model (`model.rs`), gezeichnet wird in `ui.rs`; die Arbeit läuft auf einem Worker-Thread, der UI-Thread pollt nur.

## Toolkit-Entscheidung (vorläufig)

| Kriterium aus ADR-0002 | Befund egui/eframe 0.37 + AccessKit |
| --- | --- |
| Native Rust-GUI, kein Web-Frontend | ja (winit, glow/wgpu); keine WebView |
| Accessibility-Baum für Standardcontrols | ja: Button, ComboBox, TextInput, PasswordInput, Dialog, ProgressIndicator; Namen über `labelled_by` |
| Eigene Tabelle/Liste | `egui_extras::TableBuilder` liefert keine Tabellenrollen; Zeilen wurden als `ListBoxOption` mit Auswahlzustand und Position „n von m“ ausgezeichnet (Windows: `ListItem` + SelectionItemPattern) |
| Virtuelle Liste, 10.000 Zeilen | ja, nur sichtbare Zeilen existieren als Knoten; durchschnittlich ca. 3 ms je Frame (Debug-Build, headless) |
| Keine blockierende I/O im UI-Thread | ja (Worker-Thread, `poll()` je Frame); Test: Drop des Modells bricht den Worker ab und wartet |
| Lizenzen/Systembedarf | Abhängigkeiten überwiegend MIT/Apache-2.0, Details unten; Linux-Start braucht weitere Systembibliotheken (siehe „Lücken“) |
| CLI bleibt unabhängig | ja: `cargo tree -p armasync-cli` enthält keine GUI-Crates |
| Alternativen (Slint, Tauri, Iced) | **nicht prototypisiert**; ADR-0002 sieht Alternativen erst bei Scheitern des Gates vor |

Weil der Screenreader-Nachweis fehlt, ist egui/eframe die **vorläufige** Wahl, nicht die angenommene. egui 0.37 verlangt Rust ≥ 1.95; deshalb hat nur `armasync-gui` `rust-version = "1.95"`, der übrige Workspace bleibt bei MSRV 1.85 (siehe [RUST_TOOLCHAIN](RUST_TOOLCHAIN.md)).

## Ergebnisse

### Automatisierte Tests (`crates/armasync-gui`: 29 Tests, davon 12 View-Model und Kontrasthilfe, 17 UI/Accessibility)

Der Harness (`egui_kittest`) prüft nach jedem Lauf, dass jedes Eingabeelement einen zugänglichen Namen hat, und fragt den **AccessKit-Baum** ab – denselben Baum, den NVDA (über UI Automation) und Orca (über AT-SPI) erhalten. Geprüft werden unter anderem:

* Rollen, Namen, Zustände: Buttons `Repository hinzufügen…`, `Synchronisieren` (deaktiviert ohne Auswahl), `Abbrechen` (deaktiviert ohne Job); Textfeld `Filter:`; ComboBoxen `Darstellung:` und `Skalierung:`; Listenzeilen als `ListBoxOption` mit `selected`, `position_in_set`, `size_of_set`.
* Dialog: Rolle `Dialog` mit Name „Repository hinzufügen“, Felder `Name:`, `Adresse:`, `Benutzername (optional):`, `Kennwort (optional):` (Rolle `PasswordInput`); Feldfehler als Text „Fehler: …“ neben dem Feld; Escape schließt und gibt den Fokus an den Öffner zurück; Zugangsdaten in der Adresse werden mit Erklärung abgelehnt; das eingegebene Kennwort taucht in keinem Knoten (Name oder Wert) auf.
* Fortschritt als `ProgressIndicator` mit Text („n von m Dateien, …“); Abbrechen endet im Zustand „Abgebrochen“ (nicht „Erfolg“); der Fehlerzustand nennt Repository, Ursache und nächsten Schritt; „Anmeldedaten erforderlich“ startet keinen Job.
* Tastatur: Tab-Reihenfolge Hinzufügen → Darstellung → Skalierung → Filter → Sortierbuttons → Liste; Pfeiltasten ändern die Auswahl; der Fokus ist immer bekannt.
* Layout: alle Kernaktionen vollständig sichtbar bei 1024×720 @ 200 %, 640×480 @ 200 %, 1920×1080 @ 150 % und 640×480 @ 100 %.
* Kontrast (WCAG 2.x, berechnet aus den Theme-Farben):

| Theme | Text | schwacher Text | Fehlertext | Text auf Auswahl | Fokus-/Auswahlring | Widget-Umriss | Auswahlfläche |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Hell | 7,59 | 6,49 | 6,48 | 5,65 | 17,92 | 4,80 | 3,17 |
| Dunkel | 5,12 | 8,31 | 7,55 | 4,68 | 17,22 | 6,12 | 3,68 |

Beim Prüfen fielen zwei Abweichungen der egui-Standardfarben auf und wurden korrigiert: schwacher Text im dunklen Theme nur 2,27:1 (jetzt über `themed_visuals`) und Widget-Umrisse im dunklen Theme 1,22:1. **Nachbesserung MIG-012A (2026-10-18):** Die Auswahlfläche im hellen Theme war zunächst nur ein heller Farbton (1,56:1, Soll 3:1) und als Ausnahme geführt. Sie ist jetzt `#5A8FCC` mit Text und Ring `#101010` (Tabelle oben; Werte aus dem Test `colours_meet_wcag_aa`, der die Ausnahme nicht mehr enthält). egui zeichnet Text in ausgewählten Zeilen und den Fokusring mit derselben Farbe (`selection.stroke`); deshalb liegt die Lösung in einem schmalen Band (Füllung 3:1 zur Fläche *und* Text 4,5:1 auf der Füllung). Der Zustand wird weiterhin zusätzlich durch Fettschrift und den Zugänglichkeitszustand `selected` vermittelt. Berechnet und im Harness geprüft, **nicht** am realen Bildschirm begutachtet (Prüfschritt im [Screenreader-Prüfprotokoll](GUI_SCREENREADER_PROTOCOL.md)).

Beim Testen entdeckte Probleme (alle behoben): (1) Die egui-Standardschrift enthält viele Symbole nicht (▲ ▼ führten zum Abbruch „No glyph“), deshalb nutzt die UI nur Text. (2) `selectable_label` wird von egui als gedrückter Schalter gemeldet, nicht als ausgewähltes Listenelement; korrigiert über eine eigene AccessKit-Auszeichnung. (3) Der Dialog hatte zunächst keinen Namen. (4) Die Tabellenspalte „Dateien“ war abgeschnitten (Screenshot-Befund). (5) Die Beschriftung von Darstellung/Skalierung stand hinter dem Auswahlfeld.

Auszug des Baums im Startzustand (AccessKit, Rolle | Name):

```text
Button | Repository hinzufügen…
Button | Synchronisieren   (deaktiviert)
Button | Abbrechen         (deaktiviert)
ComboBox | Darstellung:    (Wert „System“)
ComboBox | Skalierung:     (Wert „100 %“)
TextInput | Filter:
Button | Name (aufsteigend) · Button | Status · Button | Dateien
ListBoxOption | Repository 00000 – Community Modpack mit sehr langem Anzeigenamen für Layouttests
ListBoxOption | Repository 00001 …
```

Dialog geöffnet: `Dialog | Repository hinzufügen` mit `TextInput | Name:` (fokussiert), `TextInput | Adresse:`, `TextInput | Benutzername (optional):`, `PasswordInput | Kennwort (optional):`, `Button | Hinzufügen`, `Button | Abbrechen`; der Hintergrund ist deaktiviert.

### Live-Prüfung unter Windows 11 (UI Automation, Build `914e022` bzw. `ae5d466`)

Die laufende `armasync-gui.exe` wurde über die .NET-UI-Automation-API abgefragt (dieselbe Schnittstelle, die NVDA nutzt). Umgebung: Windows 11 10.0.26200, 100 % Skalierung, Dunkel (System).

* Fenster `ArmaSync – Lynxnode.it` (Window) mit 91 Nachfahren; Buttons, ComboBoxen (`Darstellung:`, `Skalierung:`), Edit `Filter:`, ein ListItem je Zeile; `Synchronisieren` und `Abbrechen` korrekt deaktiviert.
* Auswahl über `SelectionItemPattern.Select()`: `IsSelected = True`, danach ist `Synchronisieren` aktiviert.
* Der Dialog erscheint als UIA-`Window` „Repository hinzufügen“; die Edit-Felder sind benannt, das Kennwortfeld hat `IsPassword = True`; beim Öffnen steht der Fokus im Feld `Name:`; nach Escape hat `Repository hinzufügen…` wieder den Tastaturfokus.
* Der Fortschritt erscheint als UIA-`ProgressBar` (Pattern `RangeValue`) mit Namen. Verlauf „2 von 11, 6 von 11, 9 von 11, 11 von 11 Dateien“ bei bedienbarer UI; beim Abbruch „Abgebrochen“ plus Hinweis „bereits übertragene Dateien sind vollständig“; eine beschädigte Übertragung ergibt „Fehler (1)“ mit Repository und nächstem Schritt.

Screenshots (Windows 11, Dunkel, 100 %): [Hauptfenster](gui-evidence/main-windows11-dark-100pct.png), [Synchronisation läuft](gui-evidence/sync-progress-windows11-dark-100pct.png), [Dialog](gui-evidence/dialog-windows11-dark-100pct.png).

## Lücken (nicht behauptet) und reproduzierbare manuelle Prüfschritte

1. **NVDA (Windows) wurde nicht ausgeführt.** NVDA ist nicht installiert (`C:\Program Files (x86)\NVDA` und `C:\Program Files\NVDA` fehlen); eine Installation erfolgte ohne Freigabe nicht. Die UI-Automation-Prüfung belegt die Schnittstelle, nicht die tatsächliche Ansage.
   *Manueller Schritt (ausführliches Protokoll: [GUI_SCREENREADER_PROTOCOL](GUI_SCREENREADER_PROTOCOL.md)):* NVDA installieren oder portabel starten, `target\debug\armasync-gui.exe` öffnen und mit Tab, Umschalt+Tab und Pfeiltasten prüfen: (a) Toolbar-Buttons werden mit Namen und „nicht verfügbar“ angesagt; (b) eine Listenzeile wird mit Namen, „ausgewählt“ und Position angesagt; (c) der Dialog wird als „Repository hinzufügen, Dialog“ angesagt, Felder mit Beschriftung, das Kennwortfeld ohne Inhalt, der Fehlertext nach leerem Absenden; (d) Escape setzt den Fokus auf „Repository hinzufügen…“ zurück; (e) Fortschritt ist als Fortschrittsanzeige ansagbar, Phasenwechsel, Fehler und Abbruch erscheinen als Text; (f) Skalierung 200 % über das Auswahlfeld. Ergebnis mit NVDA-Version, Build-SHA und Datum hier eintragen.
2. **Orca (Linux) wurde nicht ausgeführt.** Orca ist in WSL nicht installiert. Außerdem **startet der Prototyp unter WSLg nicht**: Der Wayland-Pfad endet mit `Io error: Connection reset by peer` und `WinitEventLoop(ExitFailure(1))`, der X11-Pfad mit `Library libxkbcommon-x11.so could not be loaded` (Systembibliothek fehlt). Ein AT-SPI-Abbild der laufenden Anwendung liegt deshalb nicht vor. Nachgewiesen ist unter Linux nur: Build, `clippy -D warnings` und alle Tests einschließlich der AccessKit-Tests (headless).
   *Manueller Schritt:* auf einem Linux-Desktop mit laufender AT-SPI-Sitzung (GNOME unter Wayland oder X11 mit `libxkbcommon-x11`) `cargo run -p armasync-gui` starten, Orca aktivieren und die Punkte (a) bis (f) wiederholen; zusätzlich mit `accerciser` den Baum prüfen.
3. **Nicht geprüft:** Kontrast und Skalierung am realen Bildschirm für das helle Theme und 200 % (nur berechnet bzw. im Harness), Hochkontrast- und Reduzierte-Bewegung-Systemmodi, macOS, Sprachen außer Deutsch. Die Liste ist als Listbox mit Spaltentext modelliert, nicht als Tabelle: Status und Dateizahl stehen als eigene Textknoten neben dem Zeilennamen und werden von einem Screenreader möglicherweise nicht als Teil derselben Zeile angesagt (mit NVDA/Orca prüfen, ggf. Statusinformation in den Zeilennamen aufnehmen).
4. Fortschrittsansagen sind nicht gedrosselt (der Fortschrittstext wird je Datei aktualisiert); bei echten Übertragungen mit vielen Dateien ist eine Drosselung nötig.

## Abhängigkeiten und Lizenzen

`armasync-gui` zieht über eframe/egui rund 400 Pakete in den Abhängigkeitsbaum (`cargo tree -p armasync-gui --target all`, einschließlich Plattformpaketen für macOS/Wasm, die auf Windows und Linux nicht gebaut werden). Die vollständige Liste mit Version und Lizenz steht in [gui-evidence/gui-dependencies.txt](gui-evidence/gui-dependencies.txt). Lizenzen laut Crate-Metadaten: überwiegend MIT/Apache-2.0 (teils zusätzlich Zlib/LLVM-exception), außerdem Unicode-3.0 (ICU-Daten), BSD-2/3-Clause, ISC, BSL-1.0, Zlib, `Apache-2.0 OR GPL-2.0-only` (`self_cell`, Apache wählbar) und `epaint_default_fonts` mit `(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0`: Die mitgelieferten egui-Standardschriften erfordern beim Ausliefern Notice-Texte (RQ-06 bleibt offen). Es wurden keine Altassets (Icons, Grafiken, Schriften) aus dem Bestand verwendet (RQ-04); das Fenster hat keine eigenen Icons. Das ist keine Rechtsprüfung.
