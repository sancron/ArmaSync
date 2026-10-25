# ADR-0013 — Upload-Ausschlüsse: feste Regeln, Vorschau und die offene Formatfrage (MIG-042, Schritt 3b)

Datum: 2026-10-25. Status: **angenommen**. Entscheidung des Projektverantwortlichen (2026-10-25): **Option A**, explizite Datei- und Ordnerpfade. Angenommen sind die fest eingebauten Regeln, die Ausschlüsse als validierte Pfade, die komponentenweise Ordnerabgrenzung und die Vorschau im Plan. **Nicht entschieden und nicht Teil dieses ADR:** Muster (Option B) und gitignore-artige Regeldateien (Option C); beide bleiben spätere, separat zu entscheidende Erweiterungen. Bezug: [ADR-0011](0011-upload-contract.md) (Upload), [ADR-0012](0012-revisions-and-publication.md) (Revisionen), [ADR-0004](0004-core-contracts.md) (Kernverträge, keine I/O im Core).

## Kontext

Der Upload darf keine internen ArmaSync-Daten, keine Teildateien und keine ausdrücklich ausgeschlossenen Pfade hochladen. Die Vorschau muss vollständig zeigen, was einbezogen und was ausgeschlossen wird und warum. Der Scan der lokalen Quelle hatte außerdem eine Nebenwirkung: Er entfernte beim Auflisten Staging-Reste älter als eine Stunde und verwaiste Teildateien, und er blendete interne Namen aus. Für eine Upload-Quelle ist beides falsch: Die Quelle darf nicht verändert werden, und die Vorschau muss die internen Dateien zeigen.

## Entscheidungen

1. **Feste Regeln (nicht abschaltbar).**
   - **Teildateien:** Dateiname beginnt mit `.armasync-partial-`. Grund `partial-file`.
   - **Interne Daten:** eine Pfadkomponente beginnt mit `.armasync-` (Staging-Dateien, Sperrdatei, alles, was die Adapter anlegen). Grund `internal-data`. Die Präfixe sind im Core definiert (`armasync_core::exclusion::INTERNAL_PREFIX`, `PARTIAL_PREFIX`); der Adapter verwendet dieselben Konstanten.
   - Feste Regeln können durch keine Benutzerregel aufgehoben werden.
2. **Ausdrückliche Ausschlüsse als validierte Pfade.** Eine Regel ist ein `RelativePath`: sie bezeichnet eine Datei oder einen Ordner mit allem Inhalt. Muster gibt es nicht. Pfade mit `..`, absolute Pfade und Rückstriche werden bereits beim Erzeugen abgelehnt; eine unsichere Regel lässt sich nicht ausdrücken. Doppelte Regeln werden zusammengefasst.
3. **Ein Grund je ausgeschlossener Datei.** Gründe: `partial-file`, `internal-data`, `user-rule` (mit der Regel, die gegriffen hat). Eine Teildatei zählt als Teildatei, auch wenn sie in einem internen Ordner liegt.
4. **Ungenutzte Regeln** werden in der Vorschau gemeldet, nicht stillschweigend ignoriert.
5. **Vorschau im Plan.** `UploadPlan` enthält `exclusions` (Pfad, Größe, Grund, sortiert nach Pfad) und `unused_rules`. Ordner werden nur für einbezogene Dateien angelegt; ein Ordner, der nur ausgeschlossene Dateien enthält, erscheint nicht.
6. **Quelle wird nicht verändert.** Die Auflistung der Quelle ist rein lesend: kein Entfernen von Staging-Resten, keine Sperre, keine Verfolgung von Links. Die Quelle liefert zwei Sichten: `files()` (einbeziehbare Dateien mit Hash) und `inventory()` (alle regulären Dateien mit Größe, auch interne). Daraus bildet die Anwendung die Partition.
7. **Revision und Plan teilen dieselbe Partition.** Eine Veröffentlichung enthält nur einbezogene Dateien; Manifest und Vorschau stimmen überein.
8. **Keine Geheimnisse in Vorschau oder Manifest.** Die Vorschau zeigt Pfade, Größen und Gründe. Das Revisions-Manifest enthält nur einbezogene Dateien. Zugangsdaten gelangen weder in die Vorschau noch in Logs oder Upload-Metadaten.

## Benutzerformat: Entscheidung (Option A angenommen)

Der Projektverantwortliche hat Option A gewählt. Die Optionen im Überblick, mit Stand der Entscheidung:

- **A. Explizite Pfade und Ordner (in Schritt 3b umgesetzt).** Keine Muster, keine Mehrdeutigkeit, leicht zu prüfen. Nachteil: bei vielen Dateien umständlich.
- **B. Muster (z. B. `*.log`, `**/cache`).** Mächtig; braucht eine dokumentierte Semantik (Groß-/Kleinschreibung, Trennzeichen, Vorrang) und eine eigene Testmatrix. Muster dürfen keine internen oder Teildateien wieder einbeziehen.
- **C. Regeldatei im Quellordner (gitignore-artig).** Nutzerfreundlich, aber die Datei selbst wird zur Nutzerdatei; braucht Speicherort, Vorrangregeln und den Umgang mit ihrer eigenen Hochladbarkeit.

Stand: **A angenommen und umgesetzt.** **B und C** sind nicht entschieden und nicht implementiert; sie brauchen eine eigene Entscheidung und einen eigenen Vertrag.

## Nicht im Scope

Parallelität, Wiederaufnahme, Legacy-Schreibformate, CLI- und GUI-Verdrahtung, das Benutzerformat (siehe oben), Ereignisse und Changelogs.

## Belege

- Core: `crates/armasync-core/src/exclusion.rs` (Regeln, Gründe, Entscheidung, ungenutzte Regeln; Modultests).
- Quelle: `crates/armasync-adapters/src/localfs/mod.rs` (`files`, `inventory`, rein lesendes `walk`).
- Anwendung: `crates/armasync-application/src/exclusion.rs` (Partition), `upload.rs` (`plan_with_rules`, `plan_included`), `revision.rs` (`publish_with_rules`).
- Tests: `crates/armasync-application/tests/exclusion_cases.rs` (9), `crates/armasync-adapters/tests/exclusion_flow.rs` (4, echter Ordner und Loopback-Server).
