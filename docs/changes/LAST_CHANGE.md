# Letzte integrierte Änderung

**2026-10-19 · MIG-030A · Claude · Sync-Metadaten-Leser, SHA-1 im Core, Manifest-Abbildung** (nach der MIG-030-Vorprüfung)

Nach `development` integriert (Merge `460e4ac`; Basis `729d681`, davor `4831fa1` mit grüner GitHub-CI). Push und CI-Ergebnis dieses Stands stehen in der Antwort an den Projektverantwortlichen; diese Datei behauptet kein CI-Ergebnis für ihn.

* **Neu:** begrenzte Leser für `.a3s/sync` und `.a3s/serverinfo` (`armasync-compat`), `HashAlgorithm::Sha1` (nur Integrität des Altprotokolls), Adapter `sync_meta` (Dateiliste → `Manifest`, unsichere Namen mit Warnung weggelassen, komprimierte Blätter als nicht unterstützt geführt), SHA-1-Verifikation in `apply`. [Bericht](2026-10-19-MIG-030A.md), [Handoff](../handoffs/MIG-030A.md), [Vorprüfung](2026-10-19-MIG-030-preliminary.md), [ADR-0006](../adr/0006-sync-metadata-and-http-transport.md), [SYNC_METADATA](../migration/SYNC_METADATA.md).
* **Es wird weiterhin nichts Reales synchronisiert:** kein Netzwerk-, kein Dateisystemadapter, keine CLI-Synchronisation. HTTP-Client `ureq` ist gewählt und erprobt, aber noch nicht eingebunden (MIG-030B).
* **Neue Abhängigkeit:** `sha1` 0.11.0; MSRV bleibt 1.85.

Prüfung (Windows 11, Rust 1.99.1, am zusammengesetzten Stand): fmt, clippy `-D warnings`, 206 Tests, MSRV 1.85 177 Tests, GUI-freier CLI-Baum, `git diff --check`. **Lokale WSL-/Linux-Prüfung: offen (zurückgestellt gemäß Prüfregel); keine Linux-Aussage aus älteren Läufen.** Ubuntu-CI wird getrennt dokumentiert.

**Offen:** **P1-GUI-Gate: NVDA, Orca und Sichtprüfung des hellen Themes sind nicht ausgeführt – das Gate bleibt offen**; Rechtefragen RQ-01..RQ-08 alle offen; Entschlüsselungsentscheidung (ADR-0003/RQ-01) und `export.a3s.autoconfig`; MIG-030B/C/D/E; Schlüsselbund (ADR-0005); GitHub-Schritte (SHA-Pinning, Rulesets, Required Checks); große/echte Repositories ungeprüft; macOS/BSD.

Vorherige Integrationen: [MIG-023](2026-10-19-MIG-023.md), [MIG-030-Vorprüfung](2026-10-19-MIG-030-preliminary.md), [MIG-022](2026-10-19-MIG-022.md), [PROC-001](2026-10-19-PROC-001.md), [MIG-021](2026-10-18-MIG-021.md), [MIG-020](2026-10-18-MIG-020.md), [MIG-012A](2026-10-18-MIG-012A.md), [MIG-013](2026-10-18-MIG-013.md), [MIG-012](2026-10-18-MIG-012.md), [MIG-011](2026-10-18-MIG-011.md), [MIG-010](2026-10-18-MIG-010.md), [MIG-002](2026-10-18-MIG-002.md), [MIG-003](2026-10-18-MIG-003.md), [MIG-001](2026-10-18-MIG-001.md).
