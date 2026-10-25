# Mehrstufiger Upgrade-Plan

Stand: 2026-10-25. Nachtrag: MIG-032B, MIG-032R und MIG-032R2 integriert (Resume nur für gewöhnliche Dateien, gepackte Archive nicht fortsetzbar); MIG-040A-Planung für P4 ([ADR-0010](../adr/0010-ftp-sftp-webdav-contract.md), angenommen) und MIG-040B (WebDAV-Upload mit OPTIONS, PROPFIND, MKCOL, PUT und DELETE über `reqwest`). RB-01, GUI-/Screenreader-Gate, Rechtefragen, Linux-Abschlussvalidierung und pausierte GitHub-CI bleiben offen. Vorbereitung dieses Plans abgeschlossen; **P1: MIG-010 bis MIG-013 und MIG-012A integriert (erste GitHub-CI auf `development` am 2026-10-18 erfolgreich); GUI-/Accessibility-Gate offen (NVDA/Orca fehlen), daher P1-Gate nicht erfüllt. P2: MIG-020, MIG-021 und MIG-022 integriert (MIG-021 ohne Entschlüsselung lokaler Repositorydateien; MIG-022 liest nur Name und Adresse aus der Server-Autoconfig, ohne Netzwerkabruf; MIG-023: CLI `repository list`/`profile list`, damit CLI-Teil des P2-Gates erfüllt, GUI-Anzeige folgt mit MIG-031). **P3 begonnen:** MIG-030 in 030A–E zerlegt ([ADR-0006](../adr/0006-sync-metadata-and-http-transport.md)); 030A–E (+030P, 030H, 030T; 030E: gepackte Blätter mit Archivgröße, [ADR-0008](../adr/0008-packed-files-zip.md)), MIG-031 (Sync-GUI) und MIG-032A (Ctrl+C-Abbruch, Zielordner-Fehlerfälle) integriert: CLI- und GUI-Synchronisation unter Windows gegen einen lokalen Loopback-Server belegt, Linux nur Plan (Release-Blocker RB-01), P3-Gate (Linux, Screenreader) nicht erfüllt; offen bleiben die Entschlüsselungsentscheidung (ADR-0003/RQ-01) sowie Einstellungen und Addon-Listen.** **P0-Gate am 2026-10-18 laut unten stehenden Kriterien mit Auflagen erfüllt** (MIG-001/002/003 lokal integriert; Rechtefragen getrennt geführt, aber ungeklärt: [RIGHTS_QUESTIONS](../tasks/RIGHTS_QUESTIONS.md); vollständiger Linux-Generatorlauf bestanden; MIG-010 integriert). Aufwand nicht in Kalendertage umrechnen, bevor Datenformat- und GUI-Prototypen vermessen sind. Jede Phase liefert ausführbare bzw. überprüfbare Ergebnisse mit einem klaren Gate. Kein automatisches Release am Phasenende.

## P0 — Ausgangsstand, Formate und Rechte absichern

Voraussetzung: sauberer Baseline-Commit und dieses Dokumentationspaket.

- `MIG-001`: reproduzierbare synthetische Golden Fixtures für Konfiguration/Profile, Repository-Metadaten, verschlüsselte Beispiele, zsync und ungültige Eingaben erstellen. Originalklassen und serialVersionUIDs katalogisieren; sichere Exportstrategie spezifizieren.
- `MIG-002`: datei-/assetbezogene Rechteprüfung einschließlich lokaler JARs und Jazsync; Lizenzübernahmen dokumentieren, Abhängigkeiten inventarisieren.
- `MIG-003`: Transport-/Featurematrix aus Code und Tests konkretisieren; minimale Testserver, Plattformen und Datenmengen festlegen. Java-Ergebnisartefakte als Vergleich sichern.

**Gate:** Fixtures mit Herkunft, Hashes, erwarteten fachlichen Ergebnissen und Generatoranleitung; keine personenbezogenen Daten; Rechtefragen als bearbeitbare Tasks getrennt; Baseline/Branchregeln bestätigt. Legacy-Quellnamen bleiben unverändert. Keine voreilige Java-Packageumbenennung.

**Parallel möglich:** MIG-001 Formate; MIG-002 Rechte; MIG-003 Netzwerk/Tests. Jeder Task eigener Eigentümer und Worktree; keine gemeinsamen Quelldateien.

## P1 — Rust-Fundament und GUI-Entscheidung

Voraussetzung: P0-Gate für die jeweils verwendeten Daten und Quellen.

- `MIG-010`: Cargo-Workspace für Core, Application, Adapters und CLI; Rust-Toolchain/MSRV verifizieren und pinnen, SPDX/Cargo-Lizenzen für neue unabhängige Quellen, Format/Lint/Tests, reproduzierbarer CLI-Build unter Windows/Linux.
- `MIG-011`: gemeinsame Modelle, Ports, Fehler, Events und Jobzustände anhand eines kleinen In-Memory-Use-Cases fixieren. Prüffälle zuerst aus fachlichen Beispielen ableiten.
- `MIG-012`: nativen GUI-Prototyp mit Repositoryliste, Auswahl, Dialog, Progress und Fehlerfall auf beiden Systemen bauen; Tastatur und Screenreader prüfen; [ADR-0002](../adr/0002-gui-toolkit.md) entscheiden.
- `MIG-013`: CI für Windows/Linux einrichten, CLI ohne GUI-Dependencies prüfen; Branch-/Statuschecks und Release-Umgebung nach [BRANCHING.md](../BRANCHING.md) einrichten. Bestehende WinGet-Automatik vor neuer Produktveröffentlichung kontrolliert ersetzen.

