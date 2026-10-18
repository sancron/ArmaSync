# Letzte integrierte Änderung

**2026-10-18 · MIG-001 · Claude · Formatfixtures, Class-/UID-Inventar und Exportstrategie**

Lokal nach `development` integriert (Merge `de8c868`, nicht gepusht; Codex-Review bis `4f71a36` freigegeben). Neu: `fixtures/legacy-formats/`, `migration-tools/fixture-generator/`, `docs/migration/FORMAT_INVENTORY.md`, `.gitattributes` (Shell-Skripte LF). Kein Rust-Code, kein Laufzeitverhalten geändert. [Bericht](2026-10-18-MIG-001.md), [Handoff](../handoffs/MIG-001.md), [aktueller Stand](../migration/STATUS.md).

Prüfung im zusammengesetzten Stand: `run.ps1` Exit 0, `verify.py` 0 Probleme, Ergebnis byte-identisch zum eingecheckten Stand. **Offen:** vollständiger Linux-Generatorlauf (kein JDK in WSL-Ubuntu), macOS/BSD, Hardlinks.

Vorherige Integration: [MIG-000](2026-10-18-MIG-000.md). Nächste Arbeit: MIG-002 und MIG-003 (Codex, reserviert). Das P0-Gate bleibt bis zu deren Abschluss offen.
