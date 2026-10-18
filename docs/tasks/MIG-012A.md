# MIG-012A — GUI-Nachbesserung P1: Kontrast der Auswahlfläche, Screenreader-Prüfprotokoll

- Status: reviewbereit (2026-10-18). **Das P1-GUI-/Accessibility-Gate bleibt offen**, bis NVDA (Windows) und Orca (Linux) belegt sind; dieser Task setzt es nicht auf „bestanden“.
- Eigentümer und Integrator: Claude.
- Basis-Branch und vollständige Basis-SHA: `development`, `6b1f261dd47dcc757a10bdb57ac3e349be7700d0`; der Reservierungscommit steht als Erster auf dem Task-Branch/`development`.
- Eigener Branch und Worktree: `claude/mig-012a-contrast`, Worktree `../ArmaSync-claude-mig-012a`.
- Phase / Voraussetzungen / ADRs: P1-Nachbesserung zu MIG-012; [ADR-0002](../adr/0002-gui-toolkit.md), [UI_REQUIREMENTS](../migration/UI_REQUIREMENTS.md) (inklusive Leitvorgabe).
- Schreibbereich (exklusiv):
  - `crates/armasync-gui/src/ui.rs` (nur `themed_visuals` und direkt abhängige Farben), `crates/armasync-gui/tests/ui_accessibility.rs` (Kontrasttest)
  - `docs/migration/GUI_PROTOTYPE_EVIDENCE.md` (Kontrasttabelle, Verweis auf das Protokoll), `docs/migration/GUI_SCREENREADER_PROTOCOL.md` (neu: manuelles NVDA-/Orca-/Sicht-Prüfprotokoll)
  - `docs/tasks/MIG-012A.md`, `docs/changes/*-MIG-012A.md`, `docs/handoffs/MIG-012A.md`
- Exklusiv reservierte Shared Files: keine (`Cargo.lock` bleibt unverändert).
- Ziele: Auswahlfläche im hellen Theme von 1,56:1 auf mindestens 3:1 gegen die Panelfläche bringen, ohne den Kontrast des Texts auf der Auswahl (4,5:1) und des Fokus-/Auswahlrings (3:1) zu verletzen; Ausnahme im Test und Protokoll entfernen. Konkrete, ausführbare manuelle Prüfschritte für NVDA und Orca (Vorbereitung, Bedienung, erwartete Ansagen, Protokollvorlage) bereitstellen.
- Nicht im Scope: Gate auf „bestanden“ setzen, Toolkitwechsel, neue Funktionen, Layoutänderungen.
- Abnahme: `cargo fmt/clippy/test` (GUI-Crate und Workspace), Kontrasttabelle aus dem Test, Nachweis des hellen Themes soweit ohne Zusatzsoftware möglich, `git diff --check`.
- Resultat / Codecommits / Report / Handoff: Codecommit `554f065`, Dokumentation `0a15ead` auf `claude/mig-012a-contrast`, Merge `9577b58`; [Bericht](../changes/2026-10-18-MIG-012A.md), [Handoff](../handoffs/MIG-012A.md).
- Blocker: NVDA/Orca in dieser Umgebung nicht vorhanden (Lücke bleibt, manueller Schritt für den Projektverantwortlichen).