**Gate:** Core/CLI unabhängig von GUI baubar; ein sichtbarer GUI-Prototyp mit nachgewiesener Accessibility; keine Platzhalterbehauptung fertiger Synchronisation; Toolchain, GUI-Wahl und Plattformminimum dokumentiert.

**Parallel möglich:** Nach MIG-011-Vertragsfreeze getrennte CLI-/GUI-Aufgaben. Workspace-Manifest und Lockdatei haben einen Eigentümer.

## P2 — Datenmodell, Speicher und Import

- `MIG-020`: versionierte neue JSON-Daten/TOML-Einstellungen; Repository-/Profil-Store, IDs, AppData-/XDG-/portable Pfade und sichere Credentials.
- `MIG-021`: offline Export → neutraler Import für lokale Altdateien; verschlüsselte Daten explizit behandeln, keine Secrets exportieren. Kopie/Backup/Validierung/Abbruch/erneuter Import prüfen.
- `MIG-022`: begrenzter parserbasierter Direktimport von Server-Metadaten für bestehende Repositories; kein JVM-Laden von Netzwerkdaten. Transportneutrale Core-Modelle.

**Gate:** Golden-Datensätze werden verlustfrei bzw. mit explizitem Importbericht übernommen; Originaldaten unverändert; unbekannte Version/Typen/zu große Daten werden sicher abgelehnt. Neue eigene Typen tragen ausschließlich neutrale Namen. CLI und GUI können importierte Repositories anzeigen.

## P3 — Erster vollständiger GUI-/CLI-Sync-Ablauf

- `MIG-030`: Scan → Plan → HTTP(S)-Download → Hashprüfung → atomare Installation → Verify. Range/Resume mit Validatoren und begrenzten Retries.
- `MIG-031`: GUI für Import, Zielauswahl, Änderungsprüfung, Start, Fortschritt, Abbruch und Ergebnis (Leitvorgabe aus [UI_REQUIREMENTS](UI_REQUIREMENTS.md): vertraute Abläufe der Baseline, moderner und zugänglicher); CLI für denselben Ablauf einschließlich JSON und nichtinteraktivem Betrieb.
- `MIG-032`: Fehlerfälle: Offline, Auth/TLS, fehlender Platz, Schreibschutz, geänderte Remote-Datei, Prozessabbruch, ungültige Pfade und parallele Zielzugriffe.

**Gate:** Beide Frontends synchronisieren denselben Testbestand mit identischem Dateiinhalt unter Windows/Linux. GUI bleibt bedienbar; Abbruch hinterlässt keine beschädigten Zieldateien. Ergebnisse und verbleibende Lücken sind als Alpha dokumentiert.

## P4 — Transporte, Delta und Administration

- `MIG-040`: FTP/FTPS, SFTP und WebDAV mit Capability-Matrix, Proxyverhalten, TLS-/Host-Key-Prüfung; Timeout-/Fehlerszenarien an lokalen Testservern. Stand: MIG-040A ([ADR-0010](../adr/0010-ftp-sftp-webdav-contract.md), angenommen); MIG-040B liefert WebDAV-Upload mit Ordner anlegen und Existenzprüfung (`reqwest`). Reihenfolge WebDAV → FTP/FTPS → SFTP; Umsetzung nach Entscheidungen.
- `MIG-041`: Jazsync/zsync-Parität mit Golden Vectors einschließlich MD4-Varianten; zunächst Voll-Download-Fallback, dann Delta/Resume freigeben.
- `MIG-042`: Build/Upload, Revisionen, Events/Changelogs, Dateiausschlüsse, Metadaten zuletzt veröffentlichen; separate Parallelitätslimits, Wiederaufnahme und Abbruch. Stand: Schritt 1 (Upload-Vertrag [ADR-0011](../adr/0011-upload-contract.md), Plan und Apply mit WebDAV-Ziel und lokaler Quelle, Abbruch, keine Löschungen) umgesetzt; Schritt 2 (Revisionen und Veröffentlichung, [ADR-0012](../adr/0012-revisions-and-publication.md)) integriert; Schritt 3a (Rückleseprüfung vor der Veröffentlichung) integriert; Schritt 3b (Upload-Ausschlüsse, [ADR-0013](../adr/0013-upload-exclusions.md)) integriert, Benutzerformat: Option A entschieden, Muster und Regeldatei sind spätere Entscheidungen; Schritt 4a (CLI, [ADR-0014](../adr/0014-upload-cli.md)) integriert, GUI (4b) und Fortschritt pro Datei (4c) offen: Kennung aus dem Inhalt, Dateien vor Manifest vor Zeiger, alter Zeiger in `history/`, keine Atomarität behauptet. Legacy-Metadatenformat (ADR-0003), Ausschlüsse, Parallelität, Wiederaufnahme, Verifikation und CLI/GUI-Verdrahtung sind Folgeschritte.
- `MIG-043`: Entpacken/exakter Abgleich, verwaltete Dateimengen, Vorschau, Symlink-/Junction-/Case-/Unicodekonflikte; UI-Administration zugänglich integrieren.

