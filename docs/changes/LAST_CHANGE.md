# Letzte integrierte Änderung

**2026-10-18 · MIG-012 · Claude · GUI-Prototyp (egui/eframe mit AccessKit)**

Lokal nach `development` integriert (Merge `67557f3`, nicht gepusht). Neu: Crate `crates/armasync-gui` mit Binary `armasync-gui` (View-Model, Zeichencode, 29 Tests), [Prüfprotokoll](../migration/GUI_PROTOTYPE_EVIDENCE.md) mit Screenshots und Abhängigkeitsliste, Ergebnisabschnitt in [ADR-0002](../adr/0002-gui-toolkit.md). Die Daten im Prototyp sind simuliert; es gibt weiterhin keinen echten Sync. [Bericht](2026-10-18-MIG-012.md), [Handoff](../handoffs/MIG-012.md), [aktueller Stand](../migration/STATUS.md).

Prüfung (Windows 11): fmt, clippy `-D warnings`, 81 Tests, MSRV 1.85 ohne GUI-Crate, Live-Abfrage der laufenden GUI über UI Automation. Linux (WSL2): Build, clippy und 81 Tests im LF-Clone.

**Offen:** NVDA und Orca nicht ausgeführt, Prototyp startet unter WSLg nicht – das P1-GUI-/Accessibility-Gate ist **nicht erfüllt**, ADR-0002 „vorläufig gewählt“. Außerdem: MIG-013 (CI), Rechtefragen RQ-01..RQ-08 (ungeklärt), echte Adapter (P3), macOS/BSD, Hardlinks, FTP/FTPS/SFTP/TLS-Tests.

Vorherige Integrationen am selben Tag: [MIG-011](2026-10-18-MIG-011.md), [MIG-010](2026-10-18-MIG-010.md), [MIG-002](2026-10-18-MIG-002.md), [MIG-003](2026-10-18-MIG-003.md), [MIG-001](2026-10-18-MIG-001.md).
