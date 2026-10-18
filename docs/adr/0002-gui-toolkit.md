# ADR-0002 — Native GUI mit nachgewiesener Accessibility

Datum: 2026-10-18. Status: **vorgeschlagen**, Auswahl durch P1-Prototyp; keine bereits erfolgte Framework-Freigabe.

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
