# Handoff MIG-030A

- Status: reviewbereit; Claude. Datum 2026-10-19. Branch `claude/mig-030a-sync-metadata`, Basis `729d681`.
- Fertig: Leser für `.a3s/sync`/`serverinfo`, `HashAlgorithm::Sha1`, Abbildung auf `Manifest` mit Warnungen, SHA-1-Verifikation in `apply`. Bericht: [MIG-030A](../changes/2026-10-19-MIG-030A.md).
- Windows: fmt, clippy, 206 Tests, MSRV 1.85 (177), GUI-freier Baum. **Offen:** lokale WSL-/Linux-Prüfung (zurückgestellt); Ubuntu-CI getrennt.
- **Nicht** fertig: Netzwerk, Dateisystem, Entpacken, CLI-Sync – es wird noch nichts Reales synchronisiert.
- P1-GUI-Gate offen; RQ-01..RQ-08 offen.
- Nächster Schritt: MIG-030C (Dateisystemadapter: Scan mit SHA-1, Staging, atomarer Austausch, Containment; Windows-Namensregeln) und MIG-030B (HTTP mit `ureq`, Port-Signaturen für streamende Übertragung und Staging als getestete Typen festlegen). Beide dürfen parallel auf disjunkten Dateien laufen; Port-Änderungen im Core zuerst als eigener Vertrags-Commit. In 030D `unsupported`/`protected` beim Löschen respektieren.
