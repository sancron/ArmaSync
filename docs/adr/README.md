# Architekturentscheidungen

Status: `vorgeschlagen`, `angenommen`, `ersetzt`, `verworfen`. Nur angenommene Entscheidungen sind für ihre konkrete Implementierungswahl verbindlich; Produktanforderungen bleiben unabhängig davon verbindlich.

| ADR | Thema | Status |
| --- | --- | --- |
| [0001](0001-rust-workspace.md) | Rust-Workspace und Layergrenzen | angenommen für Modulgrenzen; Bibliotheksdetails offen |
| [0002](0002-gui-toolkit.md) | Native GUI-Technologie und Accessibility-Gate | vorgeschlagen |
| [0003](0003-legacy-formats.md) | Altformatimport und schrittweiser Cutover | angenommen für sicheren Import; Schreibformat offen |
| [0004](0004-core-contracts.md) | Kernverträge: Pfade, Manifest, Plan, Fehler, Events, Jobs, Ports | angenommen für die in MIG-011 implementierten Typen |

Jede neue Entscheidung nennt Datum, Kontext, Optionen, Entscheidung/Status, Folgen, Belege und betroffene Tasks. Bestehende Entscheidung bei Richtungswechsel nicht kommentarlos umschreiben: neue ADR erstellen und alte als ersetzt verlinken.
