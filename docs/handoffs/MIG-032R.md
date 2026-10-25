# Handoff MIG-032R

- Status: **integriert** (Fast-Forward nach `development`, 2026-10-25); Windows-Prüfungen bestanden. Claude (Projektleitung, Hauptbearbeiter, Integrator); 2026-10-25; Basis `development` @ `1d7049d`. Bericht: [MIG-032R](../changes/2026-10-25-MIG-032R.md). Vertrag: [ADR-0009](../adr/0009-resume-range.md).
- Commits: `8bcdcfd` (Reservierung und ADR), `793f43d` (Implementierung), `06063f7` (Abgleich der Doku).
- Umfang: starker ETag als einziger Validator; HTTP `Range`/`If-Range` mit strenger 206-Prüfung; Teildateien im Zielordner (Abbruch entfernt, Netzfehler behält); einmaliger Neustart bei verweigertem Resume; Gesamtprüfung über Präfix und Hash; Aufräumen unter der Zielsperre. **Gepackte Archive sind nicht fortsetzbar** (bewusst, siehe Bericht).
- Prüfungen (Windows, lokal): fmt, Clippy `-D warnings`, Workspace-Tests inklusive GUI und Ende-zu-Ende, MSRV 1.85 ohne GUI, CLI ohne GUI-Crate: alle bestanden.
- **Nicht geprüft:** WSL/Linux (zurückgestellt bis zur Abschlussvalidierung), GitHub-CI (pausiert), echte Server, Kill eines Prozesses mit gültigem Zustand.
- **Risiken:** kein unabhängiges Vertragsreview; große Archive werden bei jedem Fehler neu geladen; Teildaten belegen bis zur Aufräumfrist Platz; Verhalten echter Server ist nicht belegt.
- Offen (nicht Teil dieses Tasks): Authentifizierung (ADR-0005), RB-01 bis RB-06, RQ-01 bis RQ-08, P1-GUI-Gate.
- Nächster konkreter Schritt: Gegenprüfung von ADR-0009 durch einen zweiten Reviewer. Danach Folgetask für Resume gepackter Archive (Port-Schritt: Teildatei und Staging ohne gemeinsame Ausleihe des Speichers).
