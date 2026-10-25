# Handoff MIG-032B

- Status: integriert (Fast-Forward-Push auf `development`); Claude; 2026-10-25; Basis `603909e`. Bericht: [MIG-032B](../changes/2026-10-25-MIG-032B.md).
- Umfang: Freiplatz-Vorabkontrolle (`LocalStore::free_space`, Standard `None`; Dateisystem über `fs4::available_space`; Simulation des Laufs in Planreihenfolge mit 16 MiB Reserve, Archivbedarf bei gepackten Blättern, Gutschrift bei ersetzten Dateien); Testkit `Published::trickle`; Abbruchtests für großen normalen und gepackten Download in der GUI.
- Prüfungen (Windows, lokal): fmt, clippy `-D warnings`, `cargo test --workspace --locked` (inkl. GUI und Ende-zu-Ende) bestanden; MSRV 1.85 ohne GUI bestanden; CLI-Baum ohne GUI-Crates.
- **Nicht geprüft:** WSL/Linux (Abschlussvalidierung), GitHub-CI (pausiert), echte Server, NVDA/Orca/Sichtprüfung, Konsolen-Tastatur-Ctrl+C für diese Fälle (nur über den gemeinsamen CancelToken der GUI getestet).
- **Risiken:** Freiplatzprüfung ist nur vorab; anderer Prozess kann Platz verbrauchen, dann scheitert der Schreibvorgang. 16-MiB-Reserve ist eine Heuristik. Unix-`statvfs` ungeprüft. Unbekannt-Zweig des Adapters nur über den Code und Anwendungstests belegt.
- Offen: MIG-032R (Resume, eigener ADR-Zusatz; Implementierung erst nach diesem Stand, er ist jetzt integriert), Authentifizierung (ausgesetzt, ADR-0005), RB-01..RB-06, RQ-01..RQ-08, P1-GUI-Gate.
- Nächster Schritt: MIG-032R nach ADR-Zusatz und Validatorvertrag ([Vorbereitung](../tasks/MIG-032R.md)).
