# GUI-, Bedienungs- und Accessibility-Anforderungen

Planungsbrief, keine fertig implementierte oder visuell abgenommene GUI. Verbindlich sind moderne, intuitive und zugängliche Bedienung sowie Lynxnode.it-Branding. Konkrete visuelle Ausarbeitung und Toolkitwahl werden in P1 anhand eines lauffähigen Desktop-Prototyps entschieden; keine unbestätigten Logos oder Farbwerte als bestehendes Designsystem ausgeben.

## Leitvorgabe: vertraute Abläufe, moderne Bedienung (Projektverantwortlicher, 2026-10-18)

Die ArmaSync-GUI orientiert sich an den **vertrauten Abläufen und der Informationsstruktur der ursprünglichen Anwendung** (Legacy-Baseline), soll dabei aber **moderner, intuitiver, angenehmer und zugänglicher** werden. Nutzer sollen bekannte Aufgaben wiederfinden (Repository hinzufügen und prüfen, Änderungen ansehen, synchronisieren, Profile/Gruppen verwalten, unter Windows starten, Einstellungen, Administration), aber **klarere Navigation, verständliche Beschriftungen, bessere Rückmeldungen und zugängliche Zustände** erhalten.

Folgerungen für jeden GUI-Task (MIG-012 und alle späteren, insbesondere MIG-031, MIG-043, MIG-050, MIG-053):

- **Aufgabenparität vor Layoutkopie:** Eine Aufgabe der Baseline darf nicht ohne dokumentierte Entscheidung entfallen oder schwerer erreichbar werden. Reihenfolge, Gruppierung und Begriffe der Baseline dienen als Ausgangspunkt, werden aber nur übernommen, wenn sie verständlich sind; Abweichungen kurz begründen (Paritätsmatrix bzw. Report).
- **Besser statt gleich:** Klare Hauptaktion je Ansicht, sprechende Beschriftungen statt Fachjargon, sichtbarer Zustand und Rückmeldung nach jeder Aktion (Phase, Ergebnis, nächste sichere Handlung), Fehler neben dem betroffenen Objekt, Status durch Text und Symbol.
- **Zugänglichkeit ist Teil der Vorgabe**, nicht nachgelagert: Kriterien siehe unten; NVDA/Orca-Nachweis bleibt Gate.
- **Keine Übernahme von Altmarken, Classnames, Team-/Länderpräfixen oder ungeklärten Grafiken, Icons und Texten** (RQ-04, [BRANDING](BRANDING.md)). Vertraut heißt Ablauf und Struktur, nicht Optik oder Quelltext der Baseline. Die Baseline dient nur als Verhaltens- und Strukturquelle; Wortlaut und Oberflächentexte werden neu formuliert.
- Der Abgleich neuer Ansichten mit der Baseline erfolgt anhand des Verhaltens (Quellcode, Fixtures, Screenshots eigener Testläufe), nicht aus alter Dokumentation.

## Aufgaben und Informationsarchitektur

Die Hauptaufgabe ist ein verifizierter synchroner Modbestand mit nachvollziehbaren Änderungen. Hauptnavigation: **Repositories**, **Synchronisation**, **Profile & Start** unter Windows beziehungsweise **Profile** unter Linux, **Administration**, **Einstellungen**, **Hilfe & Lizenzen**. Administrationsoptionen nicht in jeden Einsteigerablauf mischen; Fähigkeiten über denselben Core anbieten.

Der wichtigste Ablauf:

1. Repository hinzufügen/importieren; URL und Credentials getrennt eingeben.
2. Zielordner wählen; Platzbedarf, Schreibbarkeit und Pfadkonflikte prüfen.
3. Änderungsvorschau: neu, ersetzt, unverändert, optional gelöscht; Größen und Folgen erklären.
4. Synchronisation starten; Phase, übertragene/gesamte Bytes, Geschwindigkeit und Abbruch anzeigen. ETA nur bei belastbarer Messung.
5. Verifiziertes Ergebnis oder konkrete Fehler mit Wiederholen/Diagnose; erst danach optional Windows-Spielstart.

Keine Einstellungsänderung ohne erkennbare Wirkung. Fehler benennen Objekt, Ursache, sichere nächste Handlung; technische Details ausklappbar. Secrets weder in Fehlerdialogen noch in exportierten Diagnosen anzeigen.

## Layout und visuelle Richtung als Vorschlag

