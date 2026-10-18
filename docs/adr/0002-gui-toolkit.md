# ADR-0002 — Native GUI mit nachgewiesener Accessibility

Datum: 2026-10-18. Status: **vorläufig gewählt (egui/eframe mit AccessKit)**, Annahme erst nach Screenreader-Nachweis (NVDA und Orca); P1-Prototyp MIG-012 vorhanden, Gate offen.

## Kontext

Primäre GUI, Rust-Portierung, Windows/Linux und Zugänglichkeit sind vorgegeben. Ein Toolkit kann plattformübergreifend rendern, ohne alle benötigten Controls gut an Screenreader zu vermitteln. Die Auswahl darf deshalb nicht nur anhand eines hübschen Screenshots erfolgen.

## Empfehlung und Alternativen

Erste Evaluierung: **egui/eframe mit AccessKit**, da Rust-Integration und Workflow-/Progressoberflächen ohne Web-Frontend möglich sind. Aktuelle Projektdokumentation beschreibt Accessibilityinformationen für Standardcontrols und integrierte Tests; das ist keine Abnahme unserer eigenen Tabellen/Bäume.

Alternative: **Slint**, mit deklarativer UI und Rust-Anbindung; Backendzugänglichkeit und Lizenzmodell der konkret verwendeten Version vor Auswahl prüfen. **Tauri 2** ist eine mögliche Alternative bei nachgewiesenem Vorteil, bringt aber HTML/CSS/JS bzw. TypeScript sowie WebView2/WebKitGTK mit und wäre keine ausschließlich native Rust-UI. **Iced** kann bei konkretem Nachweis ebenfalls geprüft werden; nicht pauschal eine Accessibility-Reife behaupten.

Keine Mischung mehrerer GUI-Toolkits für das Produkt. Keine Festlegung einer aktuellen „neuesten“ Versionsnummer ohne Prüfung zum Implementierungszeitpunkt.

## Entscheidender Prototyp

Repositoryliste mit 10.000 synthetischen Zeilen und langen Namen, sortier-/filterbarer Auswahl, Input-/Authdialog, Fehlerzusammenfassung, Progress/Abbruch, Theme-/Skalierungswechsel. Unter Windows und Linux: Tastaturfolge, NVDA/Orca, Fokuswiederherstellung, Accessibilitybaum, 200 % Skalierung und UI-Reaktionsfähigkeit protokollieren.

Gate: Alle Kernaktionen erreichbar und verständlich angesagt; keine blockierende I/O; Lizenzen/Systembedarf dokumentiert; CLI bleibt unabhängig. Bei Scheitern zuerst Ursache/Backend prüfen, dann Alternative testen und Entscheidung dokumentieren. Keine reine Renderlösung freigeben, die das Gate nicht erfüllt. Auswahlstatus danach auf angenommen setzen und konkrete Version/Backend/Belege hinzufügen.

Primärquellen, recherchiert 2026-10-18: [egui Accessibility](https://github.com/emilk/egui/blob/main/docs/accessibility.md), [eframe](https://docs.rs/eframe/latest/eframe/), [Slint](https://slint.dev/), [Tauri-Systemabhängigkeiten](https://v2.tauri.app/start/prerequisites/). Bibliothekslizenzen und aktuelle Fähigkeiten in P1 erneut prüfen.

## Ergebnis des P1-Prototyps (MIG-012, 2026-10-18)

* **Vorläufige Wahl:** egui/eframe 0.37 mit AccessKit (`crates/armasync-gui`). Der Prototyp erfüllt die technischen Teile des Gates: nativ unter Windows 11 gestartet; Linux baut und besteht alle Tests; Repositoryliste mit 10.000 Zeilen virtualisiert (ca. 3 ms/Frame headless), Dialog mit Feldfehlern, Fortschritt/Abbruch, Fehlerzustand, Theme und Skalierung; keine blockierende I/O im UI-Thread; CLI bleibt GUI-frei. Der Zugänglichkeitsbaum wurde per AccessKit-Harness und live per Windows UI Automation geprüft.
* **Nicht erfüllt:** Screenreader-Nachweis mit NVDA und Orca (nicht installiert; unter WSLg startet der Prototyp zudem nicht). Deshalb ist die Entscheidung nicht auf „angenommen“ gesetzt. Belege, Lücken und manuelle Prüfschritte: [GUI_PROTOTYPE_EVIDENCE](../migration/GUI_PROTOTYPE_EVIDENCE.md).
* **Bekannte Schwächen von egui:** keine Tabellenrollen (Liste als Listbox ausgezeichnet), Standardschrift ohne viele Symbole, Standardfarben mit zu geringem Kontrast (korrigiert über `themed_visuals`), Mindest-Rust 1.95 nur für das GUI-Crate.
* **Alternativen** (Slint, Iced, Tauri) wurden nicht prototypisiert; sie werden nur geprüft, wenn der Screenreader-Nachweis scheitert.
