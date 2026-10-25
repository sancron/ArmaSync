# Handoff MIG-040A

- Status und Eigentümer: integriert (Fast-Forward nach `development`), Claude (Integrator). Planung abgeschlossen, keine Implementierung.
- Datum mit Zeitzone: 2026-10-25, Europe/Berlin.
- Basis- und letzter Codecommit: Basis `145f121`; kein Codecommit, nur Dokumentation.
- Branch / eigener Worktree: `claude/mig-040a-transport-plan` im Hauptcheckout.
- Fertiges Verhalten und Änderungen: ADR-0010 (vorgeschlagen) mit Priorisierung (WebDAV, dann FTP/FTPS, dann SFTP), gemeinsamem Vertrag, Auth-Tabelle, TLS-/Host-Key-Regeln, Fehlerliste, Testserver-Vorschlag und acht offenen Entscheidungen.
- Getestet und Ergebnis (Windows 11, PowerShell 7, 2026-10-25): Markdown-Linkprüfung über 172 Markdown-Dateien und 591 lokale Links: 0 defekt. `git diff --check`: fehlerfrei (nur CRLF-Hinweise von git für die Arbeitskopie). Kein Build, keine Protokolltests.
- Nicht getestet / Blocker:
  - Kandidaten-Bibliotheken nicht gebaut; MSRV nicht durch `cargo +1.85.0` belegt.
  - Blockiert: `suppaftp` (MSRV 1.88) und `russh` (MSRV 1.89) über dem Workspace-MSRV 1.85; `reqwest_dav` async.
  - Offene Entscheidungen 1 bis 8 in ADR-0010.
- Abgestimmte Verträge / neue ADRs: ADR-0010 (vorgeschlagen, nicht angenommen).
- Konfliktgefährdete Dateien / reservierter Schreibbereich: keine gemeinsamen Dateien außer den Integratordateien, die der Integrator bei Übernahme ändert.
- Nächster konkreter Schritt:
  1. Projektverantwortlicher entscheidet die Punkte 1 bis 4 in ADR-0010 (MSRV, SFTP-Bibliothek, Plaintext-FTP, Basic über `http`).
  2. Bei Zustimmung zu Punkt 4 („nein“): MIG-040B (WebDAV) als eigener Task mit `claude/mig-040b-webdav`, Basis `development`.
  3. Vor MIG-040B: MSRV-Probe der XML-Parser-Abhängigkeit mit `cargo +1.85.0` (nur außerhalb des Repositorys), Lizenz in RQ-06 festhalten.
- Report: [2026-10-25-MIG-040A](../changes/2026-10-25-MIG-040A.md)
