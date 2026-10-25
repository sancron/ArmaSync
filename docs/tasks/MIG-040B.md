# MIG-040B — WebDAV-Upload-Client (F07), erster Schritt

- Status: **integriert, teilweise** (2026-10-25, Fast-Forward nach `development`; `MKCOL` und `PROPFIND` blockiert, siehe Ergebnis). Eigentümer und Integrator: Claude. Basis-Branch `development`, Basis-SHA `053746a` (Stand nach MIG-040A). Eigener Branch `claude/mig-040b-webdav` (Hauptcheckout).
- Phase / Voraussetzungen / relevante ADRs: P4; MIG-040A, MIG-030B (HTTP-Adapter), [ADR-0010](../adr/0010-ftp-sftp-webdav-contract.md) (Entscheidung 4 angenommen: Basic nur über HTTPS; Nachtrag zur MKCOL-/PROPFIND-Blockade), ADR-0005 (keine persistenten Credentials), RQ-06.
- Schreibbereich: `crates/armasync-adapters/src/webdav/` (neu), `crates/armasync-adapters/src/http/mod.rs` (nur Sichtbarkeiten und zwei interne Accessoren), `crates/armasync-adapters/src/lib.rs` (Modul eintragen), `crates/armasync-adapters/tests/webdav.rs` (neu), Dokumentation dieses Tasks.
- Integrator-exklusiv (bei Übernahme): `STATUS.md`, `LAST_CHANGE.md`, `docs/tasks/README.md`, `CHANGELOG.md`, `ROADMAP.md`, `docs/adr/README.md`.
- Ziele: Die Operationen, die ein WebDAV-Upload braucht, über den bestehenden HTTP-Stack: Fähigkeiten (`OPTIONS`, WebDAV-Klasse 1), Upload einer Datei mit bekannter Länge (`PUT`), Entfernen (`DELETE`). Zugangsdaten nur über HTTPS und nur an die Herkunft der Basis-Adresse; Weiterleitungen auf eine andere Herkunft werden abgelehnt.
- Nicht im Scope: Ordner anlegen (`MKCOL`) und Existenzprüfung (`PROPFIND`), Auflistung, XML-Auswertung, Sperren, Fortsetzen, Verdrahtung in CLI/GUI/Sync (MIG-042), Schlüsselauthentifizierung, Proxy, Digest/NTLM, neue Abhängigkeiten.
- Ergebnis: Teilweise umgesetzt. **Blockiert:** `MKCOL` und `PROPFIND`. Grund: `ureq` 3.4.2 lehnt diese Methoden über HTTP/1.1 vor dem Senden ab (`MethodVersionMismatch`, belegt im Test). Entscheidung über den Transport offen, siehe ADR-0010 Nachtrag (Optionen a bis c).
- Abnahmekriterien und geplante Checks:
  - Verhalten nach ADR-0010 Entscheidung 4 und den Umsetzungsregeln, durch Tests belegt (13 Tests, Loopback, Windows).
  - Weiterleitung auf anderen Port und Downgrade auf HTTP: Zugangsdaten nie gesendet, Zielserver nie kontaktiert.
  - `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo +1.85.0 test --workspace --exclude armasync-gui --locked`, Markdown-Linkprüfung, `git diff --check` (Windows).
- Resultat / Codecommits / Report / Handoff: siehe [Bericht](../changes/2026-10-25-MIG-040B.md) und [Handoff](../handoffs/MIG-040B.md).
- Blocker und nächste sichere Arbeit: Blockiert sind Ordner anlegen und Existenzprüfung. Entscheidung des Projektverantwortlichen nötig: a) eigener HTTP/1.1-Schreiber über `rustls`, b) HTTP-Bibliothek mit beliebigen Methoden, c) Ordner außerhalb des Clients (Stand). Unabhängig davon sicher weiterführbar: MIG-040C (FTP/FTPS), sobald ADR-0010 Entscheidungen 1, 3, 7 geklärt sind.
