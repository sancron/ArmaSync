# ArmaSync · Lynxnode.it

ArmaSync ist die geplante Rust-Neuentwicklung des vorhandenen Arma-3-Mod- und Repository-Managers unter dem Branding **Lynxnode.it**. Die GUI bleibt das Hauptprodukt; eine eigenständige CLI verwendet denselben Core. Windows und Linux werden unterstützt, Linux zunächst nur für Synchronisation.

**Stand 2026-10-18:** Analyse und Migrationsvorbereitung abgeschlossen. Der ausführbare Bestand ist weiterhin Java; ein Cargo-Workspace existiert noch nicht. Das AGPLv3-Ziel gilt für neue Rust-Komponenten gemäß [LICENSING.md](LICENSING.md); vorhandener Code behält seine Lizenzen.

## Einstieg

- [Projektanalyse](docs/migration/ANALYSIS.md) und [Dateiinventar](docs/migration/INVENTORY.md).
- [Mehrstufiger Upgrade-Plan](docs/migration/ROADMAP.md), [Zielarchitektur](docs/migration/ARCHITECTURE.md), [Funktionsparität](docs/migration/FEATURE_PARITY.md) und [Datenmigration](docs/migration/COMPATIBILITY.md).
- [Branding](docs/migration/BRANDING.md), [Produktanforderungen](PRODUCT.md), [UI-/Accessibility-Brief](docs/migration/UI_REQUIREMENTS.md) und [CLI-Vertrag](docs/migration/CLI_CONTRACT.md).
- [Gemeinsame KI-Anweisungen](AGENTS.md), [Claude-Einstieg](CLAUDE.md) und [parallele Zusammenarbeit](docs/ai/WORKFLOW.md).
- [Aktueller Stand](docs/migration/STATUS.md), [Änderungsjournal](docs/changes/README.md) und [letzte Integration](docs/changes/LAST_CHANGE.md).
- [Fertiger Startprompt für Claude](docs/ai/CLAUDE_START_PROMPT.md).

## Branches

| Branch | Aufgabe |
| --- | --- |
| `master` | Veröffentlichter Code; Übernahme nach erfolgreichem `testing`-Durchlauf. |
| `development` | Integration aktiver Entwicklung und dieser Migrationsunterlagen. |
| `testing` | Release-Kandidaten aus `development`, danach Freigabe nach `master`. |
| `legacy` | Unveränderter Projektstand vor Beginn: `b7753cea18b2ac2cc57e29f926901997a4de8570`. |

`master` und `testing` enthalten zunächst den übernommenen Java-Ausgangsstand; das ist noch keine Rust-Veröffentlichung. Details: [BRANCHING.md](docs/BRANCHING.md).

## Java-Bestand bauen

JDK 25 und globales Gradle werden benötigt; der unabhängige Updater zielt auf Java 21. Es existiert kein Gradle-Wrapper.

```text
gradle test :updater:test --console=plain
gradle run
gradle fatJar :updater:fatJar
```

Die Baseline-Prüfung unter Windows ergab 64 Tests: 63 erfolgreich, 1 übersprungen, 0 fehlgeschlagen. GUI, Netzwerk-End-to-End und Linux wurden dabei nicht geprüft. Details: [VALIDATION.md](docs/migration/VALIDATION.md).

Historische Anleitungen: [BUILD.md](BUILD.md), [TESTING.md](TESTING.md), [UPDATER.md](UPDATER.md), [release/README.md](release/README.md). Sie beschreiben den Bestand, keine Rust-Implementierung. Der vorherige README-Stand bleibt auf `legacy` verfügbar.

## Lizenz und Herkunft

Neue Rust-Komponenten sollen unter **GNU AGPL Version 3 (`AGPL-3.0-only`)** entstehen. [LICENCE.md](LICENCE.md) bleibt die vorhandene GPLv3-Lizenz. [LICENSING.md](LICENSING.md) und [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) dokumentieren Geltungsbereich und Herkunft. Der aktive Markenauftritt lautet ArmaSync / Lynxnode.it; erforderliche rechtliche Herkunftsnachweise bleiben erhalten.
