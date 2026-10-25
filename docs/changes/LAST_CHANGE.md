# Letzte integrierte Änderung

**2026-10-25 · MIG-040B · Claude · WebDAV-Upload mit OPTIONS, PROPFIND, MKCOL, PUT und DELETE über `reqwest`** (nach `development`, Fast-Forward)

* **MIG-040B** ([Bericht](2026-10-25-MIG-040B.md), [Handoff](../handoffs/MIG-040B.md), [ADR-0010](../adr/0010-ftp-sftp-webdav-contract.md)): Die Blockade aus dem ersten Schritt ist aufgelöst. Entschieden wurde Option b: `reqwest` 0.13.5 (MIT OR Apache-2.0, MSRV 1.85.0, blockierend, Features `blocking` und `rustls`). Der Client folgt keinen Redirects selbst; Basic-Zugangsdaten nur über HTTPS und nur an die Herkunft der Basisadresse, bei jeder Anfrage und jedem Redirect geprüft.
* **Lizenz:** Nachtrag in `THIRD_PARTY_NOTICES.md`. 21 neue Pakete im kompilierten Baum, 12 nur im Lockfile (nicht kompiliert). Zwei TLS-Stacks (`ring` für Downloads, `aws-lc-rs` für WebDAV) als Folgeentscheidung.
* **Prüfungen (Windows, lokal):** 17 WebDAV-Loopback-Tests (HTTP und TLS), `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked` und `cargo +1.85.0 test --workspace --exclude armasync-gui --locked` jeweils Exitcode 0. Markdown-Links 175 Dateien, 0 defekt.
* **Nicht geprüft:** echte WebDAV-Server, WSL/Linux (zurückgestellt), GitHub-CI (pausiert); ob ein CMake-Programm für `aws-lc-sys` nötig ist.

Vorheriger Stand:

**2026-10-25 · MIG-040B · Claude · WebDAV-Upload, erster Schritt (OPTIONS, PUT, DELETE); MKCOL/PROPFIND blockiert** (nach `development`, Fast-Forward)

* **MIG-040B** ([Bericht](2026-10-25-MIG-040B.md), [Handoff](../handoffs/MIG-040B.md), [ADR-0010](../adr/0010-ftp-sftp-webdav-contract.md)): `armasync-adapters::webdav` mit `OPTIONS` (WebDAV-Klasse 1), `PUT` (Datei mit bekannter Länge ab Offset 0) und `DELETE`. Basic-Zugangsdaten nur über HTTPS und nur an die Herkunft der Basis-Adresse, bei jeder Weiterleitung neu geprüft; Weiterleitungen auf eine andere Herkunft werden ohne Verbindung zum Ziel abgelehnt. Schreibende Methoden folgen keinen Weiterleitungen.
* **Blockiert:** `MKCOL` und `PROPFIND`. `ureq` 3.4.2 lehnt beide über HTTP/1.1 ab (`MethodVersionMismatch`). Entscheidung nötig: eigener HTTP/1.1-Schreiber über `rustls`, andere HTTP-Bibliothek oder Ordner außerhalb des Clients (siehe ADR-0010, Nachtrag).
* **Prüfungen (Windows, lokal):** 13 WebDAV-Tests (Loopback-HTTP und -TLS), `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked` und `cargo +1.85.0 test --workspace --exclude armasync-gui --locked` jeweils Exitcode 0. Keine neue Abhängigkeit.
* **Nicht geprüft:** echte WebDAV-Server, WSL/Linux (zurückgestellt), GitHub-CI (pausiert).

Vorheriger Stand:

**2026-10-25 · MIG-040A · Claude · Planung FTP/FTPS, SFTP und WebDAV (ADR-0010, angenommen am 2026-10-25)** (nach `development`, Fast-Forward)

* **MIG-040A** ([Bericht](2026-10-25-MIG-040A.md), [Handoff](../handoffs/MIG-040A.md), [ADR-0010](../adr/0010-ftp-sftp-webdav-contract.md)): Planung für P4, keine Protokollimplementierung und keine neue Abhängigkeit. Empfohlene Reihenfolge WebDAV (MIG-040B) → FTP/FTPS (MIG-040C) → SFTP (MIG-040D). Benannt: MSRV-Konflikt (`suppaftp` 1.88, `russh` 1.89 gegenüber 1.85), async-Laufzeitfrage, acht offene Entscheidungen.
* **Abgleich:** Statusangaben in STATUS, Board, Roadmap, Changelog, Blockerliste und den Taskdateien MIG-030E und MIG-032A korrigiert; alle lokalen Markdown-Links geprüft.
* **Nicht geprüft:** Builds der Kandidaten, echte Server, WSL/Linux (zurückgestellt), GitHub-CI (pausiert).

