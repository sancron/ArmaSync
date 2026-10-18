# Lizenzstrategie und Geltungsbereich

Stand: 2026-10-18. Ziel neuer, eigenständig erstellter Rust-Komponenten: **GNU Affero General Public License, Version 3**, SPDX **`AGPL-3.0-only`**. „Only“ ist die konservative Auslegung des gewünschten AGPLv3-Ziels; „or later“ erfordert eine ausdrückliche Projektentscheidung.

## Aktueller Bestand

`LICENCE.md` enthält GPLv3 und bleibt unverändert. Der Java-Einstieg nennt GPL Version 3; weitere Dateien haben eigene Hinweise. `jazsync`, `jazsyncmake`, `MD4` und `BrokenMD4` enthalten GPL-2.0-or-later-Hinweise und weitere Urheber. Dies ist noch keine vollständige Rechteklärung aller Quellen und Binärbibliotheken.

Eine Übersetzung nach Rust ist nicht automatisch eine unabhängige Neuschöpfung. Übertragene Implementierungsdetails, Quellen, Assets und Kommentare samt Herkunft bewerten. Produkt- oder Klassenumbenennung schafft keine zusätzlichen Lizenzrechte.

## Umsetzung

1. Neue unabhängige Rust-Dateien: `SPDX-License-Identifier: AGPL-3.0-only`; Cargo-Pakete deklarieren dies erst, wenn ihr Inhalt dazu passt.
2. Vorhandene GPL-Quellen nicht pauschal als AGPL auszeichnen. Übernahmen mit Originalpfad, Commit, Lizenz, Urheber, Zielpfad und Rechtebasis erfassen.
3. GPLv3 §13 und AGPLv3 §13 erlauben bestimmte Kombinationen. Sie erlauben keine pauschale Ersetzung der GPL-Dateilizenzen. Weitergeltende Lizenzen pro Bestandteil dokumentieren.
4. Für ausschließlich AGPLv3-lizenzierten eigenen Code unabhängig entwickelte Komponenten verwenden oder erforderliche Rechte/Zustimmung belegen. Ungelöste Übernahmen separieren; unbetroffene Arbeit fortsetzen.
5. Branding-Assets unabhängig erstellen; bei Wiederverwendung von OFL-Schriften und anderen Assets Originalhinweise beilegen. Cargo-, System- und Installerabhängigkeiten prüfen.
6. Vor Release maßgebliche Lizenztexte, Third-Party-Notices, Quellarchiv des exakten Tags und Build-/Installationsquellen bereitstellen. Gegebenenfalls AGPL-Quellangebot für Netzwerkinteraktion umsetzen. UI: „Lizenzen und Herkunft“; CLI: `armasync license`.

Neue Identität und Produktnavigation enthalten keine alten Teamnamen. Rechtlich notwendige Copyright-/Herkunftsangaben bleiben in Lizenzunterlagen und zugehörigen Quellhinweisen erhalten. Alte Namen aus dem gesamten Git-Verlauf zu entfernen widerspräche auch dem unveränderten `legacy`-Branch.

## Release-Gate

Nicht behaupten, der heutige Gesamtbestand sei bereits AGPL. P0 erfasst Herkunft, P1 setzt die Lizenz für neue Crates um, P7 prüft das gesamte Release. `LICENSES/AGPL-3.0-only.txt` ist der unveränderte Referenztext für neue Komponenten; er ersetzt `LICENCE.md` nicht.

Primärquellen, geprüft am 2026-10-18: [GPLv3, §§4–5 und 13](https://www.gnu.org/licenses/gpl-3.0.html), [AGPLv3, §13](https://www.gnu.org/licenses/agpl-3.0.html), [GNU zur Anwendung von Lizenzen](https://www.gnu.org/licenses/gpl-howto.en.html). Konkrete Rechtebelege bleiben Teil der Release-Abnahme.
