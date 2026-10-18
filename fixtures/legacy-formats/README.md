# Synthetische Altformat-Fixtures (MIG-001)

Golden-Dateien für den Rust-Parser/-Importer. **Alle Daten sind synthetisch**: Hosts sind `*.example.invalid`, IPs aus `192.0.2.0/24`, Zugangsdaten sind sichtbare Platzhalter (`fixture-user`, `fixture-password-not-a-secret`). Keine Nutzerdaten, keine echten Repositories, keine produktiven Schlüssel.

## Herkunft

- Erzeugt von `migration-tools/fixture-generator/FixtureGenerator.java` mit den **unveränderten Klassen** des Baseline-Commits `b7753cea18b2ac2cc57e29f926901997a4de8570` und dem echten Baseline-Schreiber (`A3SFilesAccessor`, `MetaFileMaker`, `MD4`/`BrokenMD4`). Jede Datei wird direkt danach mit dem echten Baseline-Leser wieder eingelesen.
- Lizenz: Generator und Fixture-Daten `AGPL-3.0-only`. Die Fixtures enthalten Serialisierungsdaten der GPL-lizenzierten Baseline-Klassen (Klassendeskriptoren); Herkunft und Rechtefragen verfolgt MIG-002.
- Die alten Paket-/Klassennamen in den Dateien und im Generator sind **Wire-Daten bzw. Bezug auf die eingefrorene Baseline** (ADR-0003, BRANDING.md) und keine Vorgabe für neue Rust-Typen.

## Aufbau (`generated/`)

| Pfad | Inhalt |
| --- | --- |
| `manifest.json` | Pfad, Art, Größe, `sha256`, bei GZIP `payload_sha256` (entpackter Serialisierungsstrom), Verweis auf Erwartungsdatei, bei ungültigen Dateien Grund und Ausgang des Baseline-Lesers |
| `valid/local/` | `a3s.cfg`, `a3s.prefs`, Profile (inkl. Unicode/langer String), zwei verschlüsselte `.a3s.repository`, `export.a3s.autoconfig` |
| `valid/server/{empty,small,hostile-paths}/.a3s/` | `autoconfig`, `serverinfo`, `sync`, `changelogs`, `events` je Szenario |
| `valid/edge/` | gültige, aber schwierige Ströme (Liste, die sich selbst enthält) |
| `valid/zsync/` | Nutzlasten (1…20 000 Byte, deterministisch) mit `.zsync`; Größe 8191 hat keine `.zsync` (Baseline-Defekt, im Manifest als `zsync-generation-failure`) |
| `valid/md4-vectors.json` | MD4- und BrokenMD4-Vektoren |
| `invalid/` | 15 ungültige Ströme (leer, kein GZIP, abgeschnitten, falsche CRC, falsche Streamversion, unbekannte/nicht erlaubte Klasse, UID-Änderung, Tiefe 150, 1-GiB-Array, GZIP-Bombe) und `zip-traversal.zip` |
| `expected/` | erwartete Feld-Dumps (siehe `docs/migration/FORMAT_INVENTORY.md`, Abschnitt 4) |
| `class-inventory.json` | Reflektion der Baseline-Klassen (UID, ob explizit deklariert) |
| `descriptor-inventory.json` | aus den Strömen abgeleitete Deskriptortabelle (von `verify.py` erzeugt, nicht im Manifest) |

`.gitattributes` in diesem Verzeichnis verhindert Zeilenende-/Textkonvertierung; die Hashes gelten für die Bytes.

## Erneut erzeugen und prüfen

Voraussetzung: JDK 17+, Gradle, Python 3, Git; Arbeitsbaum mit unverändertem `src/`, `updater/`, `build.gradle`.

```text
pwsh migration-tools/fixture-generator/run.ps1        # Windows
sh   migration-tools/fixture-generator/run.sh          # Linux
```

Die Skripte kompilieren die Baseline-Klassen, erzeugen den Ordner `generated/` neu und führen `verify.py` aus (JVM-freie Dekodierung, Hash-/Erwartungsabgleich, zsync-/MD4-Nachrechnung, Ablehnung der ungültigen Fälle). Nur prüfen, ohne neu zu erzeugen: `python -I migration-tools/fixture-generator/verify.py fixtures/legacy-formats/generated`; Hashprüfung allein mit `FixtureGenerator verify <dir>`.

Die Skripte löschen nur ein vorhandenes `generated/` mit Marker-Manifest und lehnen Pfade mit Links/Junctions in jeder Komponente ab; Test: `pwsh migration-tools/fixture-generator/test-output-guard.ps1`.

Reproduzierbarkeit: feste Zeit (2026-01-01T00:00:00Z), UTC, LCG-Nutzlasten, deterministische Serialisierung. Zwei aufeinanderfolgende Läufe auf JDK 25.0.2 (Windows) ergaben byte-identische Verzeichnisse. Andere JDK-/zlib-Versionen können andere GZIP-Bytes liefern; dann bleibt `payload_sha256` gleich.
