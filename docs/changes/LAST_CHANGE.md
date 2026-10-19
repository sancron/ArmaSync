# Letzte integrierte Änderung

**2026-10-19 · MIG-031 · Claude · Sync-GUI** (zusammen mit MIG-030T, Testflake-Stabilisierung)

Nach `development` integriert (Merge `f53ac24`). **GitHub-CI ist pausiert** ([CI](../migration/CI.md)): keine CI-Aussage; Freigabegrundlage sind die lokalen Windows-Prüfungen (letzte grüne CI: `15b7a32`).

* **Neu:** Ansicht „Synchronisation“ der GUI (Repository, Zielordner, Plan, bestätigungspflichtige Löschungen, Fortschritt, Abbruch, Ergebnis); gemeinsame Crate `armasync-workflow` (CLI und GUI nutzen dieselbe Verdrahtung; geschützte und nicht unterstützte Pfade immer im Plan); Plattformstatus im UI und in der CLI (`sync run` nur auf freigegebener Plattform, sonst Exitcode 8 bzw. `--allow-unreleased-platform`); Test-Crate `armasync-testkit`. [Bericht](2026-10-19-MIG-031.md), [Handoff](../handoffs/MIG-031.md), [GUI_SYNC_EVIDENCE](../migration/GUI_SYNC_EVIDENCE.md), [RELEASE_BLOCKERS](../migration/RELEASE_BLOCKERS.md).
* **MIG-030T:** die einmalige MSRV-Panne ließ sich unter Last in den CLI-Ende-zu-Ende-Tests reproduzieren (Ordnerkollision durch grobe Uhr, Portwiederverwendung); behoben, 108 parallele Läufe ohne Fehlschlag; der Angreifer-Test synchronisiert jetzt explizit. Ob der eine ursprüngliche Fehlschlag genau diese Ursache hatte, ist nicht bewiesen. [Bericht](2026-10-19-MIG-030T.md).
* **Nicht ausgeführt:** NVDA, Orca, Sichtprüfung – **P1-GUI-Gate bleibt offen**; keine WSL-/Linux-Läufe; Linux-Synchronisation bleibt **Release-blockiert** (Unix-Pfadrace RB-01, Abschlussvalidierung RB-03).

Prüfung (Windows 11, Rust 1.99.1, lokal, am zusammengesetzten Stand): fmt, clippy `-D warnings`, 314 Tests, MSRV 1.85 265 Tests, GUI-freier CLI-Baum, Live-Prüfung über Windows UI Automation (einmalig).

**Offen:** P1-GUI-Gate, RB-01..RB-06, RQ-01..RQ-08, Repository hinzufügen/Anmeldung in der GUI, Authentifizierung, Resume/Ctrl+C (MIG-032), ZIP (MIG-030E), Wiederaktivierung der GitHub-CI.

Vorherige Integrationen: [MIG-030D](2026-10-19-MIG-030D.md), [MIG-030T](2026-10-19-MIG-030T.md), [MIG-030H](2026-10-19-MIG-030H.md), [MIG-030C](2026-10-19-MIG-030C.md), [MIG-030B](2026-10-19-MIG-030B.md), [MIG-030P](2026-10-19-MIG-030P.md), [PROC-002](2026-10-19-PROC-002.md), [MIG-030A](2026-10-19-MIG-030A.md), [MIG-023](2026-10-19-MIG-023.md), [MIG-030-Vorprüfung](2026-10-19-MIG-030-preliminary.md), [MIG-022](2026-10-19-MIG-022.md), [PROC-001](2026-10-19-PROC-001.md), [MIG-021](2026-10-18-MIG-021.md), [MIG-020](2026-10-18-MIG-020.md), [MIG-012A](2026-10-18-MIG-012A.md), [MIG-013](2026-10-18-MIG-013.md), [MIG-012](2026-10-18-MIG-012.md), [MIG-011](2026-10-18-MIG-011.md), [MIG-010](2026-10-18-MIG-010.md), [MIG-002](2026-10-18-MIG-002.md), [MIG-003](2026-10-18-MIG-003.md), [MIG-001](2026-10-18-MIG-001.md).
