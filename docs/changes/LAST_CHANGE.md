# Letzte integrierte Änderung

**2026-10-19 · MIG-022 · Claude · Begrenzter Import von Server-Metadaten (`.a3s/autoconfig`)**

Nach `development` integriert (Merge `0f2ee31`, Basis `7f3bab1` = `origin/development` bei Start). Der Push nach `origin/development` (Fast-Forward) und das CI-Ergebnis stehen in der Antwort an den Projektverantwortlichen beziehungsweise im nächsten Statuseintrag; diese Datei behauptet kein CI-Ergebnis.

* **Neu:** Crate `armasync-compat` (begrenzter Parser, keine JVM, kein Klassenladen, keine I/O), Anbindung an den Importer (`Importer::{preview,apply}_autoconfig`), `armasync repository import <autoconfig-datei>` (nur lokale Datei, **kein Netzwerkabruf**), verständliche Umfangs-Hinweise (`scope_notices`, deutsch/englisch) für `armasync import` (MIG-021) und `repository import` (MIG-022) als gemeinsame Quelle für CLI und spätere GUI. [Bericht](2026-10-19-MIG-022.md), [Handoff](../handoffs/MIG-022.md), [SERVER_METADATA_IMPORT](../migration/SERVER_METADATA_IMPORT.md).
* **Klare Trennung:** Name und Adresse stammen aus öffentlichen Server-Metadaten; Download-Ordner, Ausschlüsse, Upload-Einstellungen und Kennwörter bleiben in den verschlüsselten `.a3s.repository`-Dateien. **MIG-022 löst die fehlende Entschlüsselung (ADR-0003/RQ-01), Einstellungen und Addon-Listen nicht.** Kennwörter werden nie gelesen, gespeichert, geloggt oder ausgegeben.
* **Neue Abhängigkeit:** `flate2` (nur Rust-Backend); MSRV bleibt 1.85.

Prüfung am zusammengesetzten Stand `development` (Windows 11, Rust 1.99.1): fmt, clippy `-D warnings`, 177 Tests, GUI-freier CLI-Baum, Branch-Policy-Tests. Linux (WSL2) und MSRV 1.85 wurden am Branch-Stand `ba5748c` ausgeführt (177 bzw. 148 Tests; Code seither unverändert, nur Dokumentation); Details im [Bericht](2026-10-19-MIG-022.md). CI-Ergebnis auf GitHub: noch offen.

**Offen:** **P1-GUI-Gate: NVDA, Orca und Sichtprüfung des hellen Themes sind nicht ausgeführt – das Gate bleibt offen** (Protokoll [GUI_SCREENREADER_PROTOCOL](../migration/GUI_SCREENREADER_PROTOCOL.md)); Entschlüsselungsentscheidung (ADR-0003/RQ-01) und ob `export.a3s.autoconfig` zugelassen wird; Schlüsselbund (ADR-0005, erst bei konkretem Bedarf); GitHub-Schritte (SHA-Pinning, Rulesets, Required Checks); Rechtefragen RQ-01..RQ-08; echte Adapter und Netzwerkabruf (P3/P4); macOS/BSD, Hardlinks, FTP/FTPS/SFTP/TLS-Tests; reale Server und Nutzerdateien für MIG-021/022 ungeprüft.

Vorherige Integrationen: [MIG-021](2026-10-18-MIG-021.md), [MIG-020](2026-10-18-MIG-020.md), [MIG-012A](2026-10-18-MIG-012A.md), [MIG-013](2026-10-18-MIG-013.md), [MIG-012](2026-10-18-MIG-012.md), [MIG-011](2026-10-18-MIG-011.md), [MIG-010](2026-10-18-MIG-010.md), [MIG-002](2026-10-18-MIG-002.md), [MIG-003](2026-10-18-MIG-003.md), [MIG-001](2026-10-18-MIG-001.md).
