# MIG-042 A11Y-SERVER — Manueller Upload-Prüfserver für die Screenreader-Prüfung

- Status: **integriert** (2026-10-25, Fast-Forward nach `development`). Eigentümer und Integrator: Claude. Basis `78151c8` (`origin/development`). Branch `claude/mig-042-manual-webdav-server`.
- Ziel: Ein manuell startbares Werkzeug, mit dem die Upload-Schritte U8 bis U24 des [Protokolls](../migration/GUI_UPLOAD_SCREENREADER_PROTOCOL.md) am echten GUI-Fenster gegen einen lokalen WebDAV-Server ausgeführt werden können, ohne Test-Harness und ohne Zugangsdaten.
- Vorgaben des Projektverantwortlichen, eingehalten:
  - Wiederverwendung des WebDAV-Testservers aus `armasync-testkit`. Das Routing wurde nicht kopiert, sondern um einen Modus erweitert (`start_manual`).
  - Werkzeug, keine Produktfunktion: liegt als Beispiel im Testkit (`examples/upload_webdav.rs`); das Testkit ist „nicht ausgeliefert“.
  - Bindet nur an `127.0.0.1` mit dynamischem Port; URL und Bedienung werden angezeigt.
  - Unterstützt `OPTIONS`, `PROPFIND`, `MKCOL`, `PUT`, `GET`, `DELETE`.
  - Synthetische Daten, Speicher nur im Arbeitsspeicher. Pfadtraversal wird verhindert. Dateigröße (64 MiB), Dateianzahl (200) und Gesamtgröße (256 MiB) sind begrenzt.
  - Keine Authentifizierung, keine echten Daten, kein Verkehr nach außen, keine Telemetrie.
  - Klarer Stopp: `q` und Enter; Strg+C beendet sofort. Nach dem Beenden kein Hintergrundprozess, Port geschlossen (am echten Prozess geprüft).
  - Windows-Anleitung: [UPLOAD_TEST_SERVER](../migration/UPLOAD_TEST_SERVER.md).
  - Keine WSL-/Linux-Läufe, keine GitHub-Actions. `testing`, `master`, `legacy` unverändert.
- Ausdrücklich nicht Teil der Aufgabe: NVDA-, Narrator- oder Orca-Nutzung, Sicht- und Tastaturprüfung. Diese Prüfungen erledigt der Server nicht; sie bleiben manuell offen.
- Schreibbereich: `crates/armasync-testkit/src/webdav.rs` (Modus `start_manual`, Grenzen, Pfadprüfung, Abschaltung der Testrouten im Modus), `crates/armasync-testkit/examples/upload_webdav.rs` (neu), `crates/armasync-testkit/tests/manual_webdav.rs` (neu), `docs/migration/UPLOAD_TEST_SERVER.md` (neu), `docs/migration/GUI_UPLOAD_SCREENREADER_PROTOCOL.md` (Vorbereitung), Taskdatei, Bericht, Handoff, Integratordokumente.
- Abnahme: Tests der Grenzen und des Routings (9 Tests, Windows, Loopback). Bestehende WebDAV-Adaptertests (20) bleiben unverändert grün. Prozesslauf des Servers mit `curl.exe`, CLI-Veröffentlichung gegen den laufenden Server, Größengrenze, Beenden und Portprüfung. fmt, Clippy `-D warnings`, Workspace und MSRV 1.85 ohne GUI. Markdown-Links. `git diff --check`.
- Blocker und offene Punkte: Die manuelle Prüfung selbst (NVDA, Sicht, Tastatur am Fenster) ist nicht ausgeführt. Das P1-Zugänglichkeitsgate (RB-02) bleibt **geschlossen**. Nicht geprüft: die Anzeige der 413-Antwort in der GUI, die Proxy-Umgebung und der GUI-Lauf gegen den Server selbst.