Vorheriger Stand:

**2026-10-25 · MIG-032R2 · Claude · Härtung der Teildateien, zwei Review-Runden** (nach `development`, Fast-Forward)

* **MIG-032R2** ([Bericht](2026-10-25-MIG-032R2.md), [Handoff](../handoffs/MIG-032R2.md), [Task](../tasks/MIG-032R2.md)): Ein vorhandener Teilnamen wird nie mehr zum Schreiben geöffnet. Frischer Start entfernt den Eintrag und legt neu an; Fortsetzen kopiert die gültigen Bytes lesend in eine neue Datei; der Zustand wird über eine Temp-Datei ersetzt. Öffnen verweigert Links; drei Sentinel-Tests belegen, dass externe Dateien unverändert bleiben.
* **Zweite Runde:** Schreiben nur über selbst angelegte Handles, Commit prüft den installierten Inhalt nach dem Rename gegen den Manifest-Hash. Schlägt die Prüfung fehl und lässt sich die unverifizierte Datei nicht entfernen, meldet der Commit einen Dateisystemfehler, der sagt, dass sie am Ziel liegen bleiben kann.
* **Grenze:** Die Linkzahl kann unter Windows auf stabilem Rust 1.85 nicht gelesen werden; RB-01 (Unix-Pfadrennen) bleibt offen. Der Löschfehler-Pfad hat keinen deterministischen Test.
* **Prüfungen (Windows, lokal):** fmt, Clippy `-D warnings`, Workspace-Tests inkl. GUI und Ende-zu-Ende, MSRV 1.85 ohne GUI bestanden.
* **Nicht geprüft:** WSL/Linux (zurückgestellt), GitHub-CI (pausiert).

Vorheriger Stand:

**2026-10-25 · MIG-032R · Claude · Resume mit `Range` und Validatorvertrag für gewöhnliche Dateien** (nach `development`, Fast-Forward)

