# Architekturentscheidungen

Status: `vorgeschlagen`, `angenommen`, `ersetzt`, `verworfen`. Nur angenommene Entscheidungen sind für ihre konkrete Implementierungswahl verbindlich; Produktanforderungen bleiben unabhängig davon verbindlich.

| ADR | Thema | Status |
| --- | --- | --- |
| [0001](0001-rust-workspace.md) | Rust-Workspace und Layergrenzen | angenommen für Modulgrenzen; Bibliotheksdetails offen |
| [0002](0002-gui-toolkit.md) | Native GUI-Technologie und Accessibility-Gate | vorgeschlagen |
| [0003](0003-legacy-formats.md) | Altformatimport und schrittweiser Cutover | angenommen für sicheren Import; Schreibformat offen |
| [0004](0004-core-contracts.md) | Kernverträge: Pfade, Manifest, Plan, Fehler, Events, Jobs, Ports | angenommen für die in MIG-011 implementierten Typen |
| [0005](0005-credentials-and-data-paths.md) | Credentials, Datenverzeichnisse, versionierte Dokumente | angenommen für Format, Pfade und Port (MIG-020); Schlüsselbund-Adapter offen |
| [0007](0007-transfer-ports.md) | Streaming-Übertragung, Staging, atomare Übernahme, Schutzpfade (Ports) | angenommen; umgesetzt in MIG-030P gegen In-Memory-Referenz |
| [0006](0006-sync-metadata-and-http-transport.md) | Sync-Metadaten, SHA-1-Integrität, HTTP(S)-Client (`ureq`), Zerlegung von MIG-030 | angenommen für Wahl und Zerlegung; Port-Signaturen mit MIG-030B/C |
| [0008](0008-packed-files-zip.md) | Gepackte Blätter (`.zip`): Entpacken mit festen Grenzen (`zip` 7.3.0, Ports `spool`/`ArchiveReader`) | angenommen für MIG-030E; Format echter Server-Archive nicht belegt |
| [0009](0009-resume-range.md) | Resume mit `Range`: starker ETag als Validator, Teildaten je Schlüssel, Abbruchregeln (ändert ADR-0007 für Netzfehler) | angenommen für MIG-032R; Verhalten echter Server nicht belegt |
| [0010](0010-ftp-sftp-webdav-contract.md) | FTP/FTPS, SFTP, WebDAV: Priorisierung, Transportvertrag, Auth, TLS-/Host-Key-Regeln (MIG-040A) | angenommen (2026-10-25) für Priorisierung, Vertrag, Auth/TLS und Entscheidung 4; Entscheidungen 1–3 und 5–8 offen; Nachtrag: Option b (`reqwest`) für MKCOL/PROPFIND umgesetzt (MIG-040B) |
| [0011](0011-upload-contract.md) | Upload-Vertrag: Ports `LocalSource`/`RemoteTarget`, Plan und Apply, Abbruch, Sicherheitsregeln; getrennte TLS-Stacks (MIG-042, Schritt 1) | angenommen für Schritt 1; Metadaten, Revisionen, Ausschlüsse, Löschen, Parallelität offen |
| [0013](0013-upload-exclusions.md) | Upload-Ausschlüsse: feste Regeln (Teildateien, interne Daten), validierte Pfad-Ausschlüsse, Vorschau mit Gründen; Benutzerformat blockiert (Produktentscheidung) | angenommen (2026-10-25): Option A, explizite Pfade, Vorschau; Muster und Regeldatei offen |
| [0012](0012-revisions-and-publication.md) | Revisionen und Veröffentlichung: Kennung aus dem Inhalt, Revisions-Manifest, Dateien vor Manifest vor Zeiger, Archivierung des alten Zeigers; keine Atomarität behauptet; Legacy-Schreibformat offen | angenommen für Schritt 2 (MIG-042) |

Jede neue Entscheidung nennt Datum, Kontext, Optionen, Entscheidung/Status, Folgen, Belege und betroffene Tasks. Bestehende Entscheidung bei Richtungswechsel nicht kommentarlos umschreiben: neue ADR erstellen und alte als ersetzt verlinken.
