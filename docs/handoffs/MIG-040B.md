# Handoff MIG-040B

- Status und Eigentümer: integriert, teilweise (Fast-Forward nach `development`), Claude (Integrator). Zwei Operationen blockiert.
- Datum mit Zeitzone: 2026-10-25, Europe/Berlin.
- Basis- und letzter Codecommit: Basis `053746a` (`development`); Codecommit `3b268e1` (ADR-0010 angenommen) plus Implementierungs- und Dokumentationscommits auf `claude/mig-040b-webdav`.
- Branch / eigener Worktree: `claude/mig-040b-webdav` im Hauptcheckout.
- Fertiges Verhalten und Änderungen:
  - `armasync_adapters::webdav::WebDavClient` mit `capabilities()` (`OPTIONS`, WebDAV-Klasse 1), `put(path, &mut File)` (`PUT`, Länge aus der Datei, Start bei Offset 0) und `delete(path)` (`DELETE`).
  - `BasicCredentials` mit redigiertem `Debug`; Basic nur über `https://`, nur an die Herkunft der Basis-Adresse, bei jeder Weiterleitung neu geprüft.
  - Weiterleitungen auf andere Herkunft abgelehnt, ohne Verbindung zum Ziel; schreibende Methoden folgen keinen Weiterleitungen.
  - `http/mod.rs`: interne Sichtbarkeiten und zwei Accessoren; Verhalten unverändert.
  - ADR-0010: Entscheidung 4 angenommen; Nachtrag zur MKCOL-/PROPFIND-Blockade.
- Getestet und Ergebnis (Windows 11):
  - `cargo test -p armasync-adapters --test webdav --locked`: 13/13.
  - `cargo fmt --all -- --check`: bestanden. `cargo clippy --workspace --all-targets --locked -- -D warnings`: Exitcode 0.
  - `cargo test --workspace --locked`: Exitcode 0. `cargo +1.85.0 test --workspace --exclude armasync-gui --locked`: Exitcode 0.
  - Markdown-Linkprüfung (175 Dateien, 601 lokale Links): 0 defekt. `git diff --check`: fehlerfrei.
- Nicht getestet / Blocker:
  - **Blockiert:** `MKCOL` (Ordner anlegen) und `PROPFIND` (Existenz) — `ureq` 3.4.2 / `ureq-proto` 0.6.5 lehnt beide über HTTP/1.1 ab (`MethodVersionMismatch`). Entscheidung nötig: a) eigener HTTP/1.1-Schreiber über `rustls`, b) HTTP-Bibliothek mit beliebigen Methoden (RQ-06), c) Ordner außerhalb des Clients (Stand).
  - Keine echten WebDAV-Server; nur Loopback.
  - Kein Abbruch- und Timeout-Test für den Upload.
- Abgestimmte Verträge / neue ADRs: ADR-0010 angenommen (Entscheidung 4, Umsetzungsregeln, Nachtrag). Keine neue Abhängigkeit.
- Konfliktgefährdete Dateien / reservierter Schreibbereich: `crates/armasync-adapters/src/http/mod.rs` (nur interne Sichtbarkeiten, keine Verhaltensänderung); `src/lib.rs` (eine Zeile). Keine Core-, CLI- oder GUI-Dateien.
- Nächster konkreter Schritt:
  1. Projektverantwortlicher entscheidet den Weg für `MKCOL`/`PROPFIND` (ADR-0010, Nachtrag a/b/c).
  2. Danach MIG-040C (FTP/FTPS) nur nach Klärung der Entscheidungen 1, 3 und 7 aus ADR-0010 beginnen.
- Report: [2026-10-25-MIG-040B](../changes/2026-10-25-MIG-040B.md)
