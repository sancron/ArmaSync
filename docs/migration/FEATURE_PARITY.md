# Funktions- und Portierungsregister

Stand: P0 vorbereitet; **alle Rust-Funktionen sind noch offen**. Priorität definiert Reihenfolge, keine stille Streichung. Windows erhält fachliche Parität zum Bestand; Linux zunächst Synchronisation samt Repositoryverwaltung. N/A nur mit dokumentierter Plattformbegründung.

Statuswerte: `offen`, `in Arbeit`, `implementiert`, `verifiziert`, `N/A`. Eine Funktion ist erst mit verlinktem Test-/Abnahmebeleg verifiziert. Der Integrator ergänzt pro Zeile Ziel-Commit, Plattformen und Belege.

| ID | Fähigkeit / Referenz im Java-Bestand | Phase | Ziel und Abnahme | Rust-Status |
| --- | --- | --- | --- | --- |
| F01 | Repository-CRUD, Import/Autoconfig; RepositoryService, ConnectionService | P2 | GUI/CLI nutzen denselben Store; Geheimnisse getrennt | offen |
| F02 | GZIP/Java-Serialisierung, verschlüsselte Konfiguration; A3SFilesAccessor | P0/P2 | Offline-Import plus direkter Metadatenparser, schema- und datenverlustgeprüft | offen (Fixtures/Inventar: MIG-001 integriert, [FORMAT_INVENTORY](FORMAT_INVENTORY.md)) |
| F03 | Lokaler Scan, Modvergleich, Dateicache | P2/P3 | deterministischer SyncPlan, Case-/Unicode-/Symlinkregeln | offen (Vertrag und In-Memory-Plan/Apply: MIG-011 integriert, [ADR-0004](../adr/0004-core-contracts.md); echter Scan/FS-Adapter fehlt) |
| F04 | HTTP(S)-Download, Range/Resume, Proxy | P3 | echter GUI/CLI-Download, TLS, Abbruch/Resume, Server ohne Range | offen (Matrix/Testserver: [TRANSPORT_MATRIX](TRANSPORT_MATRIX.md), MIG-003 integriert) |
| F05 | FTP/FTPS-Download/-Upload | P4 | vorhandene Modi interoperabel; bekannte FTPS-Einschränkung gesondert prüfen | offen (Matrix/Testserver: [TRANSPORT_MATRIX](TRANSPORT_MATRIX.md), MIG-003 integriert) |
| F06 | SFTP-Transfer | P4 | Passwort sowie geplante Schlüsseloption bewerten, Host-Key-Validierung | offen (Matrix/Testserver: [TRANSPORT_MATRIX](TRANSPORT_MATRIX.md), MIG-003 integriert) |
| F07 | HTTP(S)-WebDAV-Upload | P4 | Serverinteroperabilität, Auth, sichere Pfade | offen (Matrix/Testserver: [TRANSPORT_MATRIX](TRANSPORT_MATRIX.md), MIG-003 integriert) |
| F08 | zsync/Jazsync, MD4/BrokenMD4, SHA-1 | P0/P4 | Golden Vectors, Byte-/Dateiergebnisparität; getesteter Voll-Download-Fallback | offen (zsync-/MD4-Fixtures: MIG-001 integriert; Portierung durch RQ-02 blockiert, siehe [RIGHTS_QUESTIONS](../tasks/RIGHTS_QUESTIONS.md)) |
| F09 | Entpacken und exakter Abgleich | P3/P4 | kein Pfadausbruch; Löschplan explizit, nur verwaltete Dateien | offen |
| F10 | Repository-Build, Revision, Prüfen, versteckte Dateien | P4 | atomare Metadatenveröffentlichung; deterministische Ergebnisse | offen |
| F11 | Uploadparallelität, Session-Reuse, Limits | P4 | unabhängige Check-/Download-/Upload-Limits, konsistenter Abbruch | offen (Matrix/Testserver: [TRANSPORT_MATRIX](TRANSPORT_MATRIX.md), MIG-003 integriert) |
| F12 | Events, Changelogs, Serverinfo | P4 | Laden/Bearbeiten/Publizieren und Roundtrip ohne Feldverlust | offen |
| F13 | Profile, Addon-Gruppen/Reihenfolge, Favoriten | P2/P5 | Import/CRUD; gleiche Argumentreihenfolge und Modauswahl | offen |
| F14 | DLC/CDLC und Workshop-Metadaten | P5 | sachliche Spielnamen/IDs erhalten; Plattformgrenzen prüfen | offen |
| F15 | Launcher-Preset HTML-Import/-Export | P5 | vorhandene JUnit-Fälle als Rust-Fixtures übernehmen | offen |
| F16 | Windows-Spielstart, Parameter, externe Anwendungen | P5 | Argumentvektoren, Pfade mit Leerzeichen, keine Shell-Injektion | offen |
| F17 | Windows-Autostart, Shortcuts, Registry, Single-Instance | P5/P6 | nur passende OS-Capabilities; CLI-Jobs nicht global blockieren | offen |
| F18 | Bikey-Extraktion, TFAR/ACRE-Werkzeuge, RPT-Anzeige | P5 | Toolumfang/Fremdlizenzen getrennt verifizieren; nichts still entfernen | offen |
| F19 | GUI-Navigation, Themes, Diagnose, Einstellungen | P1/P3/P5 | primäre vollständige Oberfläche, Tastatur/NVDA/Orca/Skalierung | offen (Prototyp: MIG-012 integriert, Accessibility-Nachweis NVDA/Orca offen, [Protokoll](GUI_PROTOTYPE_EVIDENCE.md)) |
| F20 | CLI-Befehle einschließlich Build/Check/Sync/Extract/Update | P1–P6 | Subcommands, JSON, Exitcodes; Altargumente dokumentiert abbilden | offen |
| F21 | Updater: GitHub, JSON, XML, SHA-256, Staging | P6 | eigene Channels/Signaturvertrauen, Rollback; keine Altserver-Fallbacks | offen |
| F22 | Windows-/Linux-Pakete, Installation/Deinstallation | P6/P7 | native Pakete, separate CLI, Nutzerdatenerhalt, Lynxnode.it-Metadaten | offen |
| F23 | Portable-/XDG-/AppData-Pfade, Migration | P2/P6 | Kopie/Backup, wiederholbarer Import, fehlende Schreibrechte | offen |
| F24 | Branding, Lizenzanzeige, Quelle, Änderungen | P0–P7 | keine alte Produktidentität in Releaseartefakten; rechtliche Hinweise korrekt | offen |

## Regeln für den finalen Cutover

P3 ist ein Alpha-Vertikalschnitt, keine fertige Migration. P7 verlangt für alle für das jeweilige System zugesagten Funktionen `verifiziert`. Änderungen oder bewusst entfallende Altfeatures benötigen einen benannten ADR und eine explizite Scope-Entscheidung des Projektverantwortlichen. Ein CLI-Erfolg ersetzt keine GUI-Abnahme derselben Kernfunktion.
