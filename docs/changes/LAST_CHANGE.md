# Letzte integrierte Änderung

**2026-10-19 · MIG-032A · Claude · Ctrl+C-Abbruch, Fehlerfälle des Zielordners, Screenreader-Protokoll** (nach PROC-003)

Nach `development` integriert (Merge `56a40f3`). **GitHub-CI ist pausiert** ([CI](../migration/CI.md)): keine CI-Aussage; Freigabegrundlage sind die lokalen Windows-Prüfungen (letzte grüne CI: `15b7a32`).

* **PROC-003:** `sync run --allow-unreleased-platform` existiert nur noch in Validierungsbuilds (Feature `unreleased-platform-override`); in normalen und Release-Builds ist die Linux-Sperre nicht umgehbar (vorher war die Option in jedem Build vorhanden – ein Nutzer der Release-Binärdatei konnte RB-01 damit umgehen). [Bericht](2026-10-19-PROC-003.md).
* **MIG-032A:** Ctrl+C/Ctrl+Break der CLI (erster Interrupt: sauberer Abbruch, Exitcode 130; zweiter: sofortiges Ende), vier weitere Fehlerfall-Tests, erweitertes GUI-Screenreader-Protokoll. **Die Signalzustellung ließ sich in der Entwicklungsumgebung nicht automatisch belegen** (Konsole stellt Interrupts nicht an Kindprozesse zu, auch nicht für `ping`); manueller Test dokumentiert. [Bericht](2026-10-19-MIG-032A.md), [Handoff](../handoffs/MIG-032A.md).
* **Nicht ausgeführt:** NVDA, Orca, Sichtprüfung (**P1-GUI-Gate offen**); keine WSL-/Linux-Läufe (**RB-01** Unix-Pfadrace und **RB-03** Abschlussvalidierung offen, keine Linux-Sicherheitsaussage).

Prüfung (Windows 11, Rust 1.99.1, lokal): fmt, clippy `-D warnings` (auch mit dem Validierungs-Feature), 321 Tests, MSRV 1.85 272 Tests, GUI-freier CLI-Baum.

**Offen:** P1-Gate, RB-01..RB-06, RQ-01..RQ-08, Resume/Authentifizierung/Freiplatzprüfung (Rest MIG-032), ZIP (MIG-030E), Repository hinzufügen in der GUI, Wiederaktivierung der GitHub-CI.

Vorherige Integrationen: [PROC-003](2026-10-19-PROC-003.md), [MIG-030D](2026-10-19-MIG-030D.md), [MIG-030T](2026-10-19-MIG-030T.md), [MIG-030H](2026-10-19-MIG-030H.md), [MIG-030C](2026-10-19-MIG-030C.md), [MIG-030B](2026-10-19-MIG-030B.md), [MIG-030P](2026-10-19-MIG-030P.md), [PROC-002](2026-10-19-PROC-002.md), [MIG-030A](2026-10-19-MIG-030A.md), [MIG-023](2026-10-19-MIG-023.md), [MIG-030-Vorprüfung](2026-10-19-MIG-030-preliminary.md), [MIG-022](2026-10-19-MIG-022.md), [PROC-001](2026-10-19-PROC-001.md), [MIG-021](2026-10-18-MIG-021.md), [MIG-020](2026-10-18-MIG-020.md), [MIG-012A](2026-10-18-MIG-012A.md), [MIG-013](2026-10-18-MIG-013.md), [MIG-012](2026-10-18-MIG-012.md), [MIG-011](2026-10-18-MIG-011.md), [MIG-010](2026-10-18-MIG-010.md), [MIG-002](2026-10-18-MIG-002.md), [MIG-003](2026-10-18-MIG-003.md), [MIG-001](2026-10-18-MIG-001.md).
