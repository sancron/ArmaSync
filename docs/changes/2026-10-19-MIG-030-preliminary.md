# 2026-10-19 — MIG-030 Vorprüfung (Vertrag, ADR, Zerlegung)

- Zeitpunkt: 2026-10-19, Europe/Berlin. Eigentümer und Integrator: Claude. Basis `4831fa1b2a8ecc704e4c8742ff46d9d06b9878af` (`origin/development`, GitHub-CI für diesen Commit erfolgreich, per `gh run list`). Branch `claude/mig-030-contract`.
- Zweck: technische und rechtliche Vorprüfung vor der Implementierung von P3.
- Ergebnis: [SYNC_METADATA](../migration/SYNC_METADATA.md) (Inhalt, Felder, Grenzen, Fehlerfälle, Herkunft, nicht Belegtes, Rechte), [ADR-0006](../adr/0006-sync-metadata-and-http-transport.md) (SHA-1 additiv im Core, `ureq` mit rustls und Plattformverifier, Staging/Atomarität, Lizenztabelle nach RQ-06, Zerlegung in MIG-030A–E), Taskdateien [MIG-030](../tasks/MIG-030.md) und [MIG-030A](../tasks/MIG-030A.md).
- Belege: Fixtures `valid/server/*/.a3s/{sync,serverinfo}` und Erwartungsdateien; Verhaltensbeobachtung an der Baseline (nur gelesen). HTTP-Probe außerhalb des Repositories (Windows 11, Rust 1.85.0): Build, `206`-Range-Antwort, Ablehnung von abgelaufenem, unbekanntem und hostnamefalschem Zertifikat (Details im ADR). Keine Code- oder Abhängigkeitsänderung im Repository.
- Rechte: RQ-01..RQ-08 bleiben offen. Unabhängige Implementierung ohne Übernahme von Bestandsquellen; Verhaltensbeschreibung nicht rechtlich entschieden (benannt). RQ-06: Lizenzprüfung der `ureq`-Kette dokumentiert; Release-BOM offen. Kein echter Produktentscheid blockiert: die Wahl folgt aus den Vorgaben (MSRV 1.85, TLS-Prüfung, Windows, Lizenz).
- Nicht geprüft: Linux (zurückgestellt), explizite Proxys, TLS-Inspektion, große Repositories, historische Dateiversionen.
- Nächster Schritt: MIG-030A.
