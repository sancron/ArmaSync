# Letzte integrierte Änderung

**2026-10-25 · MIG-032B · Claude · Freiplatz-Vorabkontrolle und Abbruchtests für große und gepackte Downloads** (nach `development`, Fast-Forward)

* **MIG-032B** ([Bericht](2026-10-25-MIG-032B.md), [Handoff](../handoffs/MIG-032B.md)): `LocalStore::free_space` (Standard `None` = unbekannt; Dateisystem über `fs4::available_space`). Vor dem ersten Download simuliert der Lauf den Platzbedarf in Planreihenfolge (Datei plus Archiv bei gepackten Blättern, 16 MiB Reserve, Gutschrift bei ersetzten Dateien) und bricht bei Mangel vor jeder Änderung ab. Nur Vorabkontrolle: anderer Prozess kann Platz verbrauchen. Abbruchtests in der GUI für einen großen normalen und einen gepackten Download (alte Datei bleibt, keine Reste). Testkit: `Published::trickle`.
* **Prüfungen (Windows, lokal):** fmt, clippy `-D warnings`, Workspace-Tests inkl. Ende-zu-Ende und GUI bestanden, MSRV 1.85 ohne GUI bestanden, CLI-Baum ohne GUI-Crates.
* **Nicht geprüft:** WSL/Linux, GitHub-CI (pausiert), echte Server, NVDA/Orca/Sichtprüfung, Konsolen-Tastatur-Ctrl+C für diese Fälle.
* **Offen:** MIG-032R (Resume, ADR-Zusatz nötig; danach eigener Task), Authentifizierung (ausgesetzt, ADR-0005), RB-01..RB-06, RQ-01..RQ-08.

Vorheriger Stand:

**2026-10-25 · MIG-030E · Claude · gepackte Blätter (`.zip`) sicher entpacken; GUI-Hinweis angeglichen** (nach `development`, Fast-Forward)

* **MIG-030E** ([Bericht](2026-10-25-MIG-030E.md), [Handoff](../handoffs/MIG-030E.md), [ADR-0008](../adr/0008-packed-files-zip.md)): Blätter mit `compressed` und bekannter Archivgröße werden installiert. Archiv in Spool, genau ein Eintrag mit Stored oder Deflate, Grenzen 2 GiB und 1032:1, Größen- und SHA-1-Prüfung vor dem atomaren Commit. Commits: `4c6be08` (Implementierung), `72f0ecc` (Reservierung), `60372df` (MIG-032A-Nachtrag).
* **GUI-Hinweis:** Komprimierte Blätter mit nutzbarer Archivgröße erscheinen als normale Downloads; nur Blätter ohne nutzbare Größe sind „nicht unterstützt“ (gleicher Code `compressed-file-unsupported` wie im CLI-JSON). Tests für beide Fälle in `armasync-gui`.
* **MIG-032A:** Windows-Ende-zu-Ende-Test der Konsolensignale bestanden (Exitcode 130, unveränderte Zieldatei, keine Temp-Datei, Folgelauf). Keine Tastaturprüfung in einer normalen Konsole behauptet.
* **Prüfungen (Windows, lokal, Rust 1.99.1):** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked` (inkl. GUI) bestanden, `cargo +1.85.0 test --workspace --exclude armasync-gui --locked` bestanden.
* **Nicht geprüft:** echte Serverarchive (Format nicht belegt), WSL/Linux (bis zur Abschlussvalidierung), GitHub-CI (pausiert, keine Actions-Läufe), NVDA/Orca/Sichtprüfung (P1-GUI-Gate offen).
* **Offen:** MIG-032B (Freiplatzprüfung, Abbruchtests), MIG-032R (Resume), Authentifizierung (ausgesetzt), RB-01..RB-06, RQ-01..RQ-08.

Vorherige Integrationen: [MIG-032A](2026-10-19-MIG-032A.md), [PROC-003](2026-10-19-PROC-003.md), [MIG-030D](2026-10-19-MIG-030D.md), [MIG-030T](2026-10-19-MIG-030T.md), [MIG-030H](2026-10-19-MIG-030H.md), [MIG-030C](2026-10-19-MIG-030C.md), [MIG-030B](2026-10-19-MIG-030B.md), [MIG-030P](2026-10-19-MIG-030P.md), [PROC-002](2026-10-19-PROC-002.md), [MIG-030A](2026-10-19-MIG-030A.md), [MIG-023](2026-10-19-MIG-023.md), [MIG-022](2026-10-19-MIG-022.md), [PROC-001](2026-10-19-PROC-001.md), [MIG-021](2026-10-18-MIG-021.md), [MIG-020](2026-10-18-MIG-020.md), [MIG-012A](2026-10-18-MIG-012A.md), [MIG-013](2026-10-18-MIG-013.md), [MIG-012](2026-10-18-MIG-012.md), [MIG-011](2026-10-18-MIG-011.md), [MIG-010](2026-10-18-MIG-010.md), [MIG-002](2026-10-18-MIG-002.md), [MIG-003](2026-10-18-MIG-003.md), [MIG-001](2026-10-18-MIG-001.md).
