# Branding- und Umbenennungsplan

## Verbindliche Identität

| Bereich | Ziel |
| --- | --- |
| Produktname | **ArmaSync** |
| Marke / sichtbarer Publisher | **Lynxnode.it** |
| App-ID / Reverse Domain | `it.lynxnode.armasync` |
| Crates / interne Rust-Module | `armasync-*` / neutrale englische Fachnamen |
| CLI / GUI-Binary | `armasync` / `armasync-gui` |
| GUI-Titel / Desktop-Eintrag | `ArmaSync` mit Lynxnode.it im About-/Publisherbereich |
| Vorgeschlagene neue Paket-ID | `LynxnodeIT.ArmaSync`, erst vor Registryregistrierung verbindlich prüfen |

Die Namespace-Länderpräfixe `fr` und Teampräfixe `soe`, `sof`, `s.o.e` verschwinden aus **neuen eigenen Symbolen, Pfaden und Produktidentität**. Der Bestand verwendet überwiegend `fr.soe`, nicht `sof`. Neue Namen beschreiben Funktionen: `Repository`, `SyncPlan`, `ProfileStore`, `UpdateVerifier`; keine neuen Länder-/Teampräfixe an Klassen hängen.

## Zu ersetzende Flächen

- README und neue Produktdokumentation; GUI-Titel, About, Hilfe, Spenden-/Communitylinks, Logs und Fehlermeldungen.
- Cargo-Metadaten, Binärname/Dateiinformationen, App-ID, Prozess-/Locknamen, URL-Scheme und Desktop-/Startmenüeinträge.
- Icons, Splash, Installerbilder, mitgelieferte Grafiken, Screenshottexte und Dokumentmetadaten. Neue Logoassets fehlen noch; nichts als freigegeben behaupten.
- Installer/Uninstaller, Registry, Paketnamen, WinGet-Metadaten, Linux-Desktop-Datei, MIME-Handler, Versionsgeneratoren, Releaseartefakte und CI.
- Konfigurations-/Cache-/Logroots, Umgebungsvariablen, User-Agent, Telemetrie-/Supporthinweise falls später eingeführt.
- Update-/Download-/Support-URLs: nur bestätigte Lynxnode.it-Infrastruktur bzw. tatsächliches Projekt-Repository. Keine Endpunkte erfinden, keine automatischen Fallbacks zu früheren Betreibern.

## Was nicht pauschal ersetzt wird

Rechtlich notwendige Hinweise in Lizenzen und Quellheadern bleiben. Historische Analyse/Änderungsnachweise und `legacy` dürfen Herkunft erklären. Altformatbytes und Importpfade ausschließlich in begründeten Kompatibilitätstabellen/Tests. Arma 3, DLC-Namen, Steam-IDs, Dateiendungen, gültige Benutzer-URLs, französische Übersetzungen/Locales und fremde Autoren-/Bibliotheksnamen sind keine neue Publisheridentität.

Kein globales Suchen/Ersetzen von `fr`, `sof`, `soe`, `a3s` oder `Arma3`: dies träfe etwa französische Sprachcodes, `Microsoft`, Serializerdeskriptoren und sachliche Spielnamen. Analysebelege dürfen Altpfade nennen. Das neue Programm darf eigene Klassen/Module niemals nach diesen Altpräfixen benennen.

## Gestufte Umsetzung

P0/P1 neue Dokumente und Rust-Namen korrekt anlegen; Java-Quellen als Baseline unverändert lassen. P2 nur ausdrücklich ausgewählte Altdatenpfade importieren. P5/P6 alle UI-/Paketoberflächen umstellen und neue Assets freigeben. P7 Java-/Altpackagingreste aus dem aktiven Releasebaum entfernen; Baseline und rechtliche Dokumente erhalten.

## Messbare Abnahme

In P6 einen Scanner für Text, Pfade, SVG-Metadaten, ausführbare Dateimetadaten und Paketinhalt bauen. Ausgangsmuster etwa `fr[./\\]soe`, Teamnamevarianten, alte Publisher-/Updatehosts und `Arma3Sync` mit Wort-/Pfadgrenzen. `sof` nur als bekanntes Token prüfen, nie als beliebigen Teilstring.

Ausnahmen einzeln mit Pfad, exaktem Ausdruck, Grund, Verantwortlichem und Test referenzieren. Keine pauschale Allowlist ganzer Quellbäume. Der Scan trennt historischen Quellnachweis vom tatsächlich ausgelieferten Produkt; Binärstrings und Screenshots gehören zur Releaseprüfung. Rechtliche Notices sind erhaltene Herkunft, kein Werbeauftritt.

Releasepakete über explizite Positivliste packen: nur neue ausführbare Artefakte, freigegebene Assets, Lizenz-/Quellhinweise und passende Benutzeranleitung. Keine alten JARs, alten Updateradressen oder Go-Java-Bootstrapper versehentlich mitnehmen.