**Gate:** relevante Transport-/Serverkombinationen und Repository-Roundtrips belegt; sicherer Sync/Upload ohne Pfadausbruch und unbeabsichtigtes Löschen; Formatentscheidung gemäß ADR-0003. Keine Delta-Parität allein aus Cratenamen ableiten.

## P5 — Profile, Windows-Launcher und restliche GUI

- `MIG-050`: Profile/Gruppen/Prioritäten, Favoriten, DLC/CDLC, Workshop-Metadaten und Launcher-Presets; Argumentvergleich mit Baseline.
- `MIG-051`: Windows-Spielstart, externe Anwendungen, Shortcuts, Autostart, Single-Instance; kein Shell-Stringaufbau aus Nutzereingaben.
- `MIG-052`: Bikey/TFAR/ACRE/RPT-Funktionen gemäß Paritätsmatrix prüfen und portieren; Paket-/Toollizenzen beachten.
- `MIG-053`: (Leitvorgabe [UI_REQUIREMENTS](UI_REQUIREMENTS.md)) vollständige GUI-Einstellungen, Diagnose, Hilfe, Lizenzen, Deutsch/Englisch, Light/Dark/System, Tastatur-/Screenreader-Abnahme.

**Gate:** Windows-Funktionsparität nachgewiesen. Linux-Synchronisation vollständig; Windows-/Spielstartfunktionen mit klarer Capability-Begründung N/A. Kein Protonziel ohne neue Scope-Entscheidung.

## P6 — Native Auslieferung, Branding und Updates

- `MIG-060`: Windows-GUI-Installer/portable Paket und separate CLI; Linux-Pakete für in P1 festgelegte Systeme. Nutzerdaten bei Installation/Deinstallation erhalten.
- `MIG-061`: neue Lynxnode.it-Assets, Binärmetadaten, Desktop-ID, Publisher, Paket-IDs, Links und Datenschutz-/Lizenzansicht. Neue Updateadressen und Signaturschlüssel nur aus bestätigter Infrastruktur.
- `MIG-062`: gesonderter Rust-Updater, Signaturprüfung, Staging, Windows-Dateisperren, Rollback, stable/testing-Kanäle; Systempaketverwaltung respektieren. Ohne bestätigte Infrastruktur manuelle signierte Releases, kein alter Fallbackserver.
- `MIG-063`: SBOM/Notices, Source-Archive, Versionsanzeige, Hashes/Signaturen und plattformbezogene Installationsprüfungen.

**Gate:** installierbare native Produkte ohne Java-/Go-Java-Bootstrapper; Brandingprüfung erfolgreich; neue Paketidentität kollidiert nicht ungeprüft mit alten Installationen. Update-Sicherheitsmodell und Wiederherstellung belegt oder Self-Update ausdrücklich deaktiviert.

## P7 — testing, Freigabe und Legacy-Cutover

- Release-Kandidaten aus `development` nach `testing`, exakte SHA und Artefakte festhalten; reproduzierbare Build-/Test-/Lizenz-/Branding-/GUI-Abnahme gemäß [VALIDATION.md](VALIDATION.md).
- Alle zugesagten Funktionen verifiziert; offene Ausnahmen explizit entscheiden. Kein Finalrelease mit nur einer CLI und unfertiger GUI.
- Erst danach Java-Quellen, JARs, alte Launcher-/Update-/Packagingpfade aus dem **aktiven** Baum entfernen. Herkunftspflichtige Texte erhalten; `legacy` und Historie nicht umschreiben.
- Nach Cutover erneut vollständige Abnahme in `testing`. Freigegebenen RC nach `master` übernehmen, überprüften Release-Tag und Quellen/Artefakte veröffentlichen. Branchschutz und Freigaberechte beachten.

**Gate:** `master` enthält nur den geprüften Releasecode; `legacy` unverändert; dokumentiertes Upgrade und Rollback; primäre GUI sowie CLI auf Windows/Linux funktionstüchtig. Änderungen seit dem vorigen Release und zuletzt integrierte Änderung sind eindeutig auffindbar.

## Definition of Done pro Task

Eigentümer und Scope dokumentiert; implementiertes Ergebnis reviewbar; angemessene Tests/Prüfungen mit Ergebnis; Kompatibilität, Lizenz und Branding berücksichtigt; eigener Änderungsbericht und Handoff; keine fremden Änderungen; Integrator übernimmt relevante Status-/Changelog-/Paritätsupdates. Nicht verfügbare Prüfungen gelten als offen und verhindern das zugehörige Gate.
