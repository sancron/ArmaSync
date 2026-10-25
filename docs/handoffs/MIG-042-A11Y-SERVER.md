# Handoff MIG-042 A11Y-SERVER — Manueller Upload-Prüfserver

- Status und Eigentümer: integriert (Fast-Forward nach `development`), Claude (Hauptbearbeiter und Integrator). Werkzeug für die manuelle Prüfung, keine Produktfunktion.
- Datum mit Zeitzone: 2026-10-25, Europe/Berlin.
- Basis: `78151c8` (`origin/development`). Branch `claude/mig-042-manual-webdav-server`.
- Geliefert:
  - Modus `start_manual` im Testkit (`crates/armasync-testkit/src/webdav.rs`), Beispiel `upload_webdav`, Tests `manual_webdav` (9).
  - Anleitung [UPLOAD_TEST_SERVER](../migration/UPLOAD_TEST_SERVER.md): Start mit `cargo run -p armasync-testkit --example upload_webdav`, URL `http://127.0.0.1:<port>/dav`, GUI-Schritte, erwartete Ergebnisse, Beenden mit `q`.
  - Grenzen: 64 MiB je Datei (`413`), 256 MiB insgesamt und 200 Dateien (`507`), Pfadprüfung (`400`), Anfrageköpfe bis 64 KiB.
- Geprüft (Windows): Tests 9/9. Prozesslauf mit `curl.exe` (alle Methoden plus Pfad- und Größenfälle) und CLI-Veröffentlichung gegen den laufenden Server (Exitcode 0). Beenden ohne Restprozess und mit geschlossenem Port. Workspace 65 Suiten, MSRV 57 Suiten. fmt und Clippy sauber. Links 0 defekt.
- Nicht geprüft: der GUI-Lauf gegen den Server; die Anzeige der `413`-Antwort in der GUI; Proxy-Variablen.
- Nicht erledigt, bewusst: NVDA-, Narrator- oder Orca-Nutzung, Sicht- und Tastaturprüfung. Das P1-Zugänglichkeitsgate (RB-02) bleibt **geschlossen**, bis die Protokolle tatsächlich ausgefüllt sind.
- Konfliktgefährdete Dateien: `crates/armasync-testkit/src/webdav.rs`, `crates/armasync-testkit/examples/upload_webdav.rs`, `crates/armasync-testkit/tests/manual_webdav.rs`, `docs/migration/UPLOAD_TEST_SERVER.md`, `docs/migration/GUI_UPLOAD_SCREENREADER_PROTOCOL.md`.
- Nächster konkreter Schritt: Ein Mensch mit Windows startet den Server nach der Anleitung, führt U8 bis U24 im Fenster aus, trägt Ergebnisse, Beobachtungen und Build-SHA in das Protokoll ein und legt Screenshots ab. Danach Sichtprüfung (S1 bis S6) und NVDA-Lauf (U1 bis U27).
- Report: [2026-10-25-MIG-042-A11Y-SERVER](../changes/2026-10-25-MIG-042-A11Y-SERVER.md)
