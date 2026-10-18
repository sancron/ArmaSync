# Letzte integrierte Änderung

**2026-10-18 · MIG-002 und MIG-003 · Claude (Übernahme von Codex) · Rechteinventur, Transportmatrix und Loopback-Testserver**

Lokal nach `development` integriert (Merges `681981c` MIG-002, `0fd7418` MIG-003; nicht gepusht). Kein Rust-Code, kein Laufzeitverhalten geändert. Berichte: [MIG-002](2026-10-18-MIG-002.md), [MIG-003](2026-10-18-MIG-003.md); [aktueller Stand](../migration/STATUS.md).

Korrekturen im Integrationsreview: MIG-002 vier falsche SHA-256-Werte (jetzt 87/87 gegen Baseline-Blobs geprüft); MIG-003 Auth-Bypass über kodierte `/auth`-Pfade (Selbsttest 31 Checks). Prüfung am zusammengesetzten Stand: MIG-001 `run.ps1` und `verify.py` 0 Probleme, Windows-Guard-Tests, Gradle-`runtimeClasspath` ohne Fehler, MIG-003-Selbsttest unter Windows und WSL2, Linux-Mount-Guards (WSL, root), `git diff --check`.

**Offen:** Rechtefragen RQ-01..RQ-08 (alle ungeklärt, [Tracker](../tasks/RIGHTS_QUESTIONS.md)); vollständiger Linux-Generatorlauf von `run.sh` (kein JDK in WSL); FTP/FTPS/SFTP/TLS/Proxy-Tests; macOS/BSD. MIG-010 nicht gestartet (keine Rust-Toolchain).

Vorherige Integration: [MIG-001](2026-10-18-MIG-001.md).
