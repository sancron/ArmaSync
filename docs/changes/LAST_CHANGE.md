# Letzte integrierte Änderung

**2026-10-19 · MIG-030H · Claude · Hardening des Dateisystemadapters**

Nach `development` integriert (Merge `39f7ebd`). **GitHub-CI ist pausiert** ([CI](../migration/CI.md)): keine CI-Aussage; Freigabegrundlage sind die lokalen Windows-Prüfungen (letzte grüne CI: `15b7a32`).

* **Neu:** `LocalDirectory::open_exclusive` (Betriebssystem-Dateisperre, ein Schreiber je Ziel, endet mit dem Prozess), schreibgeschützter `new`, Aufräumen aller Staging-Reste unter der Sperre, Windows-Ordner-Pinning, kanonische Pfadprüfung auf allen Plattformen. [Bericht](2026-10-19-MIG-030H.md), [Handoff](../handoffs/MIG-030H.md), [Bedrohungsanalyse](../migration/LOCAL_FS_THREATS.md). Neue Abhängigkeit `fs4` 1.1.0.
* **Nicht behoben:** Unix-Race gegen einen gleichzeitig handelnden lokalen Angreifer – **Release-Blocker für „race-freie Pfadgrenze unter Linux“**; Schutzgrenze und Behebungsweg in der Analyse.
* Es wird weiterhin nichts Reales synchronisiert (MIG-030D folgt).

Prüfung (Windows 11, Rust 1.99.1, lokal): fmt, clippy `-D warnings`, 274 Tests, MSRV 1.85 245 Tests, GUI-freier Baum, `git diff --check`. **WSL/Linux lokal: offen.**

**Offen:** P1-GUI-Gate (NVDA/Orca/Sichtprüfung nicht ausgeführt), RQ-01..RQ-08, MIG-030D/E, GitHub-CI-Wiederaktivierung, Linux-Abschlussvalidierung inkl. Unix-Race.

Vorherige Integrationen: [MIG-030C](2026-10-19-MIG-030C.md), [MIG-030B](2026-10-19-MIG-030B.md), [MIG-030P](2026-10-19-MIG-030P.md), [PROC-002](2026-10-19-PROC-002.md), [MIG-030A](2026-10-19-MIG-030A.md), [MIG-023](2026-10-19-MIG-023.md), [MIG-030-Vorprüfung](2026-10-19-MIG-030-preliminary.md), [MIG-022](2026-10-19-MIG-022.md), [PROC-001](2026-10-19-PROC-001.md), [MIG-021](2026-10-18-MIG-021.md), [MIG-020](2026-10-18-MIG-020.md), [MIG-012A](2026-10-18-MIG-012A.md), [MIG-013](2026-10-18-MIG-013.md), [MIG-012](2026-10-18-MIG-012.md), [MIG-011](2026-10-18-MIG-011.md), [MIG-010](2026-10-18-MIG-010.md), [MIG-002](2026-10-18-MIG-002.md), [MIG-003](2026-10-18-MIG-003.md), [MIG-001](2026-10-18-MIG-001.md).