* **MIG-032R** ([Bericht](2026-10-25-MIG-032R.md), [Handoff](../handoffs/MIG-032R.md), [ADR-0009](../adr/0009-resume-range.md)): Ein unterbrochener Download einer Datei wird nur mit einem **starken** ETag fortgesetzt: `Range` und `If-Range`, Prüfung von 206, `Content-Range`, Gesamtgröße und Validator; 200, 416 und abweichende Antworten führen zu einem Neustart oder einer verweigerten Wiederaufnahme (einmal von Null). Teildaten liegen im Zielordner unter dem reservierten Präfix; ein Netzabbruch behält sie, ein kooperativer Abbruch entfernt sie. Vor dem Übernehmen wird die Gesamtdatei über Größe und Manifest-Hash geprüft, die Zieldatei bleibt bis dahin unverändert. Gepackte Archive sind **nicht** fortsetzbar (bewusst).
* **Commits:** `8bcdcfd` (Reservierung und ADR-0009), `793f43d` (Implementierung und Tests), `06063f7` (Abgleich der Doku).
* **Prüfungen (Windows, lokal, Rust 1.99.1):** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked` (inkl. GUI und Ende-zu-Ende) bestanden; `cargo +1.85.0 test --workspace --exclude armasync-gui --locked` bestanden; CLI-Baum ohne GUI-Crate.
* **Nicht geprüft:** WSL/Linux (zurückgestellt bis zur Abschlussvalidierung), GitHub-CI (pausiert, keine Actions-Läufe), echte Server, Prozess-Kill mit gültigem Zustand.
* **Offen:** Gegenprüfung von ADR-0009, Resume gepackter Archive (Folgetask), Authentifizierung (ausgesetzt, ADR-0005), RB-01..RB-06, RQ-01..RQ-08, P1-GUI-Gate.

Vorheriger Stand:

**2026-10-25 · MIG-032B · Claude · Freiplatz-Vorabkontrolle und Abbruchtests für große und gepackte Downloads** (nach `development`, Fast-Forward)

* **MIG-032B** ([Bericht](2026-10-25-MIG-032B.md), [Handoff](../handoffs/MIG-032B.md)): `LocalStore::free_space` (Standard `None` = unbekannt; Dateisystem über `fs4::available_space`). Vor dem ersten Download simuliert der Lauf den Platzbedarf in Planreihenfolge (Datei plus Archiv bei gepackten Blättern, 16 MiB Reserve, Gutschrift bei ersetzten Dateien) und bricht bei Mangel vor jeder Änderung ab. Nur Vorabkontrolle: anderer Prozess kann Platz verbrauchen. Abbruchtests in der GUI für einen großen normalen und einen gepackten Download (alte Datei bleibt, keine Reste). Testkit: `Published::trickle`.
* **Prüfungen (Windows, lokal):** fmt, clippy `-D warnings`, Workspace-Tests inkl. Ende-zu-Ende und GUI bestanden, MSRV 1.85 ohne GUI bestanden, CLI-Baum ohne GUI-Crates.
* **Nicht geprüft:** WSL/Linux, GitHub-CI (pausiert), echte Server, NVDA/Orca/Sichtprüfung, Konsolen-Tastatur-Ctrl+C für diese Fälle.

Vorheriger Stand:

**2026-10-25 · MIG-030E · Claude · gepackte Blätter (`.zip`) sicher entpacken; GUI-Hinweis angeglichen** (nach `development`, Fast-Forward)

* **MIG-030E** ([Bericht](2026-10-25-MIG-030E.md), [Handoff](../handoffs/MIG-030E.md), [ADR-0008](../adr/0008-packed-files-zip.md)): Blätter mit `compressed` und bekannter Archivgröße werden installiert. Archiv in Spool, genau ein Eintrag mit Stored oder Deflate, Grenzen 2 GiB und 1032:1, Größen- und SHA-1-Prüfung vor dem atomaren Commit. Commits: `4c6be08` (Implementierung), `72f0ecc` (Reservierung), `60372df` (MIG-032A-Nachtrag).
* **GUI-Hinweis:** Komprimierte Blätter mit nutzbarer Archivgröße erscheinen als normale Downloads; nur Blätter ohne nutzbare Größe sind „nicht unterstützt“ (gleicher Code `compressed-file-unsupported` wie im CLI-JSON). Tests für beide Fälle in `armasync-gui`.
* **MIG-032A:** Windows-Ende-zu-Ende-Test der Konsolensignale bestanden (Exitcode 130, unveränderte Zieldatei, keine Temp-Datei, Folgelauf). Keine Tastaturprüfung in einer normalen Konsole behauptet.
* **Prüfungen (Windows, lokal, Rust 1.99.1):** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked` (inkl. GUI) bestanden, `cargo +1.85.0 test --workspace --exclude armasync-gui --locked` bestanden.
* **Nicht geprüft:** echte Serverarchive (Format nicht belegt), WSL/Linux (bis zur Abschlussvalidierung), GitHub-CI (pausiert, keine Actions-Läufe), NVDA/Orca/Sichtprüfung (P1-GUI-Gate offen).

Vorherige Integrationen: [MIG-032A](2026-10-19-MIG-032A.md), [PROC-003](2026-10-19-PROC-003.md), [MIG-030D](2026-10-19-MIG-030D.md), [MIG-030T](2026-10-19-MIG-030T.md), [MIG-030H](2026-10-19-MIG-030H.md), [MIG-030C](2026-10-19-MIG-030C.md), [MIG-030B](2026-10-19-MIG-030B.md), [MIG-030P](2026-10-19-MIG-030P.md), [PROC-002](2026-10-19-PROC-002.md), [MIG-030A](2026-10-19-MIG-030A.md), [MIG-023](2026-10-19-MIG-023.md), [MIG-022](2026-10-19-MIG-022.md), [PROC-001](2026-10-19-PROC-001.md), [MIG-021](2026-10-18-MIG-021.md), [MIG-020](2026-10-18-MIG-020.md), [MIG-012A](2026-10-18-MIG-012A.md), [MIG-013](2026-10-18-MIG-013.md), [MIG-012](2026-10-18-MIG-012.md), [MIG-011](2026-10-18-MIG-011.md), [MIG-010](2026-10-18-MIG-010.md), [MIG-002](2026-10-18-MIG-002.md), [MIG-003](2026-10-18-MIG-003.md), [MIG-001](2026-10-18-MIG-001.md).
