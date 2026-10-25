# Handoff MIG-032R

- Status: **in Arbeit** (reserviert); Claude (Projektleitung, Hauptbearbeiter, Integrator); 2026-10-25; Basis `development` @ `1d7049d`. Bericht: [MIG-032R](../changes/2026-10-25-MIG-032R.md). Vertrag: [ADR-0009](../adr/0009-resume-range.md).
- Umfang: Validatorvertrag (starker ETag), Port-Erweiterung (`identity`, `open`, `partial`), HTTP-`Range`/`If-Range`, Teildateien im Zielordner, Abbruchregeln (Abbruch entfernt, Netzfehler behält), Tests gegen Loopback-Server.
- Nächster Schritt: Core-Ports und `is_strong_validator` implementieren, dann HTTP-Adapter, Dateisystem-Adapter und Anwendungsablauf; danach Tests und Windows-Prüfungen.
- Offen (nicht Teil dieses Tasks): Authentifizierung (ADR-0005), RB-01 bis RB-06, RQ-01 bis RQ-08, P1-GUI-Gate. WSL/Linux und GitHub-CI sind zurückgestellt.