Ruhige Arbeitsoberfläche mit klarer Hauptaktion, lesbarer Typografie, konsistenten Abständen und reduzierter Dekoration. Sidebar plus aufgabenbezogener Inhalt; Repositorydetail mit klarer Status-/Aktionszeile und Datei-/Änderungsliste. Keine dekorativen Kennzahlkarten ohne Entscheidungsnutzen.

Light, Dark und Systemeinstellung anbieten; Auswahl persistieren. Status durch Text und Symbol erklären, nicht nur durch Farbe. Typografie und Abstände als wiederverwendbare Tokens, System-/frei lizenzierte Fonts; keine alten Teamgrafiken übernehmen. Lynxnode.it in Produkt-/Publisherflächen, Produktname ArmaSync im Fenstertitel. Keine erfundene Marketingbotschaft.

Zielgrößen für P1-Abnahme: 1024×720 und 1920×1080 logische Arbeitsflächen sowie 100/150/200 % Skalierung. Bei knapper Fläche Navigation einklappen und Tabellen sinnvoll scrollen; wichtige Aktionen/Fehler dürfen nicht verschwinden. Keine mobile App als implizites Ziel.

## Verbindliche Zugänglichkeitskriterien

- Alle Kernabläufe nur mit Tastatur möglich: Repository hinzufügen, Ziel wählen, Vorschau lesen, starten, abbrechen, Fehler beheben und Einstellungen ändern. Logische Fokusreihenfolge, sichtbarer Fokus, Escape für abbrechbare Dialoge, Fokus nach Dialogende zurückgeben.
- Accessible Name, Role, Value, Status, Fehlerzuordnung und ausgewählter Zustand für alle Controls; Standardwidgets bevorzugen. Eigene Dateibäume/Tabellen/Progresscontrols mit vollständiger Accessibility-Semantik.
- NVDA unter Windows und Orca/AT-SPI unter Linux mit dem **konkret ausgewählten Backend** testen. Framework-Featureflags allein gelten nicht als Nachweis. Virtuelle Tabellen müssen bedienbare und sinnvoll angesagte Zeilen liefern.
- WCAG 2.2 AA als messbare technische Orientierung: normaler Text mindestens 4,5:1, große Schrift 3:1, wesentliche Controls/Fokusindikatoren 3:1. Desktopprüfung zusätzlich zu plattformspezifischen Anforderungen; keine ungeprüfte Zertifizierungsbehauptung.
- Inhalte bei 200 % Text-/UI-Skalierung benutzbar, lange Pfade/Namen und deutsche Übersetzungen ohne unzugängliches Abschneiden. Vollständigen Pfad kopierbar/zugänglich machen.
- Bewegung reduzieren können/Systempräferenz respektieren; kein Blinken. Fortschrittsansagen drosseln, Phasen-/Fehlerwechsel ansagen, nicht jedes Byte.
- Verbindliche Fehlermeldung neben dem betroffenen Feld, nicht ausschließlich Tooltip/Toast. Icons mit erklärendem Text oder zugänglicher Beschriftung.
- Destruktive Änderungen anhand des konkreten Plans bestätigen; sichere Standardauswahl. Abbruch, Pause und Löschen semantisch unterscheiden.

## Zustandskatalog

Erster Start; keine Repositories; konfiguriert; prüfen; offline; Auth erforderlich; ungültiges Zertifikat/SSH-Schlüsselwechsel; veraltet; synchronisieren; Abbruch angefordert; abgebrochen; fortsetzbar; voll verifiziert; Teilfehler; Disk Full; Schreibschutz; Dateisperre; Pfadkonflikt; unbekanntes Datenformat; Update verfügbar/abgelehnt. Jede Ansicht zeigt verständliche Aktion und gesicherten Datenzustand. „Pause“ nur anbieten, wenn Resume wirklich implementiert ist.

## Nachweis

P1: ein vertikaler Prototyp mit Tabelle/Dialog/Progress; Accessibilitybaum und manuelle Screenreader-/Tastaturprotokolle beider Systeme. P3: echter Syncablauf; P5: alle Kernansichten; P7: Abnahme am installierten Releasekandidaten. Screenshots mit OS, Skalierung, Theme, Build-SHA und Prüfergebnis ablegen. Keine Behauptung „zugänglich“, solange Linux- oder Windowsnachweis fehlt.

Primärquellen: [WCAG 2.2](https://www.w3.org/TR/WCAG22/), [egui-Accessibility](https://github.com/emilk/egui/blob/main/docs/accessibility.md). Die konkrete Toolkitentscheidung bleibt [ADR-0002](../adr/0002-gui-toolkit.md).
