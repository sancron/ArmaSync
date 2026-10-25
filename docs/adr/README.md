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

Jede neue Entscheidung nennt Datum, Kontext, Optionen, Entscheidung/Status, Folgen, Belege und betroffene Tasks. Bestehende Entscheidung bei Richtungswechsel nicht kommentarlos umschreiben: neue ADR erstellen und alte als ersetzt verlinken.
